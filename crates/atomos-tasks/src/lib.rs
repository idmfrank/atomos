#![forbid(unsafe_code)]
//! Durable, single-owner synthetic task runtime. No commands, network or credentials.
//! Caller time is testable and rollback-checked, not an authenticated clock.

use rusqlite::{
    params, Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior,
};
use std::fmt;
use std::fs::{self, DirBuilder, OpenOptions};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::Path;
use std::time::Duration;

const APP_ID: i64 = 0x41545331;
pub const MAX_PENDING: i64 = 16;
pub const MAX_ACTIONS: i64 = 32;
pub const MAX_ACTIVE_MS: i64 = 600_000;
pub const ACTION_MS: i64 = 60_000;
const SCHEMA: &str = include_str!("schema.sql");
pub mod worker;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Missing,
    Conflict,
    Capacity,
    Busy,
    State,
    Clock,
    Budget,
    Ambiguous,
    Storage,
    Unhealthy,
}
pub type Result<T> = std::result::Result<T, Error>;
impl From<rusqlite::Error> for Error {
    fn from(_: rusqlite::Error) -> Self {
        Self::Storage
    }
}
impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self {
        Self::Storage
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Task runtime: {self:?}")
    }
}
impl std::error::Error for Error {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Queued,
    Running,
    WaitingApproval,
    Cancelling,
    Interrupted,
    Completed,
    Failed,
    Cancelled,
}
impl State {
    fn text(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::WaitingApproval => "waiting_approval",
            Self::Cancelling => "cancelling",
            Self::Interrupted => "interrupted",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
    fn parse(s: &str) -> Result<Self> {
        Ok(match s {
            "queued" => Self::Queued,
            "running" => Self::Running,
            "waiting_approval" => Self::WaitingApproval,
            "cancelling" => Self::Cancelling,
            "interrupted" => Self::Interrupted,
            "completed" => Self::Completed,
            "failed" => Self::Failed,
            "cancelled" => Self::Cancelled,
            _ => return Err(Error::Storage),
        })
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Task {
    pub id: i64,
    pub submission_key: String,
    pub goal: String,
    pub state: State,
    pub dispatches: i64,
    pub active_ms: i64,
    pub failure: Option<String>,
}
impl fmt::Debug for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Task")
            .field("id", &self.id)
            .field("state", &self.state)
            .field("dispatches", &self.dispatches)
            .field("active_ms", &self.active_ms)
            .field("failure", &self.failure)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    pub cursor: i64,
    pub task_id: i64,
    pub state: State,
    pub time: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionStatus {
    pub id: i64,
    pub task_id: i64,
    pub state: String,
    /// A cancelled task can have a committed effect. Never imply rollback.
    pub effect_present: bool,
    pub passed: Option<bool>,
}

/// Only built-in fixture operations are supported. This is not a tool permission.
#[derive(Debug, Clone, Copy)]
pub enum Synthetic<'a> {
    Add { item: &'a str, delta: i64 },
    Assert { item: &'a str, expected: i64 },
}

pub struct Runtime {
    conn: Connection,
    _lease: Connection,
    healthy: bool,
    recovered: bool,
}

fn token(s: &str, max: usize) -> Result<()> {
    if s.is_empty()
        || s.len() > max
        || !s
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-.:".contains(&b))
    {
        return Err(Error::Invalid);
    }
    Ok(())
}
fn uid() -> Result<u32> {
    Ok(fs::metadata("/proc/self")?.uid())
}
fn directory(p: &Path) -> Result<()> {
    let m = fs::symlink_metadata(p)?;
    if !p.is_absolute()
        || p.canonicalize()? != p
        || !m.is_dir()
        || m.mode() & 0o077 != 0
        || m.uid() != uid()?
    {
        return Err(Error::Invalid);
    }
    Ok(())
}
fn file(p: &Path) -> Result<()> {
    let m = fs::symlink_metadata(p)?;
    if !m.is_file() || m.mode() & 0o077 != 0 || m.nlink() != 1 || m.uid() != uid()? {
        return Err(Error::Invalid);
    }
    Ok(())
}
fn connect(p: &Path) -> Result<Connection> {
    file(p)?;
    let c = Connection::open_with_flags(
        p,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    c.busy_timeout(Duration::from_millis(100))?;
    Ok(c)
}
fn lease(dir: &Path) -> Result<Connection> {
    let c = connect(&dir.join("lease.db"))?;
    c.execute_batch("BEGIN EXCLUSIVE")
        .map_err(|_| Error::Busy)?;
    Ok(c)
}
fn configure(c: &Connection, pages: i64) -> Result<()> {
    c.execute_batch("PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL; PRAGMA trusted_schema=OFF; PRAGMA cache_size=-1024; PRAGMA secure_delete=ON;")?;
    let mode: String = c.query_row("PRAGMA journal_mode=DELETE", [], |r| r.get(0))?;
    let size: i64 = c.query_row("PRAGMA page_size", [], |r| r.get(0))?;
    let actual: i64 = c.query_row(&format!("PRAGMA max_page_count={pages}"), [], |r| r.get(0))?;
    if mode != "delete" || size != 4096 || actual != pages {
        return Err(Error::Invalid);
    }
    Ok(())
}
fn task(c: &Connection, id: i64) -> Result<Task> {
    let (key, goal, state, n, ms, failure): (String, String, String, i64, i64, Option<String>) = c
        .query_row(
            "SELECT submission_key,goal,state,dispatches,active_ms,failure FROM task WHERE id=?1",
            [id],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            },
        )
        .optional()?
        .ok_or(Error::Missing)?;
    Ok(Task {
        id,
        submission_key: key,
        goal,
        state: State::parse(&state)?,
        dispatches: n,
        active_ms: ms,
        failure,
    })
}
fn transition(c: &Connection, id: i64, state: State, now: i64) -> Result<()> {
    c.execute("UPDATE task SET state=?2,running_since=CASE WHEN ?2='running' THEN ?3 ELSE NULL END WHERE id=?1",params![id,state.text(),now])?;
    c.execute(
        "INSERT INTO event(task_id,state,time) VALUES(?1,?2,?3)",
        params![id, state.text(), now],
    )?;
    Ok(())
}
fn fail(c: &Connection, id: i64, reason: &str, now: i64) -> Result<()> {
    c.execute(
        "UPDATE task SET failure=?2 WHERE id=?1",
        params![id, reason],
    )?;
    transition(c, id, State::Failed, now)
}
fn unresolved(c: &Connection, id: i64) -> Result<i64> {
    Ok(c.query_row(
        "SELECT count(*) FROM action WHERE task_id=?1 AND state IN ('prepared','interrupted')",
        [id],
        |r| r.get(0),
    )?)
}
fn charge(c: &Connection, id: i64, now: i64) -> Result<bool> {
    let (since, used): (Option<i64>, i64) = c.query_row(
        "SELECT running_since,active_ms FROM task WHERE id=?1",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let total = used
        .checked_add(since.map_or(0, |v| now - v))
        .ok_or(Error::Clock)?;
    c.execute("UPDATE task SET active_ms=?2,running_since=CASE WHEN running_since IS NULL THEN NULL ELSE ?3 END WHERE id=?1",params![id,total,now])?;
    Ok(total >= MAX_ACTIVE_MS)
}

impl Runtime {
    pub(crate) fn fixture_matches(&self, id: i64, item: &str, delta: i64) -> Result<bool> {
        if !self.healthy {
            return Err(Error::Unhealthy);
        }
        Ok(self.conn.query_row("SELECT count(*)=0 FROM action WHERE task_id=?1 AND (kind!='add' OR item!=?2 OR argument!=?3)",params![id,item,delta],|r|r.get(0))?)
    }
    /// Scheduler discovery only: never claims, starts or retries a task.
    pub fn next_queued(&self) -> Result<Option<i64>> {
        if !self.healthy {
            return Err(Error::Unhealthy);
        }
        Ok(self
            .conn
            .query_row("SELECT min(id) FROM task WHERE state='queued'", [], |r| {
                r.get(0)
            })?)
    }
    /// Explicit fresh initialization. `store_bytes` bounds the main DB only;
    /// rollback journal and filesystem free-space reserves require deployment policy.
    pub fn initialize(dir: &Path, owner: &str, policy: &str, store_bytes: u64) -> Result<Self> {
        token(owner, 64)?;
        token(policy, 64)?;
        if store_bytes % 4096 != 0 || !(65_536..=268_435_456).contains(&store_bytes) {
            return Err(Error::Invalid);
        }
        if !dir.exists() {
            let parent = dir.parent().ok_or(Error::Invalid)?;
            if !parent.is_absolute() || parent.canonicalize()? != parent {
                return Err(Error::Invalid);
            }
            DirBuilder::new().mode(0o700).create(dir)?;
        }
        directory(dir)?;
        if fs::read_dir(dir)?.next().is_some() {
            return Err(Error::Conflict);
        }
        for name in ["lease.db", "tasks.db"] {
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(dir.join(name))?
                .sync_all()?;
        }
        let guard = lease(dir)?;
        let mut c = connect(&dir.join("tasks.db"))?;
        c.execute_batch("PRAGMA page_size=4096")?;
        let pages = (store_bytes / 4096) as i64;
        configure(&c, pages)?;
        let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute_batch(SCHEMA)?;
        tx.execute(
            "INSERT INTO configuration VALUES(1,?1,?2,?3,0)",
            params![owner, policy, pages],
        )?;
        tx.pragma_update(None, "application_id", APP_ID)?;
        tx.pragma_update(None, "user_version", 1)?;
        tx.commit()?;
        fs::File::open(dir)?.sync_all()?;
        Ok(Self {
            conn: c,
            _lease: guard,
            healthy: true,
            recovered: true,
        })
    }

    /// Acquire an OS-released SQLite lease; never reset missing or invalid state.
    /// Recovery is explicit after open, so an inspecting caller cannot resume work.
    pub fn open(dir: &Path, owner: &str, policy: &str) -> Result<Self> {
        token(owner, 64)?;
        token(policy, 64)?;
        directory(dir)?;
        for name in ["tasks.db-journal", "lease.db-journal"] {
            let p = dir.join(name);
            match fs::symlink_metadata(&p) {
                Ok(_) => file(&p)?,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
        }
        for name in [
            "tasks.db-wal",
            "tasks.db-shm",
            "lease.db-wal",
            "lease.db-shm",
        ] {
            match fs::symlink_metadata(dir.join(name)) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                _ => return Err(Error::Invalid),
            }
        }
        let guard = lease(dir)?;
        let c = connect(&dir.join("tasks.db"))?;
        let app: i64 = c.query_row("PRAGMA application_id", [], |r| r.get(0))?;
        let version: i64 = c.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if app != APP_ID || version != 1 {
            return Err(Error::Invalid);
        }
        let (o, p, pages): (String, String, i64) = c.query_row(
            "SELECT owner,policy,page_cap FROM configuration WHERE singleton=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        if o != owner || p != policy || !(16..=65536).contains(&pages) {
            return Err(Error::Conflict);
        }
        configure(&c, pages)?;
        let check: String = c.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
        if check != "ok"
            || c.prepare("PRAGMA foreign_key_check")?
                .query([])?
                .next()?
                .is_some()
        {
            return Err(Error::Storage);
        }
        Ok(Self {
            conn: c,
            _lease: guard,
            healthy: true,
            recovered: false,
        })
    }

    fn write<T>(&mut self, now: i64, f: impl FnOnce(&Transaction<'_>) -> Result<T>) -> Result<T> {
        if !self.healthy {
            return Err(Error::Unhealthy);
        }
        if !self.recovered {
            return Err(Error::State);
        }
        let result = (|| {
            let tx = self
                .conn
                .transaction_with_behavior(TransactionBehavior::Immediate)?;
            let last: i64 = tx.query_row(
                "SELECT last_time FROM configuration WHERE singleton=1",
                [],
                |r| r.get(0),
            )?;
            if now < last || now < 0 {
                return Err(Error::Clock);
            }
            let value = f(&tx)?;
            tx.execute(
                "UPDATE configuration SET last_time=?1 WHERE singleton=1",
                [now],
            )?;
            tx.commit()?;
            Ok(value)
        })();
        if matches!(result, Err(Error::Storage)) {
            self.healthy = false;
        }
        result
    }

    pub fn get(&self, id: i64) -> Result<Task> {
        if !self.healthy {
            return Err(Error::Unhealthy);
        }
        task(&self.conn, id)
    }

    pub fn submit(&mut self, key: &str, goal: &str, now: i64) -> Result<Task> {
        token(key, 128)?;
        if goal.trim().is_empty()
            || goal.len() > 8192
            || goal
                .chars()
                .any(|c| c.is_control() && !matches!(c, '\n' | '\t'))
        {
            return Err(Error::Invalid);
        }
        self.write(now,|c|{
            let prior:Option<i64>=c.query_row("SELECT id FROM task WHERE submission_key=?1",[key],|r|r.get(0)).optional()?;
            if let Some(id)=prior {let t=task(c,id)?;return if t.goal==goal {Ok(t)}else{Err(Error::Conflict)};}
            let n:i64=c.query_row("SELECT count(*) FROM task WHERE state IN ('queued','waiting_approval','interrupted')",[],|r|r.get(0))?;
            if n>=MAX_PENDING {return Err(Error::Capacity);}
            c.execute("INSERT INTO task(submission_key,goal,state) VALUES(?1,?2,'queued')",params![key,goal])?;
            let id=c.last_insert_rowid();transition(c,id,State::Queued,now)?;task(c,id)
        })
    }

    pub fn start(&mut self, id: i64, now: i64) -> Result<()> {
        self.write(now, |c| {
            if task(c, id)?.state != State::Queued {
                return Err(Error::State);
            }
            let active: i64 = c.query_row(
                "SELECT count(*) FROM task WHERE state IN ('running','cancelling')",
                [],
                |r| r.get(0),
            )?;
            if active != 0 {
                return Err(Error::Busy);
            }
            let next: i64 =
                c.query_row("SELECT min(id) FROM task WHERE state='queued'", [], |r| {
                    r.get(0)
                })?;
            if next != id {
                return Err(Error::State);
            }
            transition(c, id, State::Running, now)
        })
    }

    /// Suspend synthetic planning, not an external approval or dispatch grant.
    pub fn wait_for_approval(&mut self, id: i64, now: i64) -> Result<()> {
        self.write(now,|c|{
            if task(c,id)?.state!=State::Running || unresolved(c,id)?!=0 {return Err(Error::State);}
            let pending:i64=c.query_row("SELECT count(*) FROM task WHERE state IN ('queued','waiting_approval','interrupted')",[],|r|r.get(0))?;
            if pending>=MAX_PENDING {return Err(Error::Capacity);}
            let expired=charge(c,id,now)?;
            if expired {fail(c,id,"task_time",now)}else{transition(c,id,State::WaitingApproval,now)}
        })
    }

    /// Only resumes synthetic local work; no API for approving external effects exists.
    pub fn requeue(&mut self, id: i64, now: i64) -> Result<()> {
        self.write(now, |c| {
            let t = task(c, id)?;
            if !matches!(t.state, State::WaitingApproval | State::Interrupted) {
                return Err(Error::State);
            }
            if unresolved(c, id)? != 0 {
                return Err(Error::Ambiguous);
            }
            if t.active_ms >= MAX_ACTIVE_MS {
                fail(c, id, "task_time", now)
            } else {
                transition(c, id, State::Queued, now)
            }
        })
    }

    pub fn prepare(&mut self, id: i64, op: Synthetic<'_>, now: i64) -> Result<i64> {
        let (kind, item, arg) = match op {
            Synthetic::Add { item, delta } => {
                if delta.unsigned_abs() > 1_000_000 {
                    return Err(Error::Invalid);
                }
                ("add", item, delta)
            }
            Synthetic::Assert { item, expected } => ("assert", item, expected),
        };
        token(item, 64)?;
        self.write(now,|c|{
            let t=task(c,id)?;
            if t.state!=State::Running || unresolved(c,id)?!=0 {return Err(Error::State);}
            let expired=charge(c,id,now)?;
            if expired || t.dispatches>=MAX_ACTIONS {
                fail(c,id,if expired {"task_time"}else{"dispatch_limit"},now)?;return Ok(0);
            }
            let old:i64=c.query_row("SELECT value FROM counter WHERE item=?1",[item],|r|r.get(0)).optional()?.unwrap_or(0);
            let expected=if kind=="add" {old.checked_add(arg).ok_or(Error::Invalid)?}else{arg};
            c.execute("INSERT INTO action(task_id,kind,item,argument,expected,state,intent_time) VALUES(?1,?2,?3,?4,?5,'prepared',?6)",params![id,kind,item,arg,expected,now])?;
            let aid=c.last_insert_rowid();
            c.execute("UPDATE task SET dispatches=dispatches+1 WHERE id=?1",[id])?;
            Ok(aid)
        }).and_then(|id|if id==0 {Err(Error::Budget)}else{Ok(id)})
    }

    /// Commit the fixture effect separately from verification to expose crash windows.
    pub fn apply(&mut self, id: i64, aid: i64, now: i64) -> Result<()> {
        self.write(now,|c|{
            if task(c,id)?.state!=State::Running {return Err(Error::State);}
            let (kind,item,arg,expected,state,intent):(String,String,i64,i64,String,i64)=c.query_row("SELECT kind,item,argument,expected,state,intent_time FROM action WHERE id=?1 AND task_id=?2",params![aid,id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).optional()?.ok_or(Error::Missing)?;
            if state!="prepared" {return Err(Error::State);}
            if c.query_row("SELECT count(*) FROM effect WHERE action_id=?1",[aid],|r|r.get::<_,i64>(0))?!=0 {return Err(Error::Conflict);}
            let expired=charge(c,id,now)?;
            if expired || now-intent>=ACTION_MS {
                c.execute("UPDATE action SET state='failed' WHERE id=?1",[aid])?;
                fail(c,id,if expired {"task_time"}else{"action_deadline"},now)?;return Ok(false);
            }
            let actual=if kind=="add" {
                let old:i64=c.query_row("SELECT value FROM counter WHERE item=?1",[&item],|r|r.get(0)).optional()?.unwrap_or(0);
                if old.checked_add(arg)!=Some(expected) {return Err(Error::Conflict);}
                c.execute("INSERT INTO counter VALUES(?1,?2) ON CONFLICT(item) DO UPDATE SET value=excluded.value",params![item,expected])?;
                expected
            }else{c.query_row("SELECT value FROM counter WHERE item=?1",[item],|r|r.get(0)).optional()?.unwrap_or(0)};
            c.execute("INSERT INTO effect VALUES(?1,?2,?3)",params![aid,actual,actual==expected])?;
            Ok(true)
        }).and_then(|ok|if ok {Ok(())}else{Err(Error::Budget)})
    }

    pub fn verify(&mut self, id: i64, aid: i64, now: i64) -> Result<bool> {
        self.write(now, |c| {
            let t = task(c, id)?;
            if !matches!(
                t.state,
                State::Running | State::Cancelling | State::Interrupted
            ) {
                return Err(Error::State);
            }
            let state: String = c
                .query_row(
                    "SELECT state FROM action WHERE id=?1 AND task_id=?2",
                    params![aid, id],
                    |r| r.get(0),
                )
                .optional()?
                .ok_or(Error::Missing)?;
            if !matches!(state.as_str(), "prepared" | "interrupted") {
                return Err(Error::State);
            }
            let passed: Option<bool> = c
                .query_row("SELECT passed FROM effect WHERE action_id=?1", [aid], |r| {
                    r.get(0)
                })
                .optional()?;
            let passed = passed.ok_or(Error::Ambiguous)?;
            let expired = charge(c, id, now)?;
            c.execute(
                "UPDATE action SET state=?2 WHERE id=?1",
                params![aid, if passed { "verified" } else { "failed" }],
            )?;
            if t.state == State::Cancelling {
                transition(c, id, State::Cancelled, now)?;
            } else if !passed || expired {
                fail(
                    c,
                    id,
                    if !passed {
                        "fixture_mismatch"
                    } else {
                        "task_time"
                    },
                    now,
                )?;
            }
            Ok(passed && !expired && t.state != State::Cancelling)
        })
    }

    pub fn complete(&mut self, id: i64, now: i64) -> Result<()> {
        self.write(now, |c| {
            let t = task(c, id)?;
            if t.state != State::Running || t.dispatches == 0 || unresolved(c, id)? != 0 {
                return Err(Error::State);
            }
            let failed: i64 = c.query_row(
                "SELECT count(*) FROM action WHERE task_id=?1 AND state!='verified'",
                [id],
                |r| r.get(0),
            )?;
            if failed != 0 {
                return Err(Error::State);
            }
            let expired = charge(c, id, now)?;
            if expired {
                fail(c, id, "task_time", now)?;
            } else {
                transition(c, id, State::Completed, now)?;
            }
            Ok(!expired)
        })
        .and_then(|ok| if ok { Ok(()) } else { Err(Error::Budget) })
    }

    pub fn cancel(&mut self, id: i64, now: i64) -> Result<()> {
        self.write(now, |c| {
            let t = task(c, id)?;
            if matches!(
                t.state,
                State::Completed | State::Failed | State::Cancelled | State::Cancelling
            ) {
                return Ok(());
            }
            charge(c, id, now)?;
            let state = if unresolved(c, id)? == 0 {
                State::Cancelled
            } else {
                State::Cancelling
            };
            transition(c, id, state, now)
        })
    }

    /// Reconcile cancellation without executing an intent. Existing effects remain.
    pub fn finish_cancel(&mut self, id: i64, now: i64) -> Result<()> {
        self.write(now,|c|{
            if task(c,id)?.state!=State::Cancelling {return Err(Error::State);}
            c.execute("UPDATE action SET state='cancelled' WHERE task_id=?1 AND state IN ('prepared','interrupted')",[id])?;
            transition(c,id,State::Cancelled,now)
        })
    }

    /// No execution on recovery: preserve intent/effects, charge interrupted wall
    /// time conservatively and mark every active task interrupted (cancel stays cancel).
    pub fn recover(&mut self, now: i64) -> Result<Vec<i64>> {
        self.recovered = true;
        let result = self.write(now, |c| {
            let mut q = c.prepare(
                "SELECT id FROM task WHERE state IN ('running','cancelling') ORDER BY id",
            )?;
            let ids = q
                .query_map([], |r| r.get::<_, i64>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            for id in &ids {
                let state = task(c, *id)?.state;
                charge(c, *id, now)?;
                c.execute(
                    "UPDATE action SET state='interrupted' WHERE task_id=?1 AND state='prepared'",
                    [id],
                )?;
                transition(
                    c,
                    *id,
                    if state == State::Cancelling {
                        State::Cancelling
                    } else {
                        State::Interrupted
                    },
                    now,
                )?;
            }
            Ok(ids)
        });
        if result.is_err() {
            self.recovered = false;
        }
        result
    }

    pub fn counter(&self, item: &str) -> Result<i64> {
        token(item, 64)?;
        if !self.healthy {
            return Err(Error::Unhealthy);
        }
        Ok(self
            .conn
            .query_row("SELECT value FROM counter WHERE item=?1", [item], |r| {
                r.get(0)
            })
            .optional()?
            .unwrap_or(0))
    }

    pub fn actions(&self, id: i64, after: i64, limit: usize) -> Result<Vec<ActionStatus>> {
        if !self.healthy {
            return Err(Error::Unhealthy);
        }
        if after < 0 || !(1..=32).contains(&limit) {
            return Err(Error::Invalid);
        }
        task(&self.conn, id)?;
        let mut q=self.conn.prepare("SELECT a.id,a.state,e.action_id IS NOT NULL,e.passed FROM action a LEFT JOIN effect e ON e.action_id=a.id WHERE a.task_id=?1 AND a.id>?2 ORDER BY a.id LIMIT ?3")?;
        let mut rows = q.query(params![id, after, limit as i64])?;
        let mut out = Vec::new();
        while let Some(r) = rows.next()? {
            out.push(ActionStatus {
                id: r.get(0)?,
                task_id: id,
                state: r.get(1)?,
                effect_present: r.get(2)?,
                passed: r.get(3)?,
            });
        }
        Ok(out)
    }

    /// Metadata-only bounded cursor replay. No goal, key or fixture values.
    pub fn events(&self, after: i64, limit: usize) -> Result<Vec<Event>> {
        if !self.healthy {
            return Err(Error::Unhealthy);
        }
        if after < 0 || !(1..=64).contains(&limit) {
            return Err(Error::Invalid);
        }
        let last: i64 =
            self.conn
                .query_row("SELECT coalesce(max(cursor),0) FROM event", [], |r| {
                    r.get(0)
                })?;
        if after > last {
            return Err(Error::Invalid);
        }
        let mut q = self.conn.prepare(
            "SELECT cursor,task_id,state,time FROM event WHERE cursor>?1 ORDER BY cursor LIMIT ?2",
        )?;
        let mut rows = q.query(params![after, limit as i64])?;
        let mut out = Vec::new();
        while let Some(r) = rows.next()? {
            out.push(Event {
                cursor: r.get(0)?,
                task_id: r.get(1)?,
                state: State::parse(&r.get::<_, String>(2)?)?,
                time: r.get(3)?,
            });
        }
        Ok(out)
    }
}
