use atomos_tasks::{Error, Runtime, State, Synthetic, MAX_ACTIVE_MS};
use std::fs;
use std::os::unix::fs::{symlink, DirBuilderExt, PermissionsExt};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "atomos-tasks-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&p).unwrap();
        Self(p)
    }
    fn path(&self) -> PathBuf {
        self.0.join("store")
    }
    fn init(&self) -> Runtime {
        Runtime::initialize(&self.path(), "owner", "local-v1", 4 * 1024 * 1024).unwrap()
    }
    fn open(&self) -> Runtime {
        Runtime::open(&self.path(), "owner", "local-v1").unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn add(r: &mut Runtime, id: i64, delta: i64, t: i64) -> i64 {
    let a = r
        .prepare(
            id,
            Synthetic::Add {
                item: "fixture",
                delta,
            },
            t,
        )
        .unwrap();
    r.apply(id, a, t).unwrap();
    assert!(r.verify(id, a, t).unwrap());
    a
}

#[test]
fn acknowledged_submission_and_idempotency_survive_restart() {
    let f = Fixture::new();
    let mut r = f.init();
    let t = r.submit("request-1", "private goal", 1).unwrap();
    assert_eq!(t.state, State::Queued);
    assert_eq!(r.submit("request-1", "private goal", 2).unwrap().id, t.id);
    assert_eq!(r.submit("request-1", "different", 3), Err(Error::Conflict));
    assert_eq!(r.events(0, 64).unwrap().len(), 1);
    drop(r);
    let mut r = f.open();
    assert_eq!(r.get(t.id).unwrap().goal, "private goal");
    assert_eq!(r.start(t.id, 4), Err(Error::State));
    assert!(r.recover(4).unwrap().is_empty());
    assert_eq!(r.submit("request-1", "private goal", 4).unwrap().id, t.id);
}

#[test]
fn fifo_single_active_and_pending_capacity() {
    let f = Fixture::new();
    let mut r = f.init();
    let first = r.submit("first", "first", 1).unwrap().id;
    let second = r.submit("second", "second", 1).unwrap().id;
    assert_eq!(r.start(second, 1), Err(Error::State));
    r.start(first, 1).unwrap();
    assert_eq!(r.start(second, 1), Err(Error::Busy));
    for i in 0..15 {
        r.submit(&format!("key-{i}"), "queued", 1).unwrap();
    }
    assert_eq!(r.submit("overflow", "no", 1), Err(Error::Capacity));
    // Existing submissions remain readable even when the queue is full.
    assert_eq!(r.submit("first", "first", 1).unwrap().id, first);
    assert_eq!(r.wait_for_approval(first, 1), Err(Error::Capacity));
    r.cancel(second, 1).unwrap();
    r.wait_for_approval(first, 1).unwrap();
    assert_eq!(r.get(first).unwrap().state, State::WaitingApproval);
}

#[test]
fn only_verified_effects_can_complete_and_duplicates_never_apply_twice() {
    let f = Fixture::new();
    let mut r = f.init();
    let t = r.submit("one", "fixture", 0).unwrap().id;
    r.start(t, 0).unwrap();
    assert_eq!(r.complete(t, 0), Err(Error::State));
    let a = r
        .prepare(
            t,
            Synthetic::Add {
                item: "fixture",
                delta: 7,
            },
            1,
        )
        .unwrap();
    assert_eq!(r.verify(t, a, 1), Err(Error::Ambiguous));
    assert_eq!(r.complete(t, 1), Err(Error::State));
    r.apply(t, a, 2).unwrap();
    assert_eq!(r.counter("fixture").unwrap(), 7);
    assert_eq!(r.apply(t, a, 2), Err(Error::Conflict));
    assert!(r.verify(t, a, 3).unwrap());
    assert_eq!(r.verify(t, a, 3), Err(Error::State));
    r.complete(t, 4).unwrap();
    assert_eq!(r.get(t).unwrap().state, State::Completed);
    assert_eq!(
        r.prepare(
            t,
            Synthetic::Add {
                item: "fixture",
                delta: 7
            },
            5
        ),
        Err(Error::State)
    );
    drop(r);
    let r = f.open();
    assert_eq!(r.counter("fixture").unwrap(), 7);
    assert_eq!(r.get(t).unwrap().dispatches, 1);
    assert_eq!(r.get(t).unwrap().failure, None);
}

#[test]
fn failed_assertion_cannot_be_reported_as_success() {
    let f = Fixture::new();
    let mut r = f.init();
    let t = r.submit("assert", "fixture", 0).unwrap().id;
    r.start(t, 0).unwrap();
    let a = r
        .prepare(
            t,
            Synthetic::Assert {
                item: "fixture",
                expected: 42,
            },
            0,
        )
        .unwrap();
    r.apply(t, a, 0).unwrap();
    assert!(!r.verify(t, a, 0).unwrap());
    assert_eq!(r.get(t).unwrap().state, State::Failed);
    assert_eq!(r.complete(t, 0), Err(Error::State));
    drop(r);
    assert_eq!(
        f.open().get(t).unwrap().failure.as_deref(),
        Some("fixture_mismatch")
    );
}

#[test]
fn waiting_releases_slot_and_does_not_spend_execution_time() {
    let f = Fixture::new();
    let mut r = f.init();
    let t = r.submit("wait", "fixture", 0).unwrap().id;
    r.start(t, 10).unwrap();
    r.wait_for_approval(t, 110).unwrap();
    let other = r.submit("other", "fixture", 120).unwrap().id;
    r.start(other, 120).unwrap();
    r.cancel(other, 125).unwrap();
    r.requeue(t, 1_000_000).unwrap();
    r.start(t, 1_000_000).unwrap();
    add(&mut r, t, 1, 1_000_010);
    r.complete(t, 1_000_020).unwrap();
    assert_eq!(r.get(t).unwrap().active_ms, 120);
}

#[test]
fn cancellation_blocks_prepared_dispatch_and_preserves_existing_effects() {
    for applied in [false, true] {
        let f = Fixture::new();
        let mut r = f.init();
        let t = r.submit("cancel", "fixture", 0).unwrap().id;
        r.start(t, 0).unwrap();
        let a = r
            .prepare(
                t,
                Synthetic::Add {
                    item: "fixture",
                    delta: 7,
                },
                0,
            )
            .unwrap();
        if applied {
            r.apply(t, a, 0).unwrap();
        }
        r.cancel(t, 1).unwrap();
        assert_eq!(r.get(t).unwrap().state, State::Cancelling);
        assert_eq!(r.apply(t, a, 1), Err(Error::State));
        assert_eq!(
            r.prepare(
                t,
                Synthetic::Add {
                    item: "fixture",
                    delta: 1
                },
                1
            ),
            Err(Error::State)
        );
        r.finish_cancel(t, 2).unwrap();
        r.cancel(t, 3).unwrap();
        assert_eq!(r.get(t).unwrap().state, State::Cancelled);
        assert_eq!(r.counter("fixture").unwrap(), if applied { 7 } else { 0 });
        let status = r.actions(t, 0, 32).unwrap();
        assert_eq!(status.len(), 1);
        assert_eq!(status[0].state, "cancelled");
        assert_eq!(status[0].effect_present, applied);
        assert_eq!(status[0].passed, if applied { Some(true) } else { None });
    }
}

#[test]
fn queued_and_waiting_cancel_are_terminal_without_dispatch() {
    let f = Fixture::new();
    let mut r = f.init();
    let t = r.submit("q", "fixture", 0).unwrap().id;
    r.cancel(t, 1).unwrap();
    assert_eq!(r.get(t).unwrap().state, State::Cancelled);
    let t = r.submit("w", "fixture", 1).unwrap().id;
    r.start(t, 1).unwrap();
    r.wait_for_approval(t, 1).unwrap();
    r.cancel(t, 2).unwrap();
    assert_eq!(r.requeue(t, 2), Err(Error::State));
    assert_eq!(r.counter("fixture").unwrap(), 0);
}

#[test]
fn task_dispatch_and_time_budgets_fail_durably() {
    let f = Fixture::new();
    let mut r = f.init();
    let t = r.submit("dispatch", "fixture", 0).unwrap().id;
    r.start(t, 0).unwrap();
    for _ in 0..32 {
        add(&mut r, t, 1, 0);
    }
    assert_eq!(
        r.prepare(
            t,
            Synthetic::Add {
                item: "fixture",
                delta: 1
            },
            0
        ),
        Err(Error::Budget)
    );
    assert_eq!(r.get(t).unwrap().state, State::Failed);
    assert_eq!(r.counter("fixture").unwrap(), 32);
    assert_eq!(r.get(t).unwrap().failure.as_deref(), Some("dispatch_limit"));
    let t = r.submit("time", "fixture", 0).unwrap().id;
    r.start(t, 0).unwrap();
    assert_eq!(
        r.prepare(
            t,
            Synthetic::Add {
                item: "fixture",
                delta: 1
            },
            MAX_ACTIVE_MS
        ),
        Err(Error::Budget)
    );
    assert_eq!(r.get(t).unwrap().state, State::Failed);
    drop(r);
    let r = f.open();
    assert_eq!(r.get(t).unwrap().state, State::Failed);
    assert_eq!(r.get(t).unwrap().failure.as_deref(), Some("task_time"));
}

#[test]
fn action_deadline_precedes_any_effect_and_late_completion_is_not_success() {
    let f = Fixture::new();
    let mut r = f.init();
    let t = r.submit("deadline", "fixture", 0).unwrap().id;
    r.start(t, 0).unwrap();
    let a = r
        .prepare(
            t,
            Synthetic::Add {
                item: "fixture",
                delta: 7,
            },
            0,
        )
        .unwrap();
    assert_eq!(r.apply(t, a, 60_000), Err(Error::Budget));
    assert_eq!(
        r.get(t).unwrap().failure.as_deref(),
        Some("action_deadline")
    );
    assert_eq!(r.counter("fixture").unwrap(), 0);
    let t = r.submit("complete", "fixture", 60_000).unwrap().id;
    r.start(t, 60_000).unwrap();
    add(&mut r, t, 1, 60_000);
    assert_eq!(r.complete(t, 660_000), Err(Error::Budget));
    assert_eq!(r.get(t).unwrap().state, State::Failed);
}

#[test]
fn clock_rollback_is_rejected_across_restart() {
    let f = Fixture::new();
    let mut r = f.init();
    let t = r.submit("clock", "fixture", 100).unwrap().id;
    assert_eq!(r.start(t, 99), Err(Error::Clock));
    drop(r);
    let mut r = f.open();
    assert_eq!(r.recover(99), Err(Error::Clock));
    assert_eq!(r.start(t, 100), Err(Error::State));
    r.recover(100).unwrap();
    r.start(t, 100).unwrap();
}

#[test]
fn bounded_metadata_events_reconnect_without_resubmission_or_goal_leak() {
    let f = Fixture::new();
    let mut r = f.init();
    let t = r.submit("secret-key", "private-fixture-goal", 0).unwrap();
    r.start(t.id, 1).unwrap();
    r.cancel(t.id, 2).unwrap();
    let e = r.events(0, 1).unwrap();
    assert_eq!(e.len(), 1);
    assert_eq!(e[0].state, State::Queued);
    let next = r.events(e[0].cursor, 64).unwrap();
    assert_eq!(next.len(), 2);
    assert_eq!(next[1].state, State::Cancelled);
    assert!(r.events(next[1].cursor, 64).unwrap().is_empty());
    assert_eq!(r.events(999, 1), Err(Error::Invalid));
    assert_eq!(r.events(0, 65), Err(Error::Invalid));
    assert!(!format!("{t:?} {next:?}").contains("private-fixture-goal"));
    assert!(!format!("{t:?}").contains("secret-key"));
}

#[test]
fn invalid_inputs_and_cross_task_actions_have_no_effect() {
    let f = Fixture::new();
    let mut r = f.init();
    for (key, goal) in [
        ("", "ok"),
        ("space key", "ok"),
        ("valid", ""),
        ("valid", "\x1b[2J"),
    ] {
        assert_eq!(r.submit(key, goal, 0), Err(Error::Invalid));
    }
    assert_eq!(r.submit("large", &"x".repeat(8193), 0), Err(Error::Invalid));
    let t = r.submit("exact", &"x".repeat(8192), 0).unwrap().id;
    r.start(t, 0).unwrap();
    assert_eq!(
        r.prepare(
            t,
            Synthetic::Add {
                item: "bad/path",
                delta: 1
            },
            0
        ),
        Err(Error::Invalid)
    );
    assert_eq!(
        r.prepare(
            t,
            Synthetic::Add {
                item: "fixture",
                delta: i64::MIN
            },
            0
        ),
        Err(Error::Invalid)
    );
    let a = r
        .prepare(
            t,
            Synthetic::Add {
                item: "fixture",
                delta: 7,
            },
            0,
        )
        .unwrap();
    assert_eq!(r.apply(t + 1, a, 0), Err(Error::Missing));
    assert_eq!(r.counter("fixture").unwrap(), 0);
}

#[test]
fn exclusive_process_lease_owner_binding_and_missing_files_fail_closed() {
    let f = Fixture::new();
    let r = f.init();
    assert!(matches!(
        Runtime::open(&f.path(), "owner", "local-v1"),
        Err(Error::Busy)
    ));
    let status = Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "crash_child", "--nocapture"])
        .env("ATOMOS_TASK_FIXTURE", f.path())
        .env("ATOMOS_TASK_STAGE", "contend")
        .env("ATOMOS_TASK_READY", f.0.join("unused"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());
    drop(r);
    assert!(matches!(
        Runtime::open(&f.path(), "wrong", "local-v1"),
        Err(Error::Conflict)
    ));
    assert!(matches!(
        Runtime::open(&f.path(), "owner", "other-policy"),
        Err(Error::Conflict)
    ));
    let r = f.open();
    drop(r);
    fs::remove_file(f.path().join("tasks.db")).unwrap();
    assert!(Runtime::open(&f.path(), "owner", "local-v1").is_err());
    assert!(!f.path().join("tasks.db").exists());
}

