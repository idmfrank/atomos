//! Offline reservation ledger prototype. No live provider or billing capability.
//! One exclusive writer. Unsettled reservations remain charged after restart.
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_INSTANCE: AtomicU64 = AtomicU64::new(1);

const MAX_LEDGER_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Priority {
    Interactive,
    Background,
}

#[derive(Clone, Copy)]
pub struct Limits {
    pub total: u64,
    pub background: u64,
    pub interactive_reserve: u64,
    pub per_request: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            total: 750_000,
            background: 75_000,
            interactive_reserve: 225_000,
            per_request: 1500,
        }
    }
}

/// Shared checked accounting decision for the reference and isolated candidate.
pub(crate) fn reservation_allowed(limits: Limits, total: u64, background: u64,
    priority: Priority, amount: u64) -> bool {
    if amount == 0 || amount > limits.per_request { return false; }
    let Some(projected) = total.checked_add(amount) else { return false; };
    if projected > limits.total { return false; }
    priority != Priority::Background || (
        background.checked_add(amount).map_or(false, |v| v <= limits.background)
        && limits.total.checked_sub(limits.interactive_reserve).map_or(false, |v| projected <= v))
}

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Invalid,
    Denied,
    Unknown,
    Corrupt,
    Unavailable,
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reservation {
    instance: u64,
    id: u64,
}

struct Entry {
    priority: Priority,
    amount: u64,
    settled: bool,
}

// A stale lock after an abnormal exit deliberately blocks further admissions.
struct Lock(PathBuf);

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

pub struct Governor {
    instance: u64,
    file: File,
    _lock: Lock,
    limits: Limits,
    entries: BTreeMap<u64, Entry>,
    next_id: u64,
    total: u64,
    background: u64,
    healthy: bool,
    halted: bool,
}

impl Governor {
    pub fn open(dir: &Path, limits: Limits) -> Result<Self, Error> {
        if limits.total == 0
            || limits.per_request == 0
            || limits.interactive_reserve > limits.total
            || limits.background > limits.total
        {
            return Err(Error::Invalid);
        }
        fs::create_dir_all(dir)?;
        let lock_path = dir.join("writer.lock");
        OpenOptions::new().write(true).create_new(true).open(&lock_path)?;
        let lock = Lock(lock_path);
        let path = dir.join("usage.log");
        let file = OpenOptions::new().create(true).append(true).read(true).open(&path)?;
        // Flush creation of the journal before accepting reservations on Linux.
        file.sync_all()?;
        #[cfg(unix)]
        File::open(dir)?.sync_all()?;
        if file.metadata()?.len() > MAX_LEDGER_BYTES {
            return Err(Error::Unavailable);
        }
        let log = fs::read_to_string(&path)?;
        if !log.is_empty() && !log.ends_with('\n') {
            return Err(Error::Corrupt);
        }
        // Use a CAS loop compatible with Rust 1.75 and newer toolchains.
        let mut instance = NEXT_INSTANCE.load(Ordering::Relaxed);
        loop {
            let next = instance.checked_add(1).ok_or(Error::Unavailable)?;
            match NEXT_INSTANCE.compare_exchange_weak(
                instance, next, Ordering::Relaxed, Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(observed) => instance = observed,
            }
        }
        let mut governor = Self {
            instance,
            file,
            _lock: lock,
            limits,
            entries: BTreeMap::new(),
            next_id: 1,
            total: 0,
            background: 0,
            healthy: true,
            halted: false,
        };
        for line in log.lines() {
            governor.replay(line)?;
        }
        Ok(governor)
    }

    pub fn charged(&self) -> u64 {
        self.total
    }

    pub fn background_charged(&self) -> u64 {
        self.background
    }

