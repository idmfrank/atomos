//! Isolated M2.5a synthetic candidate. No transport, credentials or public permit.
//! Trusted private Linux directory only; no same-UID/rollback protection.
use crate::{
    authorization::{Envelope, Policy},
    broker::{Answer, Failure, Request, Usage},
    governor::{reservation_allowed, Priority},
    resolve_local,
    responses::{self, ExtendedTerminal, Terminal},
};
use sha2::{Digest, Sha256};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub const NAMESPACE: &str = "synthetic-atomic-admission-v1";
const MAX_LOG: u64 = 1024 * 1024;
const MAX_RECORD: usize = 512;

#[derive(Clone, Copy)]
pub struct Configuration {
    pub policy: Policy,
    pub version: u64,
    pub model: u64,
    pub model_eligible: bool,
}

/// Current trusted caller state, injected anew at admission, never a permit.
/// Fixture time only; a trusted wall-clock source is a future integration gate.
#[derive(Clone, Copy)]
pub struct Context {
    pub now: u64,
    pub active_turn: u64,
    pub task: u64,
}
impl Configuration {
    fn valid(self) -> bool {
        self.policy.valid() && self.version != 0 && self.model != 0
    }
    fn header(self) -> String {
        format!(
            "AA1 {} {} {} {}",
            self.version,
            self.model,
            u8::from(self.model_eligible),
            self.policy.header()
        )
    }
}
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Scope,
    Evidence,
    Expired,
    ClockRollback,
    Replay,
    StaleTurn,
    StaleTask,
    Budget,
    Suspended,
    Storage,
    Corrupt,
    Stream,
    UnknownUsage,
    Cancelled,
    Provider(Failure),
    Overrun,
}
/// Final offline replay outcome. Refusal/failure never carry provider text.
#[derive(Debug, PartialEq, Eq)]
pub enum ReplayOutcome {
    Local(Answer),
    Completed(Answer),
    Refused {
        turn: u64,
        usage: Usage,
    },
    Failed {
        turn: u64,
        failure: Failure,
        usage: Option<Usage>,
    },
}
/// Read-only accounting view; never an authorization or dispatch permit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplayStatus {
    pub healthy: bool,
    pub halted: bool,
    pub pending_units: Option<u64>,
    pub confirmed_total: u64,
    pub confirmed_background: u64,
    pub next_request_id: u64,
}
fn io<T>(r: std::io::Result<T>) -> Result<T, Error> {
    r.map_err(|_| Error::Storage)
}
fn private(path: &Path, directory: bool) -> Result<(), Error> {
    let m = io(fs::symlink_metadata(path))?;
    if m.mode() & 0o077 != 0
        || (if directory {
            !m.is_dir()
        } else {
            !m.is_file() || m.nlink() != 1
        })
    {
        return Err(Error::Storage);
    }
    Ok(())
}
struct Lock(PathBuf);
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
fn lock(dir: &Path) -> Result<Lock, Error> {
    let p = dir.join("writer.lock");
    io(OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&p))?;
    Ok(Lock(p))
}

/// Owned immutable compiled fixture. No field setters, borrowed body or permit.
/// Recompilation creates a different value; admission records its full SHA-256 identity.
pub struct CompiledRequest {
    envelope: Envelope,
    model: u64,
    version: u64,
    turn: u64,
    task: u64,
    text: String,
    priority: Priority,
    max_output: u64,
    digest: String,
}
impl CompiledRequest {
    pub fn compile(e: Envelope, model: u64, version: u64, r: Request<'_>) -> Result<Self, Error> {
        if r.turn == 0
            || r.task == 0
            || r.text.trim().is_empty()
            || r.text.len() > 1024
            || r.text.chars().any(char::is_control)
            || r.max_output == 0
            || r.max_output > 256
        {
            return Err(Error::Invalid);
        }
        let mut hash = Sha256::new();
        hash.update(b"atomos-synthetic-compiled-v1\0");
        for n in [
            e.account,
            e.host,
            e.grant,
            e.request_id,
            e.now,
            e.expires_at,
            model,
            version,
            r.turn,
            r.task,
            r.max_output,
            u64::from(r.priority == Priority::Background),
            r.text.len() as u64,
        ] {
            hash.update(n.to_le_bytes());
        }
        hash.update(r.text.as_bytes());
        Ok(Self {
            envelope: e,
            model,
            version,
            turn: r.turn,
            task: r.task,
            text: r.text.into(),
            priority: r.priority,
            max_output: r.max_output,
            digest: format!("{:x}", hash.finalize()),
        })
    }
    pub fn identity(&self) -> &str {
        &self.digest
    }
}