#[test]
fn symlinks_hardlinks_permissions_and_unknown_initialization_are_rejected() {
    let f = Fixture::new();
    let r = f.init();
    drop(r);
    let alias = f.0.join("alias");
    symlink(f.path(), &alias).unwrap();
    assert!(matches!(
        Runtime::open(&alias, "owner", "local-v1"),
        Err(Error::Invalid)
    ));
    fs::hard_link(f.path().join("tasks.db"), f.0.join("copy")).unwrap();
    assert!(matches!(
        Runtime::open(&f.path(), "owner", "local-v1"),
        Err(Error::Invalid)
    ));
    fs::set_permissions(f.path().join("tasks.db"), fs::Permissions::from_mode(0o600)).unwrap();
    symlink(f.0.join("absent"), f.path().join("lease.db-journal")).unwrap();
    assert!(matches!(
        Runtime::open(&f.path(), "owner", "local-v1"),
        Err(Error::Invalid)
    ));
    fs::remove_file(f.0.join("copy")).unwrap();
    fs::set_permissions(f.path().join("tasks.db"), fs::Permissions::from_mode(0o644)).unwrap();
    assert!(matches!(
        Runtime::open(&f.path(), "owner", "local-v1"),
        Err(Error::Invalid)
    ));
    let f = Fixture::new();
    fs::write(f.0.join("unknown"), b"preserve").unwrap();
    assert!(matches!(
        Runtime::initialize(&f.0, "owner", "local-v1", 4 * 1024 * 1024),
        Err(Error::Conflict)
    ));
    assert_eq!(fs::read(f.0.join("unknown")).unwrap(), b"preserve");
}

