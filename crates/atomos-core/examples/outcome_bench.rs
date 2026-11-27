//! Isolated frozen-strict and extended outcome comparison; synthetic data only.
#![allow(unknown_lints)]
#![allow(unexpected_cfgs)]
#[cfg(atomos_extended_candidate)]
use atomos_core::atomic_admission::ReplayOutcome;
use atomos_core::{
    atomic_admission::{AtomicReplay, CompiledRequest, Configuration, Context},
    authorization::{Envelope, Policy},
    broker::{Request, Usage},
    governor::{Limits, Priority},
};
use std::{
    cell::Cell,
    fs,
    io::{Cursor, Read},
    path::Path,
    rc::Rc,
    time::{Duration, Instant},
};
const SSE: &[u8] = include_bytes!("../tests/fixtures/responses-text.sse");
const AUX: &[u8] = include_bytes!("../tests/fixtures/responses-compat.sse");
const REASON: &[u8] = include_bytes!("../tests/fixtures/responses-reasoning.sse");
const REFUSAL: &[u8] = include_bytes!("../tests/fixtures/responses-refusal.sse");
const EXPECTED: Usage = Usage {
    input: 35,
    output: 12,
    cached_input: 10,
    reasoning_output: 4,
};
fn policy() -> Policy {
    Policy {
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
fn req(id: u64) -> Request<'static> {
    Request {
        turn: 4,
        task: 5,
        text: "question",
        priority: if id % 2 == 0 {
            Priority::Background
        } else {
            Priority::Interactive
        },
        max_output: 64,
    }
}
fn stats(name: &str, mut data: Vec<Duration>) {
    data.sort();
    let n = data.len();
    let p = |v: usize| data[(n * v).div_ceil(100).saturating_sub(1)].as_secs_f64() * 1000.;
    println!(
        "metric={name} samples={n} p50_ms={:.6} p95_ms={:.6} p99_ms={:.6}",
        p(50),
        p(95),
        p(99)
    );
}
struct Boundary {
    input: Cursor<&'static [u8]>,
    start: Instant,
    time: Rc<Cell<Option<Duration>>>,
    trace: bool,
}
impl Read for Boundary {
    fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
        if self.time.get().is_none() {
            self.time.set(Some(self.start.elapsed()));
            if self.trace {
                println!("TRACE_READ");
            }
        }
        self.input.read(b)
    }
}
fn bytes(root: &Path) -> u64 {
    fs::read_dir(root)
        .unwrap()
        .map(|e| {
            let p = e.unwrap().path();
            if p.is_dir() {
                bytes(&p)
            } else if p.file_name().unwrap() != "writer.lock" {
                fs::metadata(p).unwrap().len()
            } else {
                0
            }
        })
        .sum()
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let case = args[1].as_str();
    let root = Path::new(&args[2]);
    let samples: u64 = args[3].parse().unwrap();
    assert!((1..=1000).contains(&samples));
    let trace = args.get(4).map_or(false, |s| s == "trace");
    let wire = match case {
        "strict" => SSE,
        "auxiliary" => AUX,
        "reasoning" => REASON,
        "refusal" => REFUSAL,
        _ => panic!("unsupported synthetic workload"),
    };
    #[cfg(not(atomos_extended_candidate))]
    assert_eq!(case, "strict");
    let c = Configuration {
        policy: policy(),
        version: 1,
        model: 7,
        model_eligible: true,
    };
    let mut b = AtomicReplay::initialize(root, c).unwrap();
    let initial = bytes(root);
    let (mut admission, mut completion, mut restart, mut validation, mut compilation) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut completed = 0u64;
    #[allow(unused_mut)]
    let mut refused = 0u64;
    for id in 1..=samples {
        let t = Instant::now();
        let r = CompiledRequest::compile(env(id), 7, 1, req(id)).unwrap();
        compilation.push(t.elapsed());
        let ctx = Context {
            now: 200,
            active_turn: 4,
            task: 5,
        };
        let t = Instant::now();
        b.validate(&r, ctx).unwrap();
        validation.push(t.elapsed());
        let measured = Rc::new(Cell::new(None));
        if trace {
            println!("TRACE_START");
        }
        let start = Instant::now();
        let source = Boundary {
            input: Cursor::new(wire),
            start,
            time: measured.clone(),
            trace,
        };
        #[cfg(atomos_extended_candidate)]
        let result = b.run_sse_extended(&r, ctx, source, || false);
        #[cfg(not(atomos_extended_candidate))]
        let result = b.run_sse(&r, ctx, source, || false);
        completion.push(start.elapsed());
        let outcome = result.expect("unexpected synthetic replay error");
        #[cfg(atomos_extended_candidate)]
        match outcome {
            ReplayOutcome::Completed(a) => {
                assert_ne!(case, "refusal");
                assert_eq!(a.text, "Café 🚀\nReady.");
                assert_eq!(a.usage, Some(EXPECTED));
                assert_eq!(a.turn, 4);
                assert!(a.simulated);
                completed += 1;
            }
            ReplayOutcome::Refused { turn, usage } => {
                assert_eq!(case, "refusal");
                assert_eq!(turn, 4);
                assert_eq!(usage, EXPECTED);
                refused += 1;
            }
            _ => panic!("unexpected outcome"),
        }
        #[cfg(not(atomos_extended_candidate))]
        {
            assert_eq!(outcome.text, "Café 🚀\nReady.");
            assert_eq!(outcome.usage, Some(EXPECTED));
            assert_eq!(outcome.turn, 4);
            assert!(outcome.simulated);
            completed += 1;
        }
        admission.push(measured.get().unwrap());
        assert_eq!(b.charged(), id * 47);
        assert_eq!(b.background_charged(), (id / 2) * 47);
        assert!(!b.suspended());
        assert_eq!(b.next_request_id(), id + 1);
        if trace {
            println!("TRACE_END");
        }
    }
    assert_eq!(completed + refused, samples);
    assert_eq!(b.persistence_counts(), (samples * 2, samples * 2));
    println!("case={case} samples={samples} completed={completed} refused={refused} failed=0 cancelled=0 unknown=0 unexpected_errors=0 accounting_answer_outcome_mismatches=0 real_calls=0 charged={} background={} journal_bytes={} journal_bytes_per_request={:.3} append_records={} file_syncs={}",
        b.charged(),b.background_charged(),bytes(root)-initial,(bytes(root)-initial) as f64/samples as f64,b.persistence_counts().0,b.persistence_counts().1);
    drop(b);
    for _ in 0..30 {
        let t = Instant::now();
        let b = AtomicReplay::open(root, c).unwrap();
        restart.push(t.elapsed());
        assert_eq!(b.charged(), samples * 47);
        assert_eq!(b.background_charged(), (samples / 2) * 47);
        assert_eq!(b.next_request_id(), samples + 1);
        assert!(!b.suspended());
        drop(b);
    }
    stats("durable_admission", admission);
    stats("completion_including_decode_and_settlement", completion);
    stats("restart_replay", restart);
    stats("validation_memory", validation);
    stats("compilation_memory", compilation);
}