#[derive(Clone)]
struct Pending {
    id: u64,
    amount: u64,
    priority: Priority,
}
#[derive(Clone)]
struct State {
    next: u64,
    last: u64,
    total: u64,
    background: u64,
    pending: Option<Pending>,
    halted: bool,
}
impl State {
    fn new(c: Configuration) -> Self {
        Self {
            next: 1,
            last: c.policy.window_start,
            total: 0,
            background: 0,
            pending: None,
            halted: false,
        }
    }
    fn replay(&mut self, c: Configuration, line: &str) -> Result<(), Error> {
        let f: Vec<_> = line.split(' ').collect();
        let num = |s: &str| s.parse::<u64>().map_err(|_| Error::Corrupt);
        match f.as_slice() {
            ["A", id, now, expiry, turn, task, bytes, output, priority, digest, amount] => {
                let (id, now, expiry, turn, task, bytes, output, amount) = (
                    num(id)?,
                    num(now)?,
                    num(expiry)?,
                    num(turn)?,
                    num(task)?,
                    num(bytes)?,
                    num(output)?,
                    num(amount)?,
                );
                let p = match *priority {
                    "I" => Priority::Interactive,
                    "B" => Priority::Background,
                    _ => return Err(Error::Corrupt),
                };
                if self.halted || self.pending.is_some() || id != self.next || id == u64::MAX || now < self.last
                    || turn == 0 || task == 0 || !(1..=1024).contains(&bytes) || !(1..=256).contains(&output)
                    || !valid_time(c.policy,now,expiry) || !c.policy.included_fixture() || !c.model_eligible
                    || digest.len()!=64 || !digest.bytes().all(|b|b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                    || bytes.checked_add(128).and_then(|v|v.checked_add(output)) != Some(amount)
                    || !reservation_allowed(c.policy.limits,self.total,self.background,p,amount)
                    || line != format!("A {id} {now} {expiry} {turn} {task} {bytes} {output} {priority} {digest} {amount}") { return Err(Error::Corrupt); }
                self.total = self.total.checked_add(amount).ok_or(Error::Corrupt)?;
                if p == Priority::Background {
                    self.background = self.background.checked_add(amount).ok_or(Error::Corrupt)?;
                }
                self.next = id.checked_add(1).ok_or(Error::Corrupt)?;
                self.last = now;
                self.pending = Some(Pending {
                    id,
                    amount,
                    priority: p,
                });
            }
            ["S", id, actual, halt] => {
                let (id, actual) = (num(id)?, num(actual)?);
                let p = self.pending.as_ref().ok_or(Error::Corrupt)?;
                if self.halted
                    || actual > 1_000_000_000
                    || p.id != id
                    || !matches!(*halt, "0" | "1")
                    || (actual > p.amount && *halt != "1")
                    || line != format!("S {id} {actual} {halt}")
                {
                    return Err(Error::Corrupt);
                }
                self.total = self
                    .total
                    .checked_sub(p.amount)
                    .and_then(|v| v.checked_add(actual))
                    .ok_or(Error::Corrupt)?;
                if p.priority == Priority::Background {
                    self.background = self
                        .background
                        .checked_sub(p.amount)
                        .and_then(|v| v.checked_add(actual))
                        .ok_or(Error::Corrupt)?;
                }
                self.halted |= *halt == "1";
                self.pending = None;
            }
            ["H"] if !self.halted => self.halted = true,
            _ => return Err(Error::Corrupt),
        }
        Ok(())
    }
}
fn valid_time(p: Policy, now: u64, expiry: u64) -> bool {
    now >= p.window_start
        && now < p.window_end
        && now < p.grant_expiry
        && expiry > now
        && expiry - now <= 60
        && expiry <= p.window_end
        && expiry <= p.grant_expiry
}

pub struct AtomicReplay {
    config: Configuration,
    state: State,
    file: File,
    _lock: Lock,
    healthy: bool,
    dispatches: u64,
    writes: u64,
    syncs: u64,
    #[cfg(test)]
    fault: Option<(u64, bool)>,
}
impl AtomicReplay {
    /// On unhealthy instances totals are last-confirmed, not an authoritative
    /// statement about ambiguous surviving journal writes.
    pub fn status(&self) -> ReplayStatus {
        ReplayStatus {
            healthy: self.healthy,
            halted: self.state.halted,
            pending_units: self.state.pending.as_ref().map(|p| p.amount),
            confirmed_total: self.state.total,
            confirmed_background: self.state.background,
            next_request_id: self.state.next,
        }
    }
    /// Explicit create-once; any existing namespace is an error, including partial creation.
    pub fn initialize(root: &Path, c: Configuration) -> Result<Self, Error> {
        if !c.valid() {
            return Err(Error::Invalid);
        }
        private(root, true)?;
        let dir = root.join(NAMESPACE);
        io(fs::DirBuilder::new().mode(0o700).create(&dir))?;
        let guard = lock(&dir)?;
        let mut file = io(OpenOptions::new()
            .append(true)
            .read(true)
            .create_new(true)
            .mode(0o600)
            .open(dir.join("admission.log")))?;
        io(writeln!(file, "{}", c.header()))?;
        io(file.sync_all())?;
        io(File::open(&dir).and_then(|f| f.sync_all()))?;
        io(File::open(root).and_then(|f| f.sync_all()))?;
        Ok(Self {
            config: c,
            state: State::new(c),
            file,
            _lock: guard,
            healthy: true,
            dispatches: 0,
            writes: 0,
            syncs: 0,
            #[cfg(test)]
            fault: None,
        })
    }
    pub fn open(root: &Path, c: Configuration) -> Result<Self, Error> {
        if !c.valid() {
            return Err(Error::Invalid);
        }
        private(root, true)?;
        let dir = root.join(NAMESPACE);
        private(&dir, true)?;
        let guard = lock(&dir)?;
        let path = dir.join("admission.log");
        private(&path, false)?;
        let mut file = io(OpenOptions::new().append(true).read(true).open(&path))?;
        let mut text = String::new();
        io((&mut file).take(MAX_LOG + 1).read_to_string(&mut text))?;
        if text.len() as u64 > MAX_LOG || !text.ends_with('\n') {
            return Err(Error::Corrupt);
        }
        let mut lines = text.split_terminator('\n');
        if lines.next() != Some(c.header().as_str()) {
            return Err(Error::Scope);
        }
        let mut state = State::new(c);
        for line in lines {
            if line.len() > MAX_RECORD {
                return Err(Error::Corrupt);
            }
            state.replay(c, line)?;
        }
        Ok(Self {
            config: c,
            state,
            file,
            _lock: guard,
            healthy: true,
            dispatches: 0,
            writes: 0,
            syncs: 0,
            #[cfg(test)]
            fault: None,
        })
    }
    pub fn charged(&self) -> u64 {
        self.state.total
    }
    pub fn background_charged(&self) -> u64 {
        self.state.background
    }
    pub fn next_request_id(&self) -> u64 {
        self.state.next
    }
    pub fn dispatches(&self) -> u64 {
        self.dispatches
    }
    pub fn suspended(&self) -> bool {
        !self.healthy || self.state.halted || self.state.pending.is_some()
    }
    /// Actual append/sync calls since open, excluding initialization and directory fsync.
    pub fn persistence_counts(&self) -> (u64, u64) {
        (self.writes, self.syncs)
    }
    fn persist(&mut self, line: &str) -> Result<(), Error> {
        if !self.healthy {
            return Err(Error::Storage);
        }
        let result = (|| {
            let mut next = self.state.clone();
            next.replay(self.config, line)?;
            if line.len() > MAX_RECORD
                || io(self.file.metadata())?
                    .len()
                    .checked_add(line.len() as u64 + 1)
                    .map_or(true, |v| v > MAX_LOG)
            {
                return Err(Error::Storage);
            }
            self.writes = self.writes.checked_add(1).ok_or(Error::Storage)?;
            #[cfg(test)]
            if self.fault == Some((self.writes, true)) {
                // Actual OS write failure, private to unit-test builds.
                io(OpenOptions::new().write(true).open("/dev/full")
                    .and_then(|mut f| f.write_all(line.as_bytes())))?;
            }
            io(writeln!(self.file, "{line}"))?;
            self.syncs = self.syncs.checked_add(1).ok_or(Error::Storage)?;
            #[cfg(test)]
            if self.fault == Some((self.writes, false)) {
                // Preserve complete journal bytes while simulating ambiguous sync.
                io(File::open("/dev/null").and_then(|f| f.sync_all()))?;
            }
            io(self.file.sync_all())?;
            self.state = next;
            Ok(())
        })();
        if result.is_err() {
            self.healthy = false;
        }
        result
    }
    pub fn revoke(&mut self) -> Result<(), Error> {
        if !self.healthy {
            return Err(Error::Storage);
        }
        if self.state.halted {
            return Err(Error::Suspended);
        }
        self.persist("H")
    }
    /// Pure validation only; returns no authorization object and permits no dispatch.
    pub fn validate(&self, r: &CompiledRequest, context: Context) -> Result<(), Error> {
        if r.turn != context.active_turn {
            return Err(Error::StaleTurn);
        }
        if r.task != context.task {
            return Err(Error::StaleTask);
        }
        if resolve_local(&r.text).is_some() {
            return Ok(());
        }
        if self.suspended() {
            return Err(Error::Suspended);
        }
        let c = self.config;
        let e = r.envelope;
        let p = c.policy;
        if (e.account, e.host, e.grant, r.model, r.version)
            != (p.account, p.host, p.grant, c.model, c.version)
        {
            return Err(Error::Scope);
        }
        if !p.included_fixture() || !c.model_eligible {
            return Err(Error::Evidence);
        }
        if context.now < self.state.last || context.now < e.now {
            return Err(Error::ClockRollback);
        }
        if e.request_id != self.state.next || e.request_id == u64::MAX {
            return Err(Error::Replay);
        }
        if !valid_time(p, e.now, e.expires_at)
            || context.now >= e.expires_at
            || context.now >= p.grant_expiry
            || context.now >= p.window_end
        {
            return Err(Error::Expired);
        }
        let amount = (r.text.len() as u64)
            .checked_add(128)
            .and_then(|v| v.checked_add(r.max_output))
            .ok_or(Error::Budget)?;
        if !reservation_allowed(
            p.limits,
            self.state.total,
            self.state.background,
            r.priority,
            amount,
        ) {
            return Err(Error::Budget);
        }
        Ok(())
    }
    /// Explicit extended offline profile using the same immutable request.
    /// Unknown/interrupted outcomes retain reservation and block admission.
    /// No transport, retry or dispatch permit.
    pub fn run_sse_extended(
        &mut self,
        r: &CompiledRequest,
        context: Context,
        input: impl Read,
        cancelled: impl FnMut() -> bool,
    ) -> Result<ReplayOutcome, Error> {
        self.validate(r, context)?;
        if let Some(text) = resolve_local(&r.text) {
            return Ok(ReplayOutcome::Local(Answer {
                turn: r.turn,
                text: text.into(),
                simulated: false,
                usage: None,
            }));
        }
        let e = r.envelope;
        let input_bound = (r.text.len() as u64)
            .checked_add(128)
            .ok_or(Error::Budget)?;
        let amount = input_bound.checked_add(r.max_output).ok_or(Error::Budget)?;
        let priority = if r.priority == Priority::Interactive {
            "I"
        } else {
            "B"
        };
        self.persist(&format!(
            "A {} {} {} {} {} {} {} {} {} {}",
            e.request_id,
            context.now,
            e.expires_at,
            r.turn,
            r.task,
            r.text.len(),
            r.max_output,
            priority,
            r.digest,
            amount
        ))?;
        self.dispatches = self.dispatches.checked_add(1).ok_or(Error::Suspended)?;
        let d = responses::decode_extended(input, cancelled)
            .0
            .map_err(|e| match e {
                responses::DecodeError::Cancelled => Error::Cancelled,
                responses::DecodeError::MissingTerminal
                | responses::DecodeError::Io
                | responses::DecodeError::Usage => Error::UnknownUsage,
                responses::DecodeError::Unsupported => Error::Provider(Failure::Unsupported),
                _ => Error::Stream,
            })?;
        let (usage, failure, refused) = match d.terminal {
            ExtendedTerminal::Completed(u) => (Some(u), None, false),
            ExtendedTerminal::Refused(u) => (Some(u), None, true),
            ExtendedTerminal::Failed(k, u) => (u, Some(k), false),
        };
        let halt = matches!(failure, Some(Failure::RateLimited | Failure::Authorization));
        if let Some(u) = usage {
            let actual = usage_total(u).ok_or(Error::UnknownUsage)?;
            if actual > 1_000_000_000 {
                return Err(Error::UnknownUsage);
            }
            let overrun = actual > amount || u.input > input_bound || u.output > r.max_output;
            self.persist(&format!(
                "S {} {} {}",
                e.request_id,
                actual,
                u8::from(halt || overrun)
            ))?;
            if overrun {
                return Err(Error::Overrun);
            }
        } else if halt {
            self.revoke()?;
        }
        if let Some(k) = failure {
            return Ok(ReplayOutcome::Failed {
                turn: r.turn,
                failure: k,
                usage,
            });
        }
        let u = usage.ok_or(Error::UnknownUsage)?;
        if refused {
            return Ok(ReplayOutcome::Refused {
                turn: r.turn,
                usage: u,
            });
        }
        if d.text.trim().is_empty() || u.output == 0 {
            return Err(Error::Stream);
        }
        Ok(ReplayOutcome::Completed(Answer {
            turn: r.turn,
            text: d.text,
            simulated: true,
            usage: Some(u),
        }))
    }

