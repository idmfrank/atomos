use atomos_core::governor::{Error, Governor, Limits, Priority};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "atomos-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn limits() -> Limits {
    Limits { total: 100, background: 30, interactive_reserve: 40, per_request: 60 }
}

#[test]
fn reserve_settle_and_recover() {
    let dir = Fixture::new();
    {
        let mut g = Governor::open(&dir.0, limits()).unwrap();
        let permit = g.reserve(Priority::Interactive, 50).unwrap();
        g.settle(permit, 20).unwrap();
        assert!(matches!(g.settle(permit, 10), Err(Error::Unknown)));
        g.reserve(Priority::Background, 25).unwrap();
        assert_eq!(g.charged(), 45);
    }
    let mut g = Governor::open(&dir.0, limits()).unwrap();
    assert_eq!(g.charged(), 45);
    assert_eq!(g.background_charged(), 25);
    assert!(matches!(g.reserve(Priority::Background, 6), Err(Error::Denied)));
    assert!(matches!(g.reserve(Priority::Interactive, 56), Err(Error::Denied)));
    assert!(g.reserve(Priority::Interactive, 55).is_ok());
    assert!(matches!(g.reserve(Priority::Interactive, 1), Err(Error::Denied)));
}

#[test]
fn interactive_reserve_and_per_request_limit() {
    let dir = Fixture::new();
    let mut g = Governor::open(&dir.0, limits()).unwrap();
    g.reserve(Priority::Interactive, 55).unwrap();
    assert!(matches!(g.reserve(Priority::Background, 6), Err(Error::Denied)));
    assert!(g.reserve(Priority::Background, 5).is_ok());
    assert!(matches!(g.reserve(Priority::Interactive, 61), Err(Error::Denied)));
    assert!(matches!(g.reserve(Priority::Interactive, 0), Err(Error::Denied)));
}

#[test]
fn authoritative_overrun_is_charged_and_blocks_new_calls() {
    let dir = Fixture::new();
    let mut g = Governor::open(&dir.0, limits()).unwrap();
    let r = g.reserve(Priority::Interactive, 50).unwrap();
    g.settle(r, 101).unwrap();
    assert_eq!(g.charged(), 101);
    assert!(matches!(g.reserve(Priority::Interactive, 1), Err(Error::Denied)));
}

#[test]
fn exclusive_writer_and_corruption_fail_closed() {
    let dir = Fixture::new();
    {
        let _g = Governor::open(&dir.0, limits()).unwrap();
        assert!(Governor::open(&dir.0, limits()).is_err());
    }
    fs::write(dir.0.join("usage.log"), "R 1 I 20\nS 1").unwrap();
    assert!(matches!(Governor::open(&dir.0, limits()), Err(Error::Corrupt)));
}

#[test]
fn stale_lock_does_not_allow_a_fresh_ledger() {
    let dir = Fixture::new();
    fs::write(dir.0.join("writer.lock"), "").unwrap();
    assert!(Governor::open(&dir.0, limits()).is_err());
}

#[test]
fn serialized_concurrent_admission_cannot_overspend() {
    let dir = Fixture::new();
    let g = std::sync::Arc::new(std::sync::Mutex::new(Governor::open(&dir.0, limits()).unwrap()));
    let threads: Vec<_> = (0..8)
        .map(|_| {
            let g = g.clone();
            std::thread::spawn(move || g.lock().unwrap().reserve(Priority::Interactive, 25).is_ok())
        })
        .collect();
    let accepted: usize = threads.into_iter().map(|t| usize::from(t.join().unwrap())).sum();
    assert_eq!(accepted, 4);
    assert_eq!(g.lock().unwrap().charged(), 100);
}

#[test]
fn reservation_cannot_settle_another_ledger_or_reopened_instance() {
    let a = Fixture::new();
    let b = Fixture::new();
    let mut ga = Governor::open(&a.0, limits()).unwrap();
    let mut gb = Governor::open(&b.0, limits()).unwrap();
    let ra = ga.reserve(Priority::Interactive, 40).unwrap();
    let rb = gb.reserve(Priority::Interactive, 50).unwrap();
    assert!(matches!(gb.settle(ra, 0), Err(Error::Unknown)));
    assert_eq!(gb.charged(), 50);
    gb.settle(rb, 10).unwrap();
    drop(ga);
    let mut ga = Governor::open(&a.0, limits()).unwrap();
    assert!(matches!(ga.settle(ra, 0), Err(Error::Unknown)));
    assert_eq!(ga.charged(), 40);
}
