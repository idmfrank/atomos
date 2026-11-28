use atomos_tasks::{
    worker::{Command, Response, ServiceError, Worker, POLICY},
    Error, Runtime, State, Synthetic,
};
use std::fs;
use std::os::unix::fs::DirBuilderExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "atomos-worker-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&p).unwrap();
        drop(Runtime::initialize(&p, "fixture", POLICY, 4 * 1024 * 1024).unwrap());
        Self(p)
    }
    fn start(&self, ms: u64) -> Worker {
        Worker::start(&self.0, "fixture", Duration::from_millis(ms)).unwrap()
    }
    fn open(&self) -> Runtime {
        Runtime::open(&self.0, "fixture", POLICY).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn task(h: &atomos_tasks::worker::Handle, c: Command) -> atomos_tasks::Task {
    match h.request(c).unwrap() {
        Response::Task(t) => t,
        _ => panic!("task response expected"),
    }
}
fn submitted(h: &atomos_tasks::worker::Handle, key: &str) -> i64 {
    task(
        h,
        Command::Submit {
            key: key.into(),
            goal: "synthetic private goal".into(),
        },
    )
    .id
}
fn wait(
    h: &atomos_tasks::worker::Handle,
    id: i64,
    p: impl Fn(&atomos_tasks::Task) -> bool,
) -> atomos_tasks::Task {
    let start = Instant::now();
    loop {
        let t = task(h, Command::Status(id));
        if p(&t) {
            return t;
        }
        assert!(start.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

#[test]
fn worker_runs_fixed_recipe_and_duplicate_submit_never_repeats_it() {
    let f = Fixture::new();
    let w = f.start(0);
    let h = w.handle();
    let id = submitted(&h, "one");
    wait(&h, id, |t| t.state == State::Completed);
    assert_eq!(submitted(&h, "one"), id);
    assert!(matches!(
        h.request(Command::Submit {
            key: "one".into(),
            goal: "different".into()
        }),
        Err(ServiceError::Runtime(Error::Conflict))
    ));
    w.shutdown().unwrap();
    assert_eq!(f.open().counter("worker-fixture").unwrap(), 1);
    assert!(matches!(
        h.request(Command::Status(id)),
        Err(ServiceError::Stopped)
    ));
}

#[test]
fn cancellation_is_durable_and_delayed_effect_never_runs_after_ack() {
    let f = Fixture::new();
    let w = f.start(30_000);
    let h = w.handle();
    let id = submitted(&h, "cancel");
    wait(&h, id, |t| t.dispatches == 1);
    let started = Instant::now();
    assert_eq!(task(&h, Command::Cancel(id)).state, State::Cancelled);
    assert!(started.elapsed() < Duration::from_secs(1));
    std::thread::sleep(Duration::from_millis(80));
    w.shutdown().unwrap();
    let r = f.open();
    assert_eq!(r.counter("worker-fixture").unwrap(), 0);
    assert_eq!(r.actions(id, 0, 32).unwrap()[0].effect_present, false);
}

#[test]
fn clean_stop_preserves_interruption_and_queued_work_resumes_next_start() {
    let f = Fixture::new();
    let w = f.start(30_000);
    let h = w.handle();
    let first = submitted(&h, "active");
    wait(&h, first, |t| t.dispatches == 1);
    let second = submitted(&h, "queued");
    let started = Instant::now();
    w.shutdown().unwrap();
    assert!(started.elapsed() < Duration::from_secs(5));
    let r = f.open();
    assert_eq!(r.get(first).unwrap().state, State::Interrupted);
    assert_eq!(r.get(second).unwrap().state, State::Queued);
    drop(r);
    let w = f.start(0);
    let h = w.handle();
    wait(&h, second, |t| t.state == State::Completed);
    assert!(matches!(
        h.request(Command::Resume(first)),
        Err(ServiceError::Runtime(Error::Ambiguous))
    ));
    assert_eq!(task(&h, Command::Cancel(first)).state, State::Cancelled);
    w.shutdown().unwrap();
    assert_eq!(f.open().counter("worker-fixture").unwrap(), 1);
}

#[test]
fn resume_reconciles_a_known_effect_without_dispatching_it_again() {
    let f = Fixture::new();
    let mut r = f.open();
    let n = now();
    r.recover(n).unwrap();
    let id = r.submit("known", "fixture", n).unwrap().id;
    r.start(id, n).unwrap();
    let a = r
        .prepare(
            id,
            Synthetic::Add {
                item: "worker-fixture",
                delta: 1,
            },
            n,
        )
        .unwrap();
    r.apply(id, a, n).unwrap();
    drop(r);
    let w = f.start(0);
    let h = w.handle();
    assert_eq!(task(&h, Command::Status(id)).state, State::Interrupted);
    task(&h, Command::Resume(id));
    wait(&h, id, |t| t.state == State::Completed);
    w.shutdown().unwrap();
    let r = f.open();
    assert_eq!(r.counter("worker-fixture").unwrap(), 1);
    assert_eq!(r.get(id).unwrap().dispatches, 1);
}

#[test]
fn start_rejects_competing_owner_policy_and_invalid_options() {
    let f = Fixture::new();
    let w = f.start(0);
    assert!(matches!(
        Worker::start(&f.0, "fixture", Duration::ZERO),
        Err(ServiceError::Runtime(Error::Busy))
    ));
    w.shutdown().unwrap();
    assert!(matches!(
        Worker::start(&f.0, "other", Duration::ZERO),
        Err(ServiceError::Runtime(Error::Conflict))
    ));
    assert!(matches!(
        Worker::start(&f.0, "fixture", Duration::from_secs(31)),
        Err(ServiceError::Invalid)
    ));
    assert!(Worker::start(&f.0.join("missing"), "fixture", Duration::ZERO).is_err());
    assert!(!f.0.join("missing").exists());
}

#[test]
fn dropping_worker_quiesces_and_metadata_does_not_resubmit_work() {
    let f = Fixture::new();
    let w = f.start(30_000);
    let h = w.handle();
    let id = submitted(&h, "drop");
    wait(&h, id, |t| t.dispatches == 1);
    match h
        .request(Command::Events {
            after: 0,
            limit: 64,
        })
        .unwrap()
    {
        Response::Events(e) => {
            assert!(e.len() >= 2);
            assert!(!format!("{e:?}").contains("synthetic private goal"));
        }
        _ => panic!("events expected"),
    }
    drop(w);
    let r = f.open();
    assert_eq!(r.counter("worker-fixture").unwrap(), 0);
    assert_eq!(r.get(id).unwrap().state, State::Interrupted);
}

#[test]
fn storage_failure_stops_worker_and_preserves_acknowledged_submissions() {
    let f = Fixture::new();
    let path = f.0.join("small");
    drop(Runtime::initialize(&path, "fixture", POLICY, 65_536).unwrap());
    let w = Worker::start(&path, "fixture", Duration::from_secs(30)).unwrap();
    let h = w.handle();
    let mut accepted = Vec::new();
    for n in 0..16 {
        match h.request(Command::Submit {
            key: format!("full-{n}"),
            goal: "x".repeat(8192),
        }) {
            Ok(Response::Task(t)) => accepted.push(t.id),
            Err(ServiceError::Runtime(Error::Storage | Error::Unhealthy))
            | Err(ServiceError::Stopped | ServiceError::Unconfirmed) => break,
            _ => panic!("unexpected fixture outcome"),
        }
    }
    assert!(!accepted.is_empty());
    assert!(matches!(w.join(), Err(Error::Storage | Error::Unhealthy)));
    assert!(matches!(
        h.request(Command::Status(accepted[0])),
        Err(ServiceError::Stopped)
    ));
    let r = Runtime::open(&path, "fixture", POLICY).unwrap();
    for id in accepted {
        assert_eq!(r.get(id).unwrap().goal.len(), 8192);
    }
}

#[test]
fn resume_rejects_a_different_recipe_even_with_matching_store_policy() {
    let f = Fixture::new();
    let mut r = f.open();
    let n = now();
    r.recover(n).unwrap();
    let id = r.submit("wrong-recipe", "fixture", n).unwrap().id;
    r.start(id, n).unwrap();
    let a = r
        .prepare(
            id,
            Synthetic::Add {
                item: "other-fixture",
                delta: 1,
            },
            n,
        )
        .unwrap();
    r.apply(id, a, n).unwrap();
    drop(r);
    let w = f.start(0);
    let h = w.handle();
    assert!(matches!(
        h.request(Command::Resume(id)),
        Err(ServiceError::Runtime(Error::State))
    ));
    assert_eq!(task(&h, Command::Status(id)).state, State::Interrupted);
    w.shutdown().unwrap();
    assert_eq!(f.open().counter("worker-fixture").unwrap(), 0);
}