    /// Strict decoder only; preserved reference behavior and conservative accounting.
    pub fn run_sse(
        &mut self,
        r: &CompiledRequest,
        context: Context,
        input: impl Read,
        cancelled: impl FnMut() -> bool,
    ) -> Result<Answer, Error> {
        self.validate(r, context)?;
        if let Some(text) = resolve_local(&r.text) {
            return Ok(Answer {
                turn: r.turn,
                text: text.into(),
                simulated: false,
                usage: None,
            });
        }
        let e = r.envelope;
        let input_bound = (r.text.len() as u64)
            .checked_add(128)
            .ok_or(Error::Budget)?;
        let amount = input_bound.checked_add(r.max_output).ok_or(Error::Budget)?;
        let priority = if r.priority == Priority::Interactive {
            "I"
        } else {
            "B"
        };
        self.persist(&format!(
            "A {} {} {} {} {} {} {} {} {} {}",
            e.request_id,
            context.now,
            e.expires_at,
            r.turn,
            r.task,
            r.text.len(),
            r.max_output,
            priority,
            r.digest,
            amount
        ))?;
        self.dispatches = self.dispatches.checked_add(1).ok_or(Error::Suspended)?;
        let d = responses::decode(input, cancelled).map_err(|e| match e {
            responses::DecodeError::Cancelled => Error::Cancelled,
            responses::DecodeError::MissingTerminal
            | responses::DecodeError::Io
            | responses::DecodeError::Usage => Error::UnknownUsage,
            responses::DecodeError::Unsupported => Error::Provider(Failure::Unsupported),
            _ => Error::Stream,
        })?;
        let (usage, failure) = match d.terminal {
            Terminal::Completed(u) => (Some(u), None),
            Terminal::Failed(k, u) => (u, Some(k)),
        };
        let halt = matches!(failure, Some(Failure::RateLimited | Failure::Authorization));
        if let Some(u) = usage {
            let actual = usage_total(u).ok_or(Error::UnknownUsage)?;
            if actual > 1_000_000_000 {
                return Err(Error::UnknownUsage);
            }
            let overrun = actual > amount || u.input > input_bound || u.output > r.max_output;
            self.persist(&format!(
                "S {} {} {}",
                e.request_id,
                actual,
                u8::from(halt || overrun)
            ))?;
            if overrun {
                return Err(Error::Overrun);
            }
        } else if halt {
            self.revoke()?;
        }
        if let Some(k) = failure {
            return Err(Error::Provider(k));
        }
        let u = usage.ok_or(Error::UnknownUsage)?;
        if d.text.trim().is_empty() || u.output == 0 {
            return Err(Error::Stream);
        }
        Ok(Answer {
            turn: r.turn,
            text: d.text,
            simulated: true,
            usage: Some(u),
        })
    }
}
fn usage_total(u: Usage) -> Option<u64> {
    if u.cached_input > u.input || u.reasoning_output > u.output {
        return None;
    }
    u.input.checked_add(u.output)
}

#[cfg(test)]
mod arithmetic_tests {
    use super::*;
    #[test]
    fn checked_accounting_overflow() {
        let l = crate::governor::Limits {
            total: u64::MAX,
            background: u64::MAX,
            interactive_reserve: 0,
            per_request: u64::MAX,
        };
        assert!(!reservation_allowed(
            l,
            u64::MAX,
            0,
            Priority::Interactive,
            1
        ));
        assert!(!reservation_allowed(
            l,
            0,
            u64::MAX,
            Priority::Background,
            1
        ));
        assert_eq!(
            usage_total(Usage {
                input: u64::MAX,
                output: 1,
                cached_input: 0,
                reasoning_output: 0
            }),
            None
        );
    }

