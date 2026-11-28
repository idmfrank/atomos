#![cfg(all(unix, feature = "atomic-admission"))]
use atomos_core::{
    atomic_admission::{
        AtomicReplay, Configuration, Error as ReplayError, ReplayOutcome, NAMESPACE,
    },
    authorization::{Envelope, Policy},
    broker::{Failure, Usage},
    governor::{Limits, Priority},
    session::{replay::Parameters, Error, Event, Session},
};
use std::{
    fs,
    io::{self, Read},
    os::unix::fs::DirBuilderExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "atomos-session-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&p).unwrap();
        Self(p)
    }
    fn log(&self) -> String {
        fs::read_to_string(self.0.join(NAMESPACE).join("admission.log")).unwrap()
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
            limits: Limits {
                total: 1_000_000,
                background: 100_000,
                interactive_reserve: 300_000,
                per_request: 1500,
            },
        },
    }
}
fn params(id: u64) -> Parameters {
    Parameters {
        envelope: Envelope {
            account: 1,
            host: 2,
            grant: 3,
            request_id: id,
            now: 200,
            expires_at: 230,
        },
        model: 7,
        version: 1,
        task: 5,
        now: 200,
        priority: Priority::Interactive,
        max_output: 64,
    }
}
const SSE: &[u8] = include_bytes!("fixtures/responses-text.sse");
const REFUSAL: &[u8] = include_bytes!("fixtures/responses-refusal.sse");
const U: Usage = Usage {
    input: 35,
    output: 12,
    cached_input: 10,
    reasoning_output: 4,
};
fn failed(known: bool, code: &str) -> Vec<u8> {
    let t = std::str::from_utf8(SSE).unwrap().replace("\r\n", "\n");
    let created = t.split("\n\n").next().unwrap();
    let usage = if known {
        r#"{"input_tokens":35,"output_tokens":12,"total_tokens":47,"input_tokens_details":{"cached_tokens":10},"output_tokens_details":{"reasoning_tokens":4}}"#
    } else {
        "null"
    };
    format!("{created}\n\nevent: response.failed\ndata: {{\"type\":\"response.failed\",\"sequence_number\":1,\"response\":{{\"id\":\"resp_synthetic\",\"object\":\"response\",\"status\":\"failed\",\"error\":{{\"code\":\"{code}\"}},\"incomplete_details\":null,\"usage\":{usage}}}}}\n\n").into_bytes()
}
#[test]
fn session_replay_completed_and_refused_have_independent_accounting_and_history() {
    let root = Root::new();
    let mut b = AtomicReplay::initialize(&root.0, config()).unwrap();
    let mut s = Session::default();
    s.replay(
        &mut b,
        "question",
        params(1),
        SSE,
        || false,
        |event| {
            match event {
                Event::Replay(Ok(ReplayOutcome::Completed(a))) => {
                    assert_eq!(a.text, "Café 🚀\nReady.");
                    assert_eq!(a.usage, Some(U));
                    assert_eq!(a.turn, 1);
                    assert!(a.simulated);
                }
                _ => panic!("wrong event"),
            }
            assert!(root.log().ends_with("S 1 47 0\n"));
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(s.history_pairs(), 1);
    assert_eq!(s.history_bytes(), 8 + "Café 🚀\nReady.".len());
    s.replay(
        &mut b,
        "question",
        params(2),
        REFUSAL,
        || false,
        |event| {
            assert_eq!(
                event,
                &Event::Replay(Ok(ReplayOutcome::Refused { turn: 2, usage: U }))
            );
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(s.history_pairs(), 1);
    assert_eq!(b.charged(), 94);
    assert_eq!(b.status().pending_units, None);
    assert!(!b.status().halted);
    assert_eq!(b.dispatches(), 2);
    s.clear();
    assert_eq!(s.history_bytes(), 0);
    assert_eq!(b.charged(), 94);
}
#[test]
fn session_failed_known_unknown_and_rate_limit_are_not_answers() {
    for (known, code) in [
        (true, "server_error"),
        (false, "server_error"),
        (true, "rate_limit_exceeded"),
        (false, "rate_limit_exceeded"),
    ] {
        let root = Root::new();
        let mut b = AtomicReplay::initialize(&root.0, config()).unwrap();
        let mut s = Session::default();
        let wire = failed(known, code);
        s.replay(
            &mut b,
            "question",
            params(1),
            wire.as_slice(),
            || false,
            |event| {
                assert_eq!(
                    event,
                    &Event::Replay(Ok(ReplayOutcome::Failed {
                        turn: 1,
                        failure: if code == "server_error" {
                            Failure::Unavailable
                        } else {
                            Failure::RateLimited
                        },
                        usage: if known { Some(U) } else { None }
                    }))
                );
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(s.history_pairs(), 0);
        assert_eq!(b.charged(), if known { 47 } else { 200 });
        assert_eq!(
            b.status().pending_units,
            if known { None } else { Some(200) }
        );
        assert_eq!(b.status().halted, code != "server_error");
        s.local("help", "local".into(), |_| Ok(())).unwrap();
        assert_eq!(b.dispatches(), 1);
        drop(b);
        let b = AtomicReplay::open(&root.0, config()).unwrap();
        assert_eq!(b.status().halted, code != "server_error");
        assert_eq!(b.charged(), if known { 47 } else { 200 });
    }
}
#[test]
fn session_cancel_after_settlement_suppresses_output_without_refund() {
    let root = Root::new();
    let mut b = AtomicReplay::initialize(&root.0, config()).unwrap();
    let mut s = Session::default();
    assert_eq!(
        s.replay(
            &mut b,
            "question",
            params(1),
            SSE,
            || root.log().contains("\nS "),
            |_| panic!("stale text")
        ),
        Err(Error::Stale)
    );
    assert_eq!(b.charged(), 47);
    assert!(!b.suspended());
    assert_eq!(s.history_pairs(), 0);
    s.replay(&mut b, "next", params(2), SSE, || false, |_| Ok(()))
        .unwrap();
    assert_eq!(b.charged(), 94);
}
#[test]
fn session_cancellation_before_and_between_reads_keeps_reservation() {
    use std::{cell::Cell, rc::Rc};
    struct Fragment {
        pos: Rc<Cell<usize>>,
    }
    impl Read for Fragment {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            let p = self.pos.get();
            if p == SSE.len() {
                return Ok(0);
            }
            out[0] = SSE[p];
            self.pos.set(p + 1);
            Ok(1)
        }
    }
    for threshold in [0, 20, SSE.len()] {
        let root = Root::new();
        let mut b = AtomicReplay::initialize(&root.0, config()).unwrap();
        let mut s = Session::default();
        let pos = Rc::new(Cell::new(0));
        assert_eq!(
            s.replay(
                &mut b,
                "question",
                params(1),
                Fragment { pos: pos.clone() },
                || pos.get() >= threshold,
                |_| panic!("cancelled output")
            ),
            Err(Error::Stale)
        );
        assert_eq!(b.charged(), 200);
        assert_eq!(b.status().pending_units, Some(200));
        assert_eq!(pos.get(), threshold);
        assert_eq!(s.history_pairs(), 0);
    }
}
#[test]
fn session_broken_output_after_settlement_does_not_retain_or_retry() {
    let root = Root::new();
    let mut b = AtomicReplay::initialize(&root.0, config()).unwrap();
    let mut s = Session::default();
    assert_eq!(
        s.replay(
            &mut b,
            "question",
            params(1),
            SSE,
            || false,
            |_| Err(io::ErrorKind::BrokenPipe.into())
        ),
        Err(Error::Output)
    );
    assert_eq!(b.charged(), 47);
    assert_eq!(b.next_request_id(), 2);
    assert_eq!(b.dispatches(), 1);
    assert_eq!(s.history_pairs(), 0);
    drop(b);
    let b = AtomicReplay::open(&root.0, config()).unwrap();
    assert_eq!(b.charged(), 47);
}
#[test]
fn session_identity_denial_is_read_free_and_unknown_stream_is_not_history() {
    struct Never;
    impl Read for Never {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            panic!("denied read")
        }
    }
    let root = Root::new();
    let mut b = AtomicReplay::initialize(&root.0, config()).unwrap();
    let mut s = Session::default();
    let mut p = params(1);
    p.model = 8;
    s.replay(
        &mut b,
        "question",
        p,
        Never,
        || false,
        |e| {
            assert_eq!(e, &Event::Replay(Err(ReplayError::Scope)));
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(b.next_request_id(), 1);
    assert_eq!(b.dispatches(), 0);
    s.replay(
        &mut b,
        "question",
        params(1),
        &b"invalid"[..],
        || false,
        |e| {
            assert!(matches!(e, Event::Replay(Err(_))));
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(s.history_pairs(), 0);
    assert_eq!(b.charged(), 200);
    assert!(b.suspended());
}
