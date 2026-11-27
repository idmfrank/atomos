#![cfg(all(unix, feature = "atomic-admission"))]
use atomos_core::{
    atomic_admission::{
        AtomicReplay, CompiledRequest, Configuration, Context, Error, ReplayOutcome, NAMESPACE,
    },
    authorization::{Envelope, Policy},
    broker::{Failure, Request, Usage},
    governor::{Limits, Priority},
};
use std::{
    fs,
    io::Read,
    os::unix::fs::DirBuilderExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "atomos-outcomes-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&p).unwrap();
        Self(p)
    }
    fn journal(&self) -> PathBuf {
        self.0.join(NAMESPACE).join("admission.log")
    }
}
impl Drop for Fixture {
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
const SSE: &[u8] = include_bytes!("fixtures/responses-text.sse");
struct NeverRead;
impl Read for NeverRead {
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        panic!("denied/local admission read source")
    }
}

const AUX: &[u8] = include_bytes!("fixtures/responses-compat.sse");
const REASON: &[u8] = include_bytes!("fixtures/responses-reasoning.sse");
const REFUSAL: &[u8] = include_bytes!("fixtures/responses-refusal.sse");
const U: Usage = Usage {
    input: 35,
    output: 12,
    cached_input: 10,
    reasoning_output: 4,
};
fn expected_text() -> &'static str {
    "Café 🚀\nReady."
}
fn failed(kind: &str, code: &str, known: bool) -> Vec<u8> {
    let fixture = std::str::from_utf8(SSE).unwrap().replace("\r\n", "\n");
    let created = fixture.split("\n\n").next().unwrap();
    let usage = if known {
        serde_json::json!({"input_tokens":35,"output_tokens":12,"total_tokens":47,
        "input_tokens_details":{"cached_tokens":10},"output_tokens_details":{"reasoning_tokens":4}})
    } else {
        serde_json::Value::Null
    };
    let event = serde_json::json!({"type":format!("response.{kind}"),"sequence_number":1,
      "response":{"id":"resp_synthetic","object":"response","status":kind,
      "error":{"code":code},"incomplete_details":null,"usage":usage}});
    format!("{created}\n\nevent: response.{kind}\ndata: {event}\n\n").into_bytes()
}
struct Chunks<'a> {
    bytes: &'a [u8],
    size: usize,
}
impl Read for Chunks<'_> {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        let n = self.bytes.len().min(self.size).min(out.len());
        out[..n].copy_from_slice(&self.bytes[..n]);
        self.bytes = &self.bytes[n..];
        Ok(n)
    }
}
#[test]
fn ordinary_wire_is_exactly_equivalent_at_all_fragment_sizes() {
    for size in [1, 7, 64, 4096] {
        let f = Fixture::new();
        let g = Fixture::new();
        let mut strict = AtomicReplay::initialize(&f.0, config()).unwrap();
        let mut ext = AtomicReplay::initialize(&g.0, config()).unwrap();
        for id in 1..=12 {
            let r = CompiledRequest::compile(
                env(id),
                7,
                1,
                Request {
                    priority: if id % 2 == 0 {
                        Priority::Background
                    } else {
                        Priority::Interactive
                    },
                    ..req()
                },
            )
            .unwrap();
            let a = strict
                .run_sse(&r, context(), Chunks { bytes: SSE, size }, || false)
                .unwrap();
            let b = ext
                .run_sse_extended(&r, context(), Chunks { bytes: SSE, size }, || false)
                .unwrap();
            assert_eq!(a.text, expected_text());
            assert_eq!(a.usage, Some(U));
            assert_eq!(b, ReplayOutcome::Completed(a));
            assert_eq!(ext.charged(), id * 47);
            assert_eq!(ext.background_charged(), (id / 2) * 47);
            assert_eq!(
                fs::read(f.journal()).unwrap(),
                fs::read(g.journal()).unwrap()
            );
        }
        assert_eq!(ext.persistence_counts(), (24, 24));
    }
}
#[test]
fn extended_profiles_are_explicit_and_do_not_release_hidden_content() {
    for fixture in [AUX, REASON, REFUSAL] {
        for size in [1, 7, 64, 4096] {
            let f = Fixture::new();
            let mut b = AtomicReplay::initialize(&f.0, config()).unwrap();
            let outcome = b
                .run_sse_extended(
                    &compiled(1),
                    context(),
                    Chunks {
                        bytes: fixture,
                        size,
                    },
                    || false,
                )
                .unwrap();
            if fixture == REFUSAL {
                assert_eq!(outcome, ReplayOutcome::Refused { turn: 4, usage: U });
            } else {
                match &outcome {
                    ReplayOutcome::Completed(a) => {
                        assert_eq!(a.text, expected_text());
                        assert_eq!(a.usage, Some(U));
                    }
                    _ => panic!("wrong outcome"),
                }
            }
            let visible = format!("{outcome:?}{}", fs::read_to_string(f.journal()).unwrap());
            for secret in [
                "Fixture refusal",
                "synthetic_final",
                "encrypted_content",
                "response.created",
            ] {
                assert!(!visible.contains(secret));
            }
            assert_eq!(b.charged(), 47);
            assert!(!b.suspended());
            assert_eq!(b.dispatches(), 1);
            let g = Fixture::new();
            let mut strict = AtomicReplay::initialize(&g.0, config()).unwrap();
            assert!(strict
                .run_sse(
                    &compiled(1),
                    context(),
                    Chunks {
                        bytes: fixture,
                        size
                    },
                    || false
                )
                .is_err());
            assert_eq!(strict.charged(), 200);
            assert!(strict.suspended());
        }
    }
}
#[test]
fn settled_refusal_survives_restart_without_replay_or_fallback() {
    let f = Fixture::new();
    let mut b = AtomicReplay::initialize(&f.0, config()).unwrap();
    assert_eq!(
        b.run_sse_extended(&compiled(1), context(), REFUSAL, || false)
            .unwrap(),
        ReplayOutcome::Refused { turn: 4, usage: U }
    );
    assert_eq!(b.persistence_counts(), (2, 2));
    drop(b);
    let mut b = AtomicReplay::open(&f.0, config()).unwrap();
    assert_eq!(b.charged(), 47);
    assert!(!b.suspended());
    assert_eq!(
        b.run_sse_extended(&compiled(1), context(), NeverRead, || false),
        Err(Error::Replay)
    );
    assert!(matches!(
        b.run_sse_extended(&compiled(2), context(), SSE, || false),
        Ok(ReplayOutcome::Completed(_))
    ));
    assert_eq!(b.charged(), 94);
    assert_eq!(b.next_request_id(), 3);
    assert_eq!(b.dispatches(), 1);
}
#[test]
fn failures_and_incomplete_distinguish_known_from_unknown_usage() {
    for kind in ["failed", "incomplete"] {
        for known in [false, true] {
            let f = Fixture::new();
            let mut b = AtomicReplay::initialize(&f.0, config()).unwrap();
            let wire = failed(kind, "server_error", known);
            let failure = if kind == "incomplete" {
                Failure::Incomplete
            } else {
                Failure::Unavailable
            };
            assert_eq!(
                b.run_sse_extended(&compiled(1), context(), wire.as_slice(), || false)
                    .unwrap(),
                ReplayOutcome::Failed {
                    turn: 4,
                    failure,
                    usage: if known { Some(U) } else { None }
                }
            );
            assert_eq!(b.charged(), if known { 47 } else { 200 });
            assert_eq!(b.suspended(), !known);
            drop(b);
            let mut b = AtomicReplay::open(&f.0, config()).unwrap();
            assert_eq!(b.suspended(), !known);
            if !known {
                assert_eq!(
                    b.run_sse_extended(&compiled(2), context(), NeverRead, || false),
                    Err(Error::Suspended)
                );
            }
        }
    }
}
#[test]
fn provider_failures_halt_with_and_without_usage() {
    for (code, failure) in [
        ("rate_limit_exceeded", Failure::RateLimited),
        ("invalid_api_key", Failure::Authorization),
    ] {
        for known in [false, true] {
            let f = Fixture::new();
            let mut b = AtomicReplay::initialize(&f.0, config()).unwrap();
            let wire = failed("failed", code, known);
            assert_eq!(
                b.run_sse_extended(&compiled(1), context(), wire.as_slice(), || false)
                    .unwrap(),
                ReplayOutcome::Failed {
                    turn: 4,
                    failure,
                    usage: if known { Some(U) } else { None }
                }
            );
            assert_eq!(b.charged(), if known { 47 } else { 200 });
            assert!(b.suspended());
            drop(b);
            let mut b = AtomicReplay::open(&f.0, config()).unwrap();
            assert!(b.suspended());
            assert_eq!(
                b.run_sse_extended(&compiled(2), context(), NeverRead, || false),
                Err(Error::Suspended)
            );
        }
    }
}
#[test]
fn component_and_total_overruns_settle_actual_and_halt() {
    for fixture in [SSE, REFUSAL] {
        for (input, output, max_output) in [(240, 12, 64), (137, 12, 64), (35, 65, 64)] {
            let wire = std::str::from_utf8(fixture)
                .unwrap()
                .replace("\"input_tokens\":35", &format!("\"input_tokens\":{input}"))
                .replace(
                    "\"output_tokens\":12",
                    &format!("\"output_tokens\":{output}"),
                )
                .replace(
                    "\"total_tokens\":47",
                    &format!("\"total_tokens\":{}", input + output),
                );
            let f = Fixture::new();
            let mut b = AtomicReplay::initialize(&f.0, config()).unwrap();
            let r = CompiledRequest::compile(
                env(1),
                7,
                1,
                Request {
                    max_output,
                    ..req()
                },
            )
            .unwrap();
            assert_eq!(
                b.run_sse_extended(&r, context(), wire.as_bytes(), || false),
                Err(Error::Overrun)
            );
            assert_eq!(b.charged(), input + output);
            drop(b);
            assert!(AtomicReplay::open(&f.0, config()).unwrap().suspended());
        }
    }
}
#[test]
fn invalid_usage_never_refunds_a_reservation() {
    for (from, to) in [
        ("\"cached_tokens\":10", "\"cached_tokens\":36"),
        ("\"reasoning_tokens\":4", "\"reasoning_tokens\":13"),
        ("\"total_tokens\":47", "\"total_tokens\":46"),
    ] {
        let wire = std::str::from_utf8(REFUSAL).unwrap().replace(from, to);
        assert_ne!(wire.as_bytes(), REFUSAL);
        let f = Fixture::new();
        let mut b = AtomicReplay::initialize(&f.0, config()).unwrap();
        assert!(b
            .run_sse_extended(&compiled(1), context(), wire.as_bytes(), || false)
            .is_err());
        assert_eq!(b.charged(), 200);
        assert!(b.suspended());
        assert_eq!(b.persistence_counts(), (1, 1));
    }
}
#[test]
fn denied_extended_admission_never_reads_or_consumes_identity() {
    let f = Fixture::new();
    let mut b = AtomicReplay::initialize(&f.0, config()).unwrap();
    for (e, m, v, c, error) in [
        (
            Envelope {
                account: 9,
                ..env(1)
            },
            7,
            1,
            context(),
            Error::Scope,
        ),
        (
            Envelope { host: 9, ..env(1) },
            7,
            1,
            context(),
            Error::Scope,
        ),
        (
            Envelope { grant: 9, ..env(1) },
            7,
            1,
            context(),
            Error::Scope,
        ),
        (env(1), 8, 1, context(), Error::Scope),
        (env(1), 7, 2, context(), Error::Scope),
        (
            env(1),
            7,
            1,
            Context {
                task: 6,
                ..context()
            },
            Error::StaleTask,
        ),
        (
            env(1),
            7,
            1,
            Context {
                active_turn: 6,
                ..context()
            },
            Error::StaleTurn,
        ),
        (
            env(1),
            7,
            1,
            Context {
                now: 230,
                ..context()
            },
            Error::Expired,
        ),
        (
            env(1),
            7,
            1,
            Context {
                now: 199,
                ..context()
            },
            Error::ClockRollback,
        ),
    ] {
        let r = CompiledRequest::compile(e, m, v, req()).unwrap();
        assert_eq!(b.run_sse_extended(&r, c, NeverRead, || false), Err(error));
    }
    assert_eq!(b.charged(), 0);
    assert_eq!(b.next_request_id(), 1);
    assert_eq!(b.dispatches(), 0);
    assert_eq!(b.persistence_counts(), (0, 0));
}
#[test]
fn budget_and_evidence_denial_remain_read_free() {
    for mode in 0..3 {
        let f = Fixture::new();
        let mut c = config();
        if mode == 0 {
            c.model_eligible = false;
        }
        if mode == 1 {
            c.policy.limits.per_request = 199;
        }
        if mode == 2 {
            c.policy.limits.background = 199;
        }
        let mut b = AtomicReplay::initialize(&f.0, c).unwrap();
        let r = CompiledRequest::compile(
            env(1),
            7,
            1,
            Request {
                priority: Priority::Background,
                ..req()
            },
        )
        .unwrap();
        let expected = if mode == 0 {
            Error::Evidence
        } else {
            Error::Budget
        };
        assert_eq!(
            b.run_sse_extended(&r, context(), NeverRead, || false),
            Err(expected)
        );
        assert_eq!(b.charged(), 0);
    }
}
#[test]
fn local_answers_remain_free_while_revoked() {
    let f = Fixture::new();
    let mut b = AtomicReplay::initialize(&f.0, config()).unwrap();
    b.revoke().unwrap();
    let r = CompiledRequest::compile(
        env(1),
        99,
        99,
        Request {
            text: "what is your name?",
            ..req()
        },
    )
    .unwrap();
    match b
        .run_sse_extended(&r, context(), NeverRead, || true)
        .unwrap()
    {
        ReplayOutcome::Local(a) => {
            assert_eq!(a.text, "I am AtomOS.");
            assert!(!a.simulated);
            assert_eq!(a.usage, None);
        }
        _ => panic!("not local"),
    }
    assert_eq!(b.charged(), 0);
    assert_eq!(b.dispatches(), 0);
    assert_eq!(
        b.run_sse_extended(
            &r,
            Context {
                task: 7,
                ..context()
            },
            NeverRead,
            || false
        ),
        Err(Error::StaleTask)
    );
}
#[test]
fn cancellation_boundaries_retain_the_admitted_charge() {
    use std::{cell::Cell, rc::Rc};
    for stop in [0usize, 1, 2, usize::MAX - 1, usize::MAX] {
        let f = Fixture::new();
        let mut b = AtomicReplay::initialize(&f.0, config()).unwrap();
        let calls = Cell::new(0usize);
        let eof = Rc::new(Cell::new(0u8));
        struct Reader<'a> {
            data: &'a [u8],
            eof: Rc<Cell<u8>>,
        }
        impl Read for Reader<'_> {
            fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
                let n = self.data.len().min(64).min(out.len());
                out[..n].copy_from_slice(&self.data[..n]);
                self.data = &self.data[n..];
                if n == 0 {
                    self.eof.set(2);
                } else if self.data.is_empty() {
                    self.eof.set(1);
                }
                Ok(n)
            }
        }
        let flag = eof.clone();
        let result = b.run_sse_extended(
            &compiled(1),
            context(),
            Reader { data: REFUSAL, eof },
            || {
                let n = calls.get();
                calls.set(n + 1);
                if stop == usize::MAX {
                    flag.get() == 2
                } else if stop == usize::MAX - 1 {
                    flag.get() >= 1
                } else {
                    n >= stop
                }
            },
        );
        assert_eq!(result, Err(Error::Cancelled));
        assert_eq!(b.charged(), 200);
        assert_eq!(b.persistence_counts(), (1, 1));
        drop(b);
        assert!(AtomicReplay::open(&f.0, config()).unwrap().suspended());
    }
}
#[test]
fn terminal_before_eof_or_io_error_does_not_settle() {
    for interrupted in [false, true] {
        struct BadEof {
            sent: bool,
            interrupted: bool,
        }
        impl Read for BadEof {
            fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
                if !self.sent {
                    self.sent = true;
                    b[..REFUSAL.len()].copy_from_slice(REFUSAL);
                    Ok(REFUSAL.len())
                } else {
                    Err(std::io::Error::from(if self.interrupted {
                        std::io::ErrorKind::Interrupted
                    } else {
                        std::io::ErrorKind::Other
                    }))
                }
            }
        }
        let f = Fixture::new();
        let mut b = AtomicReplay::initialize(&f.0, config()).unwrap();
        assert_eq!(
            b.run_sse_extended(
                &compiled(1),
                context(),
                BadEof {
                    sent: false,
                    interrupted
                },
                || false
            ),
            Err(if interrupted {
                Error::Cancelled
            } else {
                Error::UnknownUsage
            })
        );
        assert_eq!(b.charged(), 200);
        assert!(b.suspended());
    }
}
#[test]
fn malformed_or_capability_bearing_streams_remain_pending() {
    let source = std::str::from_utf8(REFUSAL).unwrap();
    let cases = [
        Vec::new(),
        REFUSAL[..REFUSAL.len() - 1].to_vec(),
        [REFUSAL, REFUSAL].concat(),
        source
            .replace("response.refusal.delta", "response.future_event")
            .into_bytes(),
        source
            .replace("\"sequence_number\":1", "\"sequence_number\":9")
            .into_bytes(),
    ];
    for wire in cases {
        let f = Fixture::new();
        let mut b = AtomicReplay::initialize(&f.0, config()).unwrap();
        assert!(b
            .run_sse_extended(&compiled(1), context(), wire.as_slice(), || false)
            .is_err());
        assert_eq!(b.charged(), 200);
        assert!(b.suspended());
        assert_eq!(b.persistence_counts(), (1, 1));
    }
}
#[test]
fn actual_child_death_keeps_writer_lock_and_durable_admission() {
    use std::{
        process::{Command, Stdio},
        thread,
        time::{Duration, Instant},
    };
    let f = Fixture::new();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "crash_child_fixture", "--ignored", "--nocapture"])
        .env("ATOMOS_SYNTHETIC_CRASH_ROOT", &f.0)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !f.0.join("ready").exists() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
    let ready = f.0.join("ready").exists();
    child.kill().unwrap();
    let status = child.wait().unwrap();
    assert!(!status.success());
    assert!(ready, "child did not reach durable A");
    assert!(f.0.join(NAMESPACE).join("writer.lock").exists());
    assert!(matches!(
        AtomicReplay::open(&f.0, config()),
        Err(Error::Storage)
    ));
    let log = fs::read_to_string(f.journal()).unwrap();
    assert_eq!(log.lines().count(), 2);
    assert!(log.ends_with(" 200\n"));
}
#[test]
#[ignore = "bounded subprocess fixture, not a standalone regression"]
fn crash_child_fixture() {
    let Some(root) = std::env::var_os("ATOMOS_SYNTHETIC_CRASH_ROOT") else {
        return;
    };
    struct Block(PathBuf);
    impl Read for Block {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            fs::write(self.0.join("ready"), b"ready").unwrap();
            loop {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }
    let root = PathBuf::from(root);
    let mut b = AtomicReplay::initialize(&root, config()).unwrap();
    let _ = b.run_sse_extended(&compiled(1), context(), Block(root), || false);
}

#[test]
fn failure_after_refusal_or_reasoning_never_exposes_partial_content() {
    for (fixture, marker) in [
        (REFUSAL, "response.refusal.delta"),
        (REASON, "response.output_item.done"),
    ] {
        for known in [false, true] {
            let text = std::str::from_utf8(fixture).unwrap().replace("\r\n", "\n");
            let mut prefix = String::new();
            let mut seq = 0;
            for frame in text.split("\n\n").filter(|s| !s.is_empty()) {
                prefix.push_str(frame);
                prefix.push_str("\n\n");
                let data = frame
                    .lines()
                    .find_map(|l| l.strip_prefix("data: "))
                    .unwrap();
                let v: serde_json::Value = serde_json::from_str(data).unwrap();
                seq = v["sequence_number"].as_u64().unwrap() + 1;
                if v["type"] == marker {
                    break;
                }
            }
            let usage = if known {
                serde_json::json!({"input_tokens":35,"output_tokens":12,"total_tokens":47,
    "input_tokens_details":{"cached_tokens":10},"output_tokens_details":{"reasoning_tokens":4}})
            } else {
                serde_json::Value::Null
            };
            let v = serde_json::json!({"type":"response.incomplete","sequence_number":seq,
    "response":{"id":"resp_synthetic","object":"response","status":"incomplete","usage":usage}});
            prefix.push_str(&format!("event: response.incomplete\ndata: {v}\n\n"));
            let f = Fixture::new();
            let mut b = AtomicReplay::initialize(&f.0, config()).unwrap();
            let result = b
                .run_sse_extended(&compiled(1), context(), prefix.as_bytes(), || false)
                .unwrap();
            assert_eq!(
                result,
                ReplayOutcome::Failed {
                    turn: 4,
                    failure: Failure::Incomplete,
                    usage: if known { Some(U) } else { None }
                }
            );
            assert_eq!(b.charged(), if known { 47 } else { 200 });
            assert_eq!(b.suspended(), !known);
            let visible = format!("{result:?}{}", fs::read_to_string(f.journal()).unwrap());
            assert!(!visible.contains("Fixture refusal"));
            assert!(!visible.contains("synthetic_final"));
        }
    }
}
#[test]
fn decoder_and_usage_limits_never_publish_partial_settlement() {
    let huge = std::str::from_utf8(REFUSAL)
        .unwrap()
        .replace("\"input_tokens\":35", "\"input_tokens\":1000000000")
        .replace("\"total_tokens\":47", "\"total_tokens\":1000000012")
        .into_bytes();
    let cases = [
        huge,
        vec![b':'; 32 * 1024 + 1],
        [REFUSAL, &vec![b'\n'; 256 * 1024]].concat(),
    ];
    for wire in cases {
        let f = Fixture::new();
        let mut b = AtomicReplay::initialize(&f.0, config()).unwrap();
        assert!(b
            .run_sse_extended(&compiled(1), context(), wire.as_slice(), || false)
            .is_err());
        assert_eq!(b.charged(), 200);
        assert!(b.suspended());
        assert_eq!(b.persistence_counts(), (1, 1));
    }
}