    #[test]
    fn failed_write_and_sync_never_dispatch() {
        // Private unit fault injection uses real OS I/O failures, no public fault switch.
        let c = Configuration {
            version: 1,
            model: 7,
            model_eligible: true,
            policy: Policy {
                account: 1,
                host: 2,
                grant: 3,
                window_start: 100,
                window_end: 1000,
                grant_expiry: 900,
                eligible: true,
                direct_scope: true,
                credits_disabled: true,
                auto_purchase_disabled: true,
                app_limit_percent: 90,
                limits: crate::governor::Limits::default(),
            },
        };
        for device in ["/dev/full", "/dev/null"] {
            let root = std::env::temp_dir().join(format!(
                "atomos-atomic-fault-{}-{}",
                std::process::id(),
                if device == "/dev/full" { 0 } else { 1 }
            ));
            fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
            let mut b = AtomicReplay::initialize(&root, c).unwrap();
            b.file = OpenOptions::new().write(true).open(device).unwrap();
            struct Never;
            impl Read for Never {
                fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                    panic!("read after persistence failure")
                }
            }
            let e = Envelope {
                account: 1,
                host: 2,
                grant: 3,
                request_id: 1,
                now: 200,
                expires_at: 230,
            };
            let r = CompiledRequest::compile(
                e,
                7,
                1,
                Request {
                    turn: 4,
                    task: 5,
                    text: "question",
                    priority: Priority::Interactive,
                    max_output: 64,
                },
            )
            .unwrap();
            assert_eq!(
                b.run_sse(
                    &r,
                    Context {
                        now: 200,
                        active_turn: 4,
                        task: 5
                    },
                    Never,
                    || false
                ),
                Err(Error::Storage)
            );
            assert_eq!(b.dispatches(), 0);
            assert_eq!(b.charged(), 0);
            assert!(b.suspended());
            drop(b);
            fs::remove_dir_all(&root).unwrap();
        }
    }
}

