//! Synthetic state-machine measurement; never a device/model performance claim.
use atomos_tasks::{Runtime, State, Synthetic};
use std::path::PathBuf;
use std::time::Instant;

fn report(name: &str, values: &mut [f64]) {
    values.sort_by(f64::total_cmp);
    let n = values.len();
    println!(
        "metric={name} samples={n} p50_ms={:.6} p95_ms={:.6} p99_ms={:.6}",
        values[n / 2],
        values[(n * 95 / 100).min(n - 1)],
        values[(n * 99 / 100).min(n - 1)]
    );
}
fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    assert_eq!(args.len(), 1, "expected fresh synthetic directory");
    let path = PathBuf::from(&args[0]);
    let mut r = Runtime::initialize(&path, "fixture", "local-v1", 4 * 1024 * 1024).unwrap();
    let mut submit = Vec::new();
    let mut intent = Vec::new();
    let mut effect = Vec::new();
    let mut verify = Vec::new();
    let mut cancel = Vec::new();
    for n in 0..200 {
        let now = n * 10;
        let t = Instant::now();
        let id = r
            .submit(&format!("sample-{n}"), "synthetic counter task", now)
            .unwrap()
            .id;
        submit.push(t.elapsed().as_secs_f64() * 1000.);
        r.start(id, now).unwrap();
        let t = Instant::now();
        let a = r
            .prepare(
                id,
                Synthetic::Add {
                    item: "fixture",
                    delta: 1,
                },
                now,
            )
            .unwrap();
        intent.push(t.elapsed().as_secs_f64() * 1000.);
        if n % 2 == 0 {
            let t = Instant::now();
            r.apply(id, a, now).unwrap();
            effect.push(t.elapsed().as_secs_f64() * 1000.);
            let t = Instant::now();
            assert!(r.verify(id, a, now).unwrap());
            verify.push(t.elapsed().as_secs_f64() * 1000.);
            r.complete(id, now).unwrap();
            assert_eq!(r.get(id).unwrap().state, State::Completed);
        } else {
            let t = Instant::now();
            r.cancel(id, now).unwrap();
            cancel.push(t.elapsed().as_secs_f64() * 1000.);
            assert!(r.apply(id, a, now).is_err());
            r.finish_cancel(id, now).unwrap();
            assert_eq!(r.get(id).unwrap().state, State::Cancelled);
        }
    }
    assert_eq!(r.counter("fixture").unwrap(), 100);
    let mut cursor = 0;
    let mut events = 0;
    loop {
        let page = r.events(cursor, 64).unwrap();
        if page.is_empty() {
            break;
        }
        events += page.len();
        cursor = page.last().unwrap().cursor;
    }
    assert_eq!(events, 700);
    drop(r);
    let t = Instant::now();
    let mut r = Runtime::open(&path, "fixture", "local-v1").unwrap();
    assert!(r.recover(2000).unwrap().is_empty());
    assert_eq!(r.counter("fixture").unwrap(), 100);
    println!(
        "metric=open_and_recovery samples=1 elapsed_ms={:.6}",
        t.elapsed().as_secs_f64() * 1000.
    );
    for (name, v) in [
        ("submit", &mut submit),
        ("durable_intent", &mut intent),
        ("synthetic_effect", &mut effect),
        ("verification", &mut verify),
        ("cancel_ack", &mut cancel),
    ] {
        report(name, v);
    }
    let bytes: u64 = std::fs::read_dir(&path)
        .unwrap()
        .map(|e| e.unwrap().metadata().unwrap().len())
        .sum();
    println!("completed=100 cancelled=100 fixture_counter=100 events=700 errors=0 real_calls=0 store_apparent_bytes={bytes}");
}
