//! One bounded actor owns the store. Fixed counter recipe, no real executor.
use crate::{Error, Event, Result, Runtime, State, Synthetic, Task};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const POLICY: &str = "synthetic-worker-v1";
const POLL: Duration = Duration::from_millis(100);
const REPLY: Duration = Duration::from_secs(5);
pub const CHANNEL_CAP: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceError {
    Runtime(Error),
    Full,
    Stopped,
    Unconfirmed,
    Invalid,
}
impl From<Error> for ServiceError {
    fn from(e: Error) -> Self {
        Self::Runtime(e)
    }
}
type ServiceResult<T> = std::result::Result<T, ServiceError>;

pub enum Command {
    Submit { key: String, goal: String },
    Status(i64),
    Cancel(i64),
    Resume(i64),
    Events { after: i64, limit: usize },
}
pub enum Response {
    Task(Task),
    Events(Vec<Event>),
}
struct Envelope {
    command: Command,
    reply: mpsc::SyncSender<ServiceResult<Response>>,
}

#[derive(Clone)]
pub struct Handle {
    tx: mpsc::SyncSender<Envelope>,
    stop: Arc<AtomicBool>,
}
impl Handle {
    /// A full channel rejects before acceptance. An unconfirmed reply may have
    /// committed: query/retry the SAME submission key, never invent a new one.
    pub fn request(&self, command: Command) -> ServiceResult<Response> {
        if self.stop.load(Ordering::SeqCst) {
            return Err(ServiceError::Stopped);
        }
        // Bound allocation/ownership before admission to the actor channel.
        if let Command::Submit { key, goal } = &command {
            if key.len() > 128 || goal.len() > 8192 {
                return Err(ServiceError::Invalid);
            }
        }
        let (tx, rx) = mpsc::sync_channel(1);
        self.tx
            .try_send(Envelope { command, reply: tx })
            .map_err(|e| match e {
                mpsc::TrySendError::Full(_) => ServiceError::Full,
                mpsc::TrySendError::Disconnected(_) => ServiceError::Stopped,
            })?;
        rx.recv_timeout(REPLY)
            .map_err(|_| ServiceError::Unconfirmed)?
    }
    pub fn stop_flag(&self) -> Arc<AtomicBool> {
        self.stop.clone()
    }
    /// Request quiescence; acknowledgement is Worker::join, not this flag set.
    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

pub struct Worker {
    handle: Handle,
    thread: Option<JoinHandle<Result<()>>>,
}
impl Worker {
    /// Store must already exist; create it explicitly before starting.
    pub fn start(path: &Path, owner: &str, delay: Duration) -> ServiceResult<Self> {
        if delay > Duration::from_secs(30) {
            return Err(ServiceError::Invalid);
        }
        let path: PathBuf = path.into();
        let owner = owner.to_owned();
        let stop = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::sync_channel(CHANNEL_CAP);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let handle = Handle {
            tx,
            stop: stop.clone(),
        };
        let thread = thread::spawn(move || {
            let opened = (|| {
                let mut runtime = Runtime::open(&path, &owner, POLICY)?;
                let clock = Clock::new()?;
                for id in runtime.recover(clock.now()?)? {
                    if runtime.get(id)?.state == State::Cancelling {
                        runtime.finish_cancel(id, clock.now()?)?;
                    }
                }
                Ok((runtime, clock))
            })();
            let (runtime, clock) = match opened {
                Ok(v) => {
                    let _ = ready_tx.send(Ok(()));
                    v
                }
                Err(e) => {
                    let _ = ready_tx.send(Err(e));
                    return Err(e);
                }
            };
            run(runtime, clock, rx, stop, delay)
        });
        match ready_rx.recv() {
            Ok(Ok(())) => Ok(Self {
                handle,
                thread: Some(thread),
            }),
            Ok(Err(e)) => {
                let _ = thread.join();
                Err(e.into())
            }
            Err(_) => {
                let _ = thread.join();
                Err(ServiceError::Stopped)
            }
        }
    }
    pub fn handle(&self) -> Handle {
        self.handle.clone()
    }
    pub fn join(mut self) -> Result<()> {
        self.thread
            .take()
            .ok_or(Error::State)?
            .join()
            .map_err(|_| Error::Storage)?
    }
    pub fn shutdown(self) -> Result<()> {
        self.handle.request_stop();
        self.join()
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.handle.request_stop();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

struct Clock {
    wall: i64,
    anchor: Instant,
}
impl Clock {
    fn new() -> Result<Self> {
        let wall = i64::try_from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| Error::Clock)?
                .as_millis(),
        )
        .map_err(|_| Error::Clock)?;
        Ok(Self {
            wall,
            anchor: Instant::now(),
        })
    }
    fn now(&self) -> Result<i64> {
        self.wall
            .checked_add(
                i64::try_from(self.anchor.elapsed().as_millis()).map_err(|_| Error::Clock)?,
            )
            .ok_or(Error::Clock)
    }
}

enum Phase {
    Prepare,
    Effect(i64, Instant),
    Verify(i64),
    Complete,
}

fn resume(r: &mut Runtime, id: i64, now: i64) -> Result<()> {
    if !r.fixture_matches(id, "worker-fixture", 1)? {
        return Err(Error::State);
    }
    if r.get(id)?.state != State::Interrupted {
        return Err(Error::State);
    }
    let actions = r.actions(id, 0, 32)?;
    if actions.len() > 1 {
        return Err(Error::State);
    }
    for a in actions {
        if a.state == "interrupted" || a.state == "prepared" {
            // Absence is ambiguous; never repeat the effect.
            if !a.effect_present {
                return Err(Error::Ambiguous);
            }
            if !r.verify(id, a.id, now)? {
                return Err(Error::State);
            }
        } else if a.state != "verified" {
            return Err(Error::State);
        }
    }
    r.requeue(id, now)
}

fn command(
    r: &mut Runtime,
    c: Command,
    now: i64,
    current: &mut Option<(i64, Phase)>,
) -> ServiceResult<Response> {
    Ok(match c {
        Command::Submit { key, goal } => Response::Task(r.submit(&key, &goal, now)?),
        Command::Status(id) => Response::Task(r.get(id)?),
        Command::Events { after, limit } => Response::Events(r.events(after, limit)?),
        Command::Resume(id) => {
            resume(r, id, now)?;
            Response::Task(r.get(id)?)
        }
        Command::Cancel(id) => {
            r.cancel(id, now)?;
            if r.get(id)?.state == State::Cancelling {
                r.finish_cancel(id, now)?;
            }
            if current.as_ref().is_some_and(|(task, _)| *task == id) {
                *current = None;
            }
            Response::Task(r.get(id)?)
        }
    })
}

fn tick(
    r: &mut Runtime,
    clock: &Clock,
    current: &mut Option<(i64, Phase)>,
    delay: Duration,
) -> Result<()> {
    let now = clock.now()?;
    if current.is_none() {
        if let Some(id) = r.next_queued()? {
            if !r.fixture_matches(id, "worker-fixture", 1)? {
                return Err(Error::State);
            }
            let a = r.actions(id, 0, 32)?;
            if a.len() > 1 || a.iter().any(|a| a.state != "verified") {
                return Err(Error::State);
            }
            r.start(id, now)?;
            *current = Some((
                id,
                if a.is_empty() {
                    Phase::Prepare
                } else {
                    Phase::Complete
                },
            ));
        }
        return Ok(());
    }
    let (id, phase) = current.take().ok_or(Error::State)?;
    let next = match phase {
        Phase::Prepare => Phase::Effect(
            r.prepare(
                id,
                Synthetic::Add {
                    item: "worker-fixture",
                    delta: 1,
                },
                now,
            )?,
            Instant::now() + delay,
        ),
        Phase::Effect(a, due) if Instant::now() < due => Phase::Effect(a, due),
        Phase::Effect(a, _) => {
            r.apply(id, a, now)?;
            Phase::Verify(a)
        }
        Phase::Verify(a) => {
            if !r.verify(id, a, now)? {
                return Ok(());
            }
            Phase::Complete
        }
        Phase::Complete => {
            r.complete(id, now)?;
            return Ok(());
        }
    };
    *current = Some((id, next));
    Ok(())
}

fn run(
    mut r: Runtime,
    clock: Clock,
    rx: mpsc::Receiver<Envelope>,
    stop: Arc<AtomicBool>,
    delay: Duration,
) -> Result<()> {
    let mut current = None;
    let mut scan = true;
    let result = (|| {
        while !stop.load(Ordering::SeqCst) {
            if scan && current.is_none() {
                tick(&mut r, &clock, &mut current, delay)?;
                scan = false;
            }
            let wait = match &current {
                Some((_, Phase::Effect(_, due))) => {
                    due.saturating_duration_since(Instant::now()).min(POLL)
                }
                Some(_) => Duration::ZERO,
                None => POLL,
            };
            match rx.recv_timeout(wait) {
                Ok(e) => {
                    if stop.load(Ordering::SeqCst) {
                        let _ = e.reply.send(Err(ServiceError::Stopped));
                        break;
                    }
                    if matches!(
                        &e.command,
                        Command::Submit { .. } | Command::Resume(_) | Command::Cancel(_)
                    ) {
                        scan = true;
                    }
                    let answer = command(&mut r, e.command, clock.now()?, &mut current);
                    let fatal = matches!(
                        answer,
                        Err(ServiceError::Runtime(
                            Error::Storage | Error::Unhealthy | Error::Clock
                        ))
                    );
                    let _ = e.reply.send(answer);
                    if fatal {
                        return Err(Error::Unhealthy);
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
            if !stop.load(Ordering::SeqCst) && current.is_some() {
                match tick(&mut r, &clock, &mut current, delay) {
                    Ok(()) => {}
                    Err(Error::Budget) => {
                        current = None;
                    }
                    Err(e) => return Err(e),
                }
                if current.is_none() {
                    scan = true;
                }
            }
        }
        Ok(())
    })();
    stop.store(true, Ordering::SeqCst);
    // Never perform another effect at shutdown. Persist interruption/cancel state.
    let quiesced = (|| {
        for id in r.recover(clock.now()?)? {
            if r.get(id)?.state == State::Cancelling {
                r.finish_cancel(id, clock.now()?)?;
            }
        }
        Ok(())
    })();
    for e in rx.try_iter() {
        let _ = e.reply.send(Err(ServiceError::Stopped));
    }
    result.and(quiesced)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn channel_overflow_and_oversized_submit_are_rejected_before_acceptance() {
        let (tx, rx) = mpsc::sync_channel(CHANNEL_CAP);
        let stop = Arc::new(AtomicBool::new(false));
        let h = Handle {
            tx,
            stop: stop.clone(),
        };
        for _ in 0..CHANNEL_CAP {
            let (reply, _) = mpsc::sync_channel(1);
            h.tx.try_send(Envelope {
                command: Command::Status(1),
                reply,
            })
            .unwrap_or_else(|_| panic!("fixture channel"));
        }
        assert!(matches!(
            h.request(Command::Status(1)),
            Err(ServiceError::Full)
        ));
        assert!(matches!(
            h.request(Command::Submit {
                key: "key".into(),
                goal: "x".repeat(8193)
            }),
            Err(ServiceError::Invalid)
        ));
        assert_eq!(rx.try_iter().count(), CHANNEL_CAP);
        stop.store(true, Ordering::SeqCst);
        assert!(matches!(
            h.request(Command::Status(1)),
            Err(ServiceError::Stopped)
        ));
    }
}