#[cfg(test)]
mod extended_fault_tests {
    use super::*;
    #[test]
    fn session_storage_fault_exposes_only_last_confirmed_accounting() {
        use crate::session::{Session, Event, replay::Parameters};
        let root=Root::new();
        let mut b=AtomicReplay::initialize(&root.0,config()).unwrap();
        b.fault=Some((2,false));
        let mut s=Session::default();
        let p=Parameters{envelope:env(1),model:7,version:1,task:5,now:200,priority:crate::governor::Priority::Interactive,max_output:64};
        s.replay(&mut b,"question",p,SSE,||false,|e|{
            assert_eq!(e,&Event::Replay(Err(Error::Storage)));Ok(())
        }).unwrap();
        let status=b.status();
        assert!(!status.healthy);assert_eq!(status.confirmed_total,200);
        assert_eq!(status.pending_units,Some(200));assert_eq!(s.history_pairs(),0);
        s.local("help","local".into(), |_|Ok(())).unwrap();
        drop(b);let b=AtomicReplay::open(&root.0,config()).unwrap();
        assert!(b.status().healthy);assert_eq!(b.status().confirmed_total,47);
        assert_eq!(b.status().pending_units,None);
    }
    use crate::governor::Limits;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Root(std::path::PathBuf);
    impl Root {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!(
                "atomos-outcome-fault-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::DirBuilder::new().mode(0o700).create(&p).unwrap();
            Self(p)
        }
    }
    impl Drop for Root {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn config() -> Configuration {
        Configuration {
            version: 1,
            model: 7,
            model_eligible: true,
            policy: Policy {
                account: 1,
                host: 2,
                grant: 3,
                window_start: 100,
                window_end: 1000,
                grant_expiry: 900,
                eligible: true,
                direct_scope: true,
                credits_disabled: true,
                auto_purchase_disabled: true,
                app_limit_percent: 90,
                limits: Limits::default(),
            },
        }
    }
    fn context() -> Context {
        Context {
            now: 200,
            active_turn: 4,
            task: 5,
        }
    }
    fn env(id: u64) -> Envelope {
        Envelope {
            account: 1,
            host: 2,
            grant: 3,
            request_id: id,
            now: 200,
            expires_at: 230,
        }
    }
    fn req() -> Request<'static> {
        Request {
            turn: 4,
            task: 5,
            text: "question",
            priority: Priority::Interactive,
            max_output: 64,
        }
    }
    fn compiled(id: u64) -> CompiledRequest {
        CompiledRequest::compile(env(id), 7, 1, req()).unwrap()
    }