#[test]
fn database_full_poisoning_preserves_acknowledged_tasks() {
    let f = Fixture::new();
    let mut r = Runtime::initialize(&f.path(), "owner", "local-v1", 65_536).unwrap();
    let mut accepted = Vec::new();
    let mut full = false;
    for n in 0..16 {
        match r.submit(&format!("key-{n}"), &"x".repeat(8192), n) {
            Ok(t) => accepted.push(t.id),
            Err(Error::Storage) => {
                full = true;
                break;
            }
            other => panic!("{other:?}"),
        }
    }
    assert!(full && !accepted.is_empty());
    assert_eq!(r.submit("after", "no", 20), Err(Error::Unhealthy));
    drop(r);
    let r = f.open();
    for id in &accepted {
        assert_eq!(r.get(*id).unwrap().goal.len(), 8192);
    }
    let raw = rusqlite::Connection::open(f.path().join("tasks.db")).unwrap();
    assert_eq!(
        raw.query_row("SELECT count(*) FROM task", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        accepted.len() as i64
    );
}

#[test]
fn cancellation_ack_latency_fixture_is_bounded_without_a_worker_claim() {
    let f = Fixture::new();
    let mut r = f.init();
    let mut times = Vec::new();
    for n in 0..30 {
        let t = r.submit(&format!("key-{n}"), "fixture", n).unwrap().id;
        r.start(t, n).unwrap();
        let a = r
            .prepare(
                t,
                Synthetic::Add {
                    item: "fixture",
                    delta: 1,
                },
                n,
            )
            .unwrap();
        let start = Instant::now();
        r.cancel(t, n).unwrap();
        times.push(start.elapsed());
        assert_eq!(r.apply(t, a, n), Err(Error::State));
        r.finish_cancel(t, n).unwrap();
    }
    times.sort();
    assert!(times[28] < Duration::from_secs(1));
    assert_eq!(r.counter("fixture").unwrap(), 0);
    // No worker is created here; real worker stop/load and device A07 remain pending.
}

#[test]
fn actual_child_death_reconciles_every_dispatch_boundary_without_reexecution() {
    for stage in [
        "before-intent",
        "after-intent",
        "uncommitted-effect",
        "after-effect",
        "after-verification",
        "after-complete",
        "cancel-before-effect",
        "cancel-after-effect",
    ] {
        let f = Fixture::new();
        drop(f.init());
        let marker = f.0.join("ready");
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", "crash_child", "--nocapture"])
            .env("ATOMOS_TASK_FIXTURE", f.path())
            .env("ATOMOS_TASK_STAGE", stage)
            .env("ATOMOS_TASK_READY", &marker)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let start = Instant::now();
        while !marker.exists() && start.elapsed() < Duration::from_secs(10) {
            if child.try_wait().unwrap().is_some() {
                panic!("fixture exited before {stage}");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        if !marker.exists() {
            let _ = child.kill();
            let _ = child.wait();
            panic!("fixture timeout {stage}");
        }
        child.kill().unwrap();
        child.wait().unwrap();
        let mut r = f.open();
        assert_eq!(r.start(1, 20), Err(Error::State));
        let recovered = r.recover(20).unwrap();
        if stage == "after-complete" {
            assert!(recovered.is_empty());
            assert_eq!(r.get(1).unwrap().state, State::Completed);
        } else if stage.starts_with("cancel-") {
            assert_eq!(r.get(1).unwrap().state, State::Cancelling);
            r.finish_cancel(1, 21).unwrap();
            assert_eq!(r.get(1).unwrap().state, State::Cancelled);
        } else {
            assert_eq!(recovered, vec![1]);
            assert_eq!(r.get(1).unwrap().state, State::Interrupted);
            match stage {
                "before-intent" => {
                    r.requeue(1, 21).unwrap();
                    r.start(1, 22).unwrap();
                    add(&mut r, 1, 7, 23);
                    r.complete(1, 24).unwrap();
                }
                "after-effect" | "after-verification" => {
                    if stage == "after-effect" {
                        assert!(r.verify(1, 1, 21).unwrap());
                    }
                    r.requeue(1, 22).unwrap();
                    r.start(1, 23).unwrap();
                    r.complete(1, 24).unwrap();
                }
                _ => {
                    assert_eq!(r.verify(1, 1, 21), Err(Error::Ambiguous));
                    assert_eq!(r.requeue(1, 21), Err(Error::Ambiguous));
                    r.cancel(1, 22).unwrap();
                    r.finish_cancel(1, 23).unwrap();
                }
            }
        }
        assert_eq!(
            r.counter("fixture").unwrap(),
            if matches!(
                stage,
                "after-intent" | "uncommitted-effect" | "cancel-before-effect"
            ) {
                0
            } else {
                7
            },
            "{stage}"
        );
        assert_eq!(r.submit("request", "fixture goal", 30).unwrap().id, 1);
    }
}

#[test]
#[ignore = "subprocess-only crash fixture"]
fn crash_child() {
    let path = PathBuf::from(std::env::var_os("ATOMOS_TASK_FIXTURE").unwrap());
    let stage = std::env::var("ATOMOS_TASK_STAGE").unwrap();
    let marker = PathBuf::from(std::env::var_os("ATOMOS_TASK_READY").unwrap());
    if stage == "contend" {
        assert!(matches!(
            Runtime::open(&path, "owner", "local-v1"),
            Err(Error::Busy)
        ));
        return;
    }
    let mut r = Runtime::open(&path, "owner", "local-v1").unwrap();
    r.recover(0).unwrap();
    let t = r.submit("request", "fixture goal", 1).unwrap().id;
    r.start(t, 2).unwrap();
    let mut raw = None;
    if stage != "before-intent" {
        let a = r
            .prepare(
                t,
                Synthetic::Add {
                    item: "fixture",
                    delta: 7,
                },
                3,
            )
            .unwrap();
        if stage == "uncommitted-effect" {
            let c = rusqlite::Connection::open(path.join("tasks.db")).unwrap();
            c.execute_batch("BEGIN IMMEDIATE; INSERT INTO counter VALUES('fixture',7)")
                .unwrap();
            raw = Some(c);
        } else if stage != "after-intent" && stage != "cancel-before-effect" {
            r.apply(t, a, 4).unwrap();
        }
        if matches!(stage.as_str(), "after-verification" | "after-complete") {
            assert!(r.verify(t, a, 5).unwrap());
        }
        if stage == "after-complete" {
            r.complete(t, 6).unwrap();
        }
        if stage.starts_with("cancel-") {
            r.cancel(t, 7).unwrap();
        }
    }
    fs::write(&marker, b"ready").unwrap();
    fs::File::open(&marker).unwrap().sync_all().unwrap();
    // Hold runtime, OS lease and optional transaction until the parent kills us.
    loop {
        std::hint::black_box((&r, &raw));
        std::thread::sleep(Duration::from_secs(1));
    }
}
