//! Synthetic durable admission measurement. Read boundary excludes decoder/settlement latency.
#[cfg(feature = "atomic-admission")]
use atomos_core::atomic_admission::{AtomicReplay, CompiledRequest, Configuration, NAMESPACE};
use atomos_core::{
    authorization::{AuthorizedReplay, Envelope, Policy},
    broker::Request,
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
    let mode = &args[1];
    let root = Path::new(&args[2]);
    let samples: u64 = args[3].parse().unwrap();
    assert!((1..=1000).contains(&samples));
    let trace = args.get(4).map_or(false, |s| s == "trace");
    let mut admission = Vec::new();
    let mut completion = Vec::new();
    let mut restart = Vec::new();
    if mode == "reference" {
        let mut b = AuthorizedReplay::initialize(root, policy()).unwrap();
        let initial = bytes(root);
        for id in 1..=samples {
            let measured = Rc::new(Cell::new(None));
            if trace {
                println!("TRACE_START");
            }
            let start = Instant::now();
            let source = Boundary {
                input: Cursor::new(SSE),
                start,
                time: measured.clone(),
                trace,
            };
            let result = b.run_sse(env(id), req(id), 4, source, || false);
            completion.push(start.elapsed());
            assert!(result.is_ok(), "reference request {id} failed: {result:?}");
            admission.push(measured.get().unwrap());
            assert_eq!(b.charged(), id * 47);
            if trace {
                println!("TRACE_END");
            }
        }
        println!(
            "mode=reference errors=0 charged={} journal_bytes={} journal_bytes_per_request={:.3}",
            b.charged(),
            bytes(root) - initial,
            (bytes(root) - initial) as f64 / samples as f64
        );
        drop(b);
        for _ in 0..30 {
            let t = Instant::now();
            let b = AuthorizedReplay::open(root, policy()).unwrap();
            restart.push(t.elapsed());
            assert_eq!(b.charged(), samples * 47);
            drop(b);
        }
    } else {
        #[cfg(feature = "atomic-admission")]
        {
            let c = Configuration {
                policy: policy(),
                version: 1,
                model: 7,
                model_eligible: true,
            };
            let mut b = AtomicReplay::initialize(root, c).unwrap();
            let initial = bytes(root);
            let mut validation = Vec::new();
            let mut compilation = Vec::new();
            for id in 1..=samples {
                let t = Instant::now();
                let r = CompiledRequest::compile(env(id), 7, 1, req(id)).unwrap();
                compilation.push(t.elapsed());
                let t = Instant::now();
                b.validate(&r, 4).unwrap();
                validation.push(t.elapsed());
                let measured = Rc::new(Cell::new(None));
                if trace {
                    println!("TRACE_START");
                }
                let start = Instant::now();
                let source = Boundary {
                    input: Cursor::new(SSE),
                    start,
                    time: measured.clone(),
                    trace,
                };
                let result = b.run_sse(&r, 4, source, || false);
                completion.push(start.elapsed());
                assert!(result.is_ok(), "candidate request {id} failed: {result:?}");
                admission.push(measured.get().unwrap());
                assert_eq!(b.charged(), id * 47);
                if trace {
                    println!("TRACE_END");
                }
            }
            println!("mode=candidate errors=0 charged={} background={} journal_bytes={} journal_bytes_per_request={:.3} append_records={} file_syncs={}",b.charged(),b.background_charged(),bytes(root)-initial,(bytes(root)-initial)as f64/samples as f64,b.persistence_counts().0,b.persistence_counts().1);
            stats("candidate_validation_memory", validation);
            stats("candidate_compilation_memory", compilation);
            drop(b);
            for _ in 0..30 {
                let t = Instant::now();
                let b = AtomicReplay::open(root, c).unwrap();
                restart.push(t.elapsed());
                assert_eq!(b.charged(), samples * 47);
                drop(b);
            }
            assert!(root.join(NAMESPACE).exists());
        }
        #[cfg(not(feature = "atomic-admission"))]
        panic!("candidate feature required");
    }
    stats("durable_admission", admission);
    stats("completion_including_decode_and_settlement", completion);
    stats("restart_replay", restart);
    println!(
        "accounting_expected={} parity_mismatches=0 real_calls=0",
        samples * 47
    );
}