    fn replay(&mut self, line: &str) -> Result<(), Error> {
        let fields: Vec<_> = line.split_whitespace().collect();
        let number = |value: &str| value.parse::<u64>().map_err(|_| Error::Corrupt);
        match fields.as_slice() {
            ["R", id, p, amount] => {
                let id = number(id)?;
                let amount = number(amount)?;
                let priority = match *p {
                    "I" => Priority::Interactive,
                    "B" => Priority::Background,
                    _ => return Err(Error::Corrupt),
                };
                if id != self.next_id || amount == 0 {
                    return Err(Error::Corrupt);
                }
                self.total = self.total.checked_add(amount).ok_or(Error::Corrupt)?;
                if priority == Priority::Background {
                    self.background = self.background.checked_add(amount).ok_or(Error::Corrupt)?;
                }
                self.entries.insert(id, Entry { priority, amount, settled: false });
                self.next_id = id.checked_add(1).ok_or(Error::Corrupt)?;
            }
            [kind @ ("S" | "X"), id, actual] => {
                let id = number(id)?;
                let actual = number(actual)?;
                let entry = self.entries.get_mut(&id).ok_or(Error::Corrupt)?;
                if entry.settled {
                    return Err(Error::Corrupt);
                }
                if *kind == "X" || actual > entry.amount { self.halted = true; }
                self.total = self.total.checked_sub(entry.amount)
                    .and_then(|v| v.checked_add(actual)).ok_or(Error::Corrupt)?;
                if entry.priority == Priority::Background {
                    self.background = self.background.checked_sub(entry.amount)
                        .and_then(|v| v.checked_add(actual)).ok_or(Error::Corrupt)?;
                }
                entry.amount = actual;
                entry.settled = true;
            }
            ["H"] => { self.halted = true; }
            _ => return Err(Error::Corrupt),
        }
        Ok(())
    }

    fn persist(&mut self, line: &str) -> Result<(), Error> {
        if !self.healthy {
            return Err(Error::Unavailable);
        }
        let result = (|| -> Result<(), Error> {
            let size = self.file.metadata()?.len();
            if size.saturating_add(line.len() as u64 + 1) > MAX_LEDGER_BYTES {
                return Err(Error::Unavailable);
            }
            writeln!(self.file, "{line}")?;
            self.file.sync_all()?;
            self.replay(line)
        })();
        if result.is_err() {
            self.healthy = false;
        }
        result
    }

    pub fn reserve(&mut self, priority: Priority, amount: u64) -> Result<Reservation, Error> {
        if !self.healthy {
            return Err(Error::Unavailable);
        }
        if self.halted || !reservation_allowed(self.limits, self.total, self.background, priority, amount) {
            return Err(Error::Denied);
        }
        if self.next_id == u64::MAX {
            return Err(Error::Unavailable);
        }
        let id = self.next_id;
        let p = if priority == Priority::Interactive { "I" } else { "B" };
        self.persist(&format!("R {id} {p} {amount}"))?;
        Ok(Reservation { instance: self.instance, id })
    }

    /// Fail closed after an observed cap violation. Replay preserves the halt.
    pub(crate) fn halt(&mut self) -> Result<(), Error> {
        if self.halted { return Ok(()); }
        self.persist("H")
    }

    /// Only authoritative, final usage may settle a reservation.
    /// Unknown/cancelled usage remains charged at its reserved amount.
    /// An overrun is recorded, even when it exceeds the policy limit.
    pub fn settle(&mut self, reservation: Reservation, actual: u64) -> Result<(), Error> {
        self.settle_with_halt(reservation, actual, false)
    }

    /// Atomic settlement and persistent halt for a component-cap violation.
    pub(crate) fn settle_with_halt(&mut self, reservation: Reservation, actual: u64, halt: bool) -> Result<(), Error> {
        if !self.healthy {
            return Err(Error::Unavailable);
        }
        if reservation.instance != self.instance {
            return Err(Error::Unknown);
        }
        let entry = self.entries.get(&reservation.id).ok_or(Error::Unknown)?;
        if entry.settled {
            return Err(Error::Unknown);
        }
        // Preflight arithmetic before writing a record replay cannot apply.
        if self.total.checked_sub(entry.amount).and_then(|v|v.checked_add(actual)).is_none()
            || (entry.priority==Priority::Background &&
                self.background.checked_sub(entry.amount).and_then(|v|v.checked_add(actual)).is_none()) {
            self.halt()?;
            return Err(Error::Unavailable);
        }
        let kind = if halt { "X" } else { "S" };
        self.persist(&format!("{kind} {} {actual}", reservation.id))
    }
}