    const SSE: &[u8] = include_bytes!("../tests/fixtures/responses-text.sse");
    const REFUSAL: &[u8] = include_bytes!("../tests/fixtures/responses-refusal.sse");
    fn auth_failure() -> Vec<u8> {
        let text = std::str::from_utf8(SSE).unwrap().replace("\r\n", "\n");
        let created = text.split("\n\n").next().unwrap();
        format!("{created}\n\nevent: error\ndata: {{\"type\":\"error\",\"sequence_number\":1,\"code\":\"invalid_api_key\",\"message\":\"synthetic secret must not escape\"}}\n\n").into_bytes()
    }
    #[test]
    fn admission_write_and_ambiguous_sync_fail_before_read() {
        struct Never;
        impl Read for Never {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                panic!("read before durable admission")
            }
        }
        for write in [true, false] {
            let f = Root::new();
            let mut b = AtomicReplay::initialize(&f.0, config()).unwrap();
            b.fault = Some((1, write));
            assert_eq!(
                b.run_sse_extended(&compiled(1), context(), Never, || false),
                Err(Error::Storage)
            );
            assert_eq!(b.charged(), 0);
            assert_eq!(b.dispatches(), 0);
            assert!(b.suspended());
            drop(b);
            let b = AtomicReplay::open(&f.0, config()).unwrap();
            assert_eq!(b.charged(), if write { 0 } else { 200 });
            assert_eq!(b.suspended(), !write);
        }
    }
    #[test]
    fn settlement_write_and_ambiguous_sync_never_release_text_or_refusal() {
        for fixture in [SSE, REFUSAL] {
            for write in [true, false] {
                let f = Root::new();
                let mut b = AtomicReplay::initialize(&f.0, config()).unwrap();
                b.fault = Some((2, write));
                assert_eq!(
                    b.run_sse_extended(&compiled(1), context(), fixture, || false),
                    Err(Error::Storage)
                );
                assert_eq!(b.charged(), 200);
                assert!(b.suspended());
                assert_eq!(b.dispatches(), 1);
                drop(b);
                let b = AtomicReplay::open(&f.0, config()).unwrap();
                assert_eq!(b.charged(), if write { 200 } else { 47 });
                assert_eq!(b.suspended(), write);
                assert_eq!(b.next_request_id(), 2);
            }
        }
    }
    #[test]
    fn permanent_halt_write_and_sync_fail_closed_without_provider_payload() {
        let wire = auth_failure();
        for write in [true, false] {
            let f = Root::new();
            let mut b = AtomicReplay::initialize(&f.0, config()).unwrap();
            b.fault = Some((2, write));
            let result = b.run_sse_extended(&compiled(1), context(), wire.as_slice(), || false);
            assert_eq!(result, Err(Error::Storage));
            assert!(!format!("{result:?}").contains("synthetic secret"));
            assert_eq!(b.charged(), 200);
            assert!(b.suspended());
            drop(b);
            let b = AtomicReplay::open(&f.0, config()).unwrap();
            assert_eq!(b.charged(), 200);
            assert!(b.suspended());
            assert_eq!(b.state.halted, !write);
        }
    }
    #[test]
    fn full_capacity_and_torn_settlement_block_delivery_and_repair() {
        struct Grow {
            path: std::path::PathBuf,
            sent: bool,
        }
        impl Read for Grow {
            fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
                if self.sent {
                    return Ok(0);
                }
                self.sent = true;
                OpenOptions::new()
                    .write(true)
                    .open(&self.path)?
                    .set_len(MAX_LOG)?;
                out[..REFUSAL.len()].copy_from_slice(REFUSAL);
                Ok(REFUSAL.len())
            }
        }
        let f = Root::new();
        let mut b = AtomicReplay::initialize(&f.0, config()).unwrap();
        assert_eq!(
            b.run_sse_extended(
                &compiled(1),
                context(),
                Grow {
                    path: f.0.join(NAMESPACE).join("admission.log"),
                    sent: false
                },
                || false
            ),
            Err(Error::Storage)
        );
        assert_eq!(b.charged(), 200);
        assert!(b.suspended());
        drop(b);
        assert!(AtomicReplay::open(&f.0, config()).is_err());
        let g = Root::new();
        let mut b = AtomicReplay::initialize(&g.0, config()).unwrap();
        let _ = b.run_sse_extended(&compiled(1), context(), &b""[..], || false);
        drop(b);
        let mut file = OpenOptions::new()
            .append(true)
            .open(g.0.join(NAMESPACE).join("admission.log"))
            .unwrap();
        file.write_all(b"S 1 4").unwrap();
        file.sync_all().unwrap();
        assert!(matches!(
            AtomicReplay::open(&g.0, config()),
            Err(Error::Corrupt)
        ));
    }
}
