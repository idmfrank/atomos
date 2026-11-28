//! Equal synthetic inputs through direct extended replay and bounded session.
use atomos_core::{
    atomic_admission::{
        AtomicReplay, CompiledRequest, Configuration, Context, Error, ReplayOutcome,
    },
    authorization::{Envelope, Policy},
    broker::{Request, Usage},
    governor::{Limits, Priority},
    session::{replay::Parameters, Error as SessionError, Event, Session},
};
use std::{
    cell::Cell,
    fs,
    io::{Cursor, Read},
    os::unix::fs::DirBuilderExt,
    path::Path,
    rc::Rc,
    time::{Duration, Instant},
};
const TEXT: &[u8] = include_bytes!("../tests/fixtures/responses-text.sse");
const REFUSAL: &[u8] = include_bytes!("../tests/fixtures/responses-refusal.sse");
const USAGE: Usage = Usage {
    input: 35,
    output: 12,
    cached_input: 10,
    reasoning_output: 4,
};
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
fn parameters(id: u64, background: bool) -> Parameters {
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
        priority: if background {
            Priority::Background
        } else {
            Priority::Interactive
        },
        max_output: 64,
    }
}
fn stats(name: &str, mut values: Vec<Duration>) {
    values.sort();
    let n = values.len();
    if n == 0 {
        println!("metric={name} samples=0 read_boundary=not_reached");
        return;
    }
    let p = |q: usize| values[(n * q).div_ceil(100).saturating_sub(1)].as_secs_f64() * 1000.;
    println!(
        "metric={name} samples={n} p50_ms={:.6} p95_ms={:.6} p99_ms={:.6}",
        p(50),
        p(95),
        p(99)
    );
}
struct Boundary<'a> {
    cursor: Cursor<&'a [u8]>,
    start: Instant,
    first: Rc<Cell<Option<Duration>>>,
    trace: bool,
}
impl Read for Boundary<'_> {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        if self.first.get().is_none() {
            self.first.set(Some(self.start.elapsed()));
            if self.trace {
                println!("TRACE_READ");
            }
        }
        self.cursor.read(out)
    }
}
fn bytes(p: &Path) -> u64 {
    fs::read_dir(p)
        .unwrap()
        .map(|e| {
            let p = e.unwrap().path();
            if p.is_dir() {
                bytes(&p)
            } else if p.file_name().unwrap() == "writer.lock" {
                0
            } else {
                fs::metadata(p).unwrap().len()
            }
        })
        .sum()
}
fn failure(known: bool) -> Vec<u8> {
    let text = std::str::from_utf8(TEXT).unwrap().replace("\r\n", "\n");
    let created = text.split("\n\n").next().unwrap();
    let usage = if known {
        r#"{"input_tokens":35,"output_tokens":12,"total_tokens":47,"input_tokens_details":{"cached_tokens":10},"output_tokens_details":{"reasoning_tokens":4}}"#
    } else {
        "null"
    };
    format!("{created}\n\nevent: response.failed\ndata: {{\"type\":\"response.failed\",\"sequence_number\":1,\"response\":{{\"id\":\"resp_synthetic\",\"object\":\"response\",\"status\":\"failed\",\"error\":{{\"code\":\"server_error\"}},\"incomplete_details\":null,\"usage\":{usage}}}}}\n\n").into_bytes()
}
fn check(event: &Event, kind: &str, turn: u64) {
    match (kind, event) {
        ("text", Event::Replay(Ok(ReplayOutcome::Completed(a)))) => {
            assert_eq!(a.text, "Café 🚀\nReady.");
            assert_eq!(a.turn, turn);
            assert_eq!(a.usage, Some(USAGE));
            assert!(a.simulated);
        }
        ("refusal", Event::Replay(Ok(ReplayOutcome::Refused { turn: t, usage }))) => {
            assert_eq!(*t, turn);
            assert_eq!(*usage, USAGE);
        }
        (
            "failed-known" | "failed-unknown",
            Event::Replay(Ok(ReplayOutcome::Failed {
                turn: t,
                failure,
                usage,
            })),
        ) => {
            assert_eq!(*t, turn);
            assert_eq!(*failure, atomos_core::broker::Failure::Unavailable);
            assert_eq!(
                *usage,
                if kind == "failed-known" {
                    Some(USAGE)
                } else {
                    None
                }
            );
        }
        _ => panic!("unexpected synthetic outcome"),
    }
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let mode = args[1].as_str();
    assert!(matches!(mode, "direct" | "session"));
    let kind = args[2].as_str();
    let root = Path::new(&args[3]);
    let samples: u64 = args[4].parse().unwrap();
    assert!((1..=1000).contains(&samples));
    let trace = args.get(5).map_or(false, |v| v == "trace");
    let wire = match kind {
        "text" | "cancelled" => TEXT.to_vec(),
        "refusal" => REFUSAL.to_vec(),
        "failed-known" => failure(true),
        "failed-unknown" => failure(false),
        _ => panic!("unknown workload"),
    };
    let isolated = matches!(kind, "failed-unknown" | "cancelled");
    let normal = root.join("state");
    fs::DirBuilder::new().mode(0o700).create(&normal).unwrap();
    let mut broker = Some(AtomicReplay::initialize(&normal, config()).unwrap());
    let initial = bytes(root);
    let mut s = Session::default();
    let (mut admission, mut complete, mut restart) = (Vec::new(), Vec::new(), Vec::new());
    let mut total = 0;
    let mut background = 0;
    let mut last = normal.clone();
    for id in 1..=samples {
        if isolated {
            drop(broker.take());
            last = root.join(format!("isolated-{id}"));
            fs::DirBuilder::new().mode(0o700).create(&last).unwrap();
            broker = Some(AtomicReplay::initialize(&last, config()).unwrap());
        }
        let b = broker.as_mut().unwrap();
        let p = parameters(if isolated { 1 } else { id }, id % 2 == 0);
        let first = Rc::new(Cell::new(None));
        if trace {
            println!("TRACE_START");
        }
        let start = Instant::now();
        let source = Boundary {
            cursor: Cursor::new(wire.as_slice()),
            start,
            first: first.clone(),
            trace,
        };
        if mode == "session" {
            let result = s.replay(
                b,
                "question",
                p,
                source,
                || kind == "cancelled",
                |e| {
                    check(e, kind, id);
                    Ok(())
                },
            );
            if kind == "cancelled" {
                assert_eq!(result, Err(SessionError::Stale));
            } else {
                result.unwrap();
            }
        } else {
            let r = CompiledRequest::compile(
                p.envelope,
                7,
                1,
                Request {
                    turn: id,
                    task: 5,
                    text: "question",
                    priority: p.priority,
                    max_output: 64,
                },
            )
            .unwrap();
            let result = b.run_sse_extended(
                &r,
                Context {
                    now: 200,
                    active_turn: id,
                    task: 5,
                },
                source,
                || kind == "cancelled",
            );
            if kind == "cancelled" {
                assert_eq!(result, Err(Error::Cancelled));
            } else {
                check(&Event::Replay(result), kind, id);
            }
        }
        complete.push(start.elapsed());
        if let Some(t) = first.get() {
            admission.push(t);
        }
        let actual = if isolated { 200 } else { 47 };
        total += actual;
        if id % 2 == 0 {
            background += actual;
        }
        assert_eq!(b.charged(), if isolated { actual } else { total });
        assert_eq!(
            b.background_charged(),
            if isolated {
                if id % 2 == 0 {
                    actual
                } else {
                    0
                }
            } else {
                background
            }
        );
        assert_eq!(
            b.persistence_counts(),
            if isolated { (1, 1) } else { (id * 2, id * 2) }
        );
        assert_eq!(b.suspended(), isolated);
        assert_eq!(b.next_request_id(), if isolated { 2 } else { id + 1 });
        if trace {
            println!("TRACE_END");
        }
    }
    let expected_history = if mode == "session" && kind == "text" {
        samples.min(4) as usize
    } else {
        0
    };
    assert_eq!(s.history_pairs(), expected_history);
    assert_eq!(s.history_bytes(), expected_history * 25);
    drop(broker);
    for _ in 0..30 {
        let start = Instant::now();
        let b = AtomicReplay::open(&last, config()).unwrap();
        restart.push(start.elapsed());
        assert_eq!(b.charged(), if isolated { 200 } else { total });
        assert_eq!(b.suspended(), isolated);
    }
    println!("mode={mode} kind={kind} samples={samples} completed={} refused={} failed={} cancelled={} unknown={} unexpected_errors=0 mismatches=0 real_calls=0 total_fixture_charge={total} background_fixture_charge={background} retained_pairs={} retained_bytes={} journal_bytes={} bytes_per_request={:.3}",if kind=="text"{samples}else{0},if kind=="refusal"{samples}else{0},if kind.starts_with("failed"){samples}else{0},if kind=="cancelled"{samples}else{0},if isolated{samples}else{0},s.history_pairs(),s.history_bytes(),bytes(root)-initial,(bytes(root)-initial)as f64/samples as f64);
    stats("admission_at_first_read", admission);
    stats("full_replay_and_session_work", complete);
    stats("restart_replay", restart);
    let mut eviction = Vec::new();
    let mut clear = Vec::new();
    let mut h = Session::default();
    for _ in 0..samples {
        for _ in 0..4 {
            h.mock("question", |_| Ok(())).unwrap();
        }
        let t = Instant::now();
        h.mock("question", |_| Ok(())).unwrap();
        eviction.push(t.elapsed());
        assert_eq!(h.history_pairs(), 4);
        let t = Instant::now();
        h.clear();
        clear.push(t.elapsed());
        assert_eq!(h.history_bytes(), 0);
    }
    stats("bounded_history_eviction", eviction);
    stats("bounded_history_clear", clear);
}
