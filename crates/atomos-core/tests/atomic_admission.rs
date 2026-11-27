#![cfg(all(unix, feature = "atomic-admission"))]
use atomos_core::{
    atomic_admission::{AtomicReplay, CompiledRequest, Configuration, Error, NAMESPACE},
    authorization::{AuthorizedReplay, Envelope, Policy},
    broker::Request,
    governor::{Limits, Priority},
};
use std::{
    fs,
    io::Read,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "atomos-atomic-{}-{}",
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

#[test]
fn admission_has_one_atomic_reservation_and_restart_retains_it() {
    let f = Fixture::new();
    let mut b = AtomicReplay::initialize(&f.0, config()).unwrap();
    assert_eq!(
        b.run_sse(&compiled(1), 4, &b""[..], || false),
        Err(Error::UnknownUsage)
    );
    assert_eq!(b.charged(), 200);
    assert_eq!(b.dispatches(), 1);
    assert_eq!(b.persistence_counts(), (1, 1));
    let log = fs::read_to_string(f.journal()).unwrap();
    assert_eq!(log.lines().count(), 2);
    assert!(log.contains(compiled(1).identity()));
    assert!(!log.contains("question"));
    drop(b);
    let mut b = AtomicReplay::open(&f.0, config()).unwrap();
    assert_eq!(b.charged(), 200);
    assert_eq!(b.next_request_id(), 2);
    assert!(b.suspended());
    assert_eq!(
        b.run_sse(&compiled(2), 4, NeverRead, || false),
        Err(Error::Suspended)
    );
}
#[test]
fn account_host_grant_model_and_policy_must_match() {
    let f = Fixture::new();
    let mut b = AtomicReplay::initialize(&f.0, config()).unwrap();
    for e in [
        Envelope {
            account: 9,
            ..env(1)
        },
        Envelope { host: 9, ..env(1) },
        Envelope { grant: 9, ..env(1) },
    ] {
        let r = CompiledRequest::compile(e, 7, 1, req()).unwrap();
        assert_eq!(b.run_sse(&r, 4, NeverRead, || false), Err(Error::Scope));
    }
    for (m, v) in [(8, 1), (7, 2)] {
        let r = CompiledRequest::compile(env(1), m, v, req()).unwrap();
        assert_eq!(b.run_sse(&r, 4, NeverRead, || false), Err(Error::Scope));
    }
    assert_eq!(b.charged(), 0);
    assert_eq!(b.persistence_counts(), (0, 0));
}
#[test]
fn missing_or_invalid_eligibility_never_reads() {
    let c = config();
    for c in [
        Configuration {
            model_eligible: false,
            ..c
        },
        Configuration {
            policy: Policy {
                eligible: false,
                ..c.policy
            },
            ..c
        },
        Configuration {
            policy: Policy {
                direct_scope: false,
                ..c.policy
            },
            ..c
        },
        Configuration {
            policy: Policy {
                credits_disabled: false,
                ..c.policy
            },
            ..c
        },
        Configuration {
            policy: Policy {
                auto_purchase_disabled: false,
                ..c.policy
            },
            ..c
        },
        Configuration {
            policy: Policy {
                app_limit_percent: 0,
                ..c.policy
            },
            ..c
        },
        Configuration {
            policy: Policy {
                app_limit_percent: 100,
                ..c.policy
            },
            ..c
        },
    ] {
        let f = Fixture::new();
        let mut b = AtomicReplay::initialize(&f.0, c).unwrap();
        assert_eq!(
            b.run_sse(&compiled(1), 4, NeverRead, || false),
            Err(Error::Evidence)
        );
        assert_eq!(b.charged(), 0);
    }
}
#[test]
fn immutable_compilation_binds_all_fields_and_active_turn() {
    let original = compiled(1);
    let mut owned = "question".to_string();
    let r = CompiledRequest::compile(
        env(1),
        7,
        1,
        Request {
            text: &owned,
            ..req()
        },
    )
    .unwrap();
    owned.replace_range(.., "different");
    assert_eq!(r.identity(), original.identity());
    for q in [
        Request {
            text: "altered!",
            ..req()
        },
        Request { task: 6, ..req() },
        Request { turn: 5, ..req() },
        Request {
            max_output: 63,
            ..req()
        },
        Request {
            priority: Priority::Background,
            ..req()
        },
    ] {
        assert_ne!(
            CompiledRequest::compile(env(1), 7, 1, q)
                .unwrap()
                .identity(),
            original.identity()
        );
    }
    let f = Fixture::new();
    let mut b = AtomicReplay::initialize(&f.0, config()).unwrap();
    assert_eq!(b.run_sse(&r, 5, NeverRead, || false), Err(Error::StaleTurn));
    assert_eq!(b.charged(), 0);
}
#[test]
fn successful_settlement_and_duplicate_identity_reject_after_restart() {
    let f = Fixture::new();
    let mut b = AtomicReplay::initialize(&f.0, config()).unwrap();
    let a = b.run_sse(&compiled(1), 4, SSE, || false).unwrap();
    assert_eq!(a.usage.unwrap().input, 35);
    assert_eq!(a.usage.unwrap().output, 12);
    assert_eq!(b.charged(), 47);
    assert_eq!(b.persistence_counts(), (2, 2));
    assert_eq!(
        b.run_sse(&compiled(1), 4, NeverRead, || false),
        Err(Error::Replay)
    );
    drop(b);
    let mut b = AtomicReplay::open(&f.0, config()).unwrap();
    assert_eq!(b.charged(), 47);
    assert_eq!(
        b.run_sse(&compiled(1), 4, NeverRead, || false),
        Err(Error::Replay)
    );
    let altered = CompiledRequest::compile(
        env(1),
        7,
        1,
        Request {
            text: "changed!",
            ..req()
        },
    )
    .unwrap();
    assert_eq!(
        b.run_sse(&altered, 4, NeverRead, || false),
        Err(Error::Replay)
    );
}
#[test]
fn expiry_grant_window_and_clock_rollback() {
    let f = Fixture::new();
    let mut b = AtomicReplay::initialize(&f.0, config()).unwrap();
    for e in [
        Envelope {
            expires_at: 200,
            ..env(1)
        },
        Envelope {
            expires_at: 261,
            ..env(1)
        },
        Envelope {
            now: 900,
            expires_at: 920,
            ..env(1)
        },
        Envelope {
            now: 899,
            expires_at: 901,
            ..env(1)
        },
        Envelope {
            now: 1000,
            expires_at: 1010,
            ..env(1)
        },
    ] {
        assert_eq!(
            b.run_sse(
                &CompiledRequest::compile(e, 7, 1, req()).unwrap(),
                4,
                NeverRead,
                || false
            ),
            Err(Error::Expired)
        );
    }
    b.run_sse(&compiled(1), 4, SSE, || false).unwrap();
    let e = Envelope { now: 199, ..env(2) };
    assert_eq!(
        b.run_sse(
            &CompiledRequest::compile(e, 7, 1, req()).unwrap(),
            4,
            NeverRead,
            || false
        ),
        Err(Error::ClockRollback)
    );
}
#[test]
fn per_request_total_background_and_interactive_reserve() {
    for (limits, priority) in [
        (
            Limits {
                total: 500,
                background: 500,
                interactive_reserve: 0,
                per_request: 199,
            },
            Priority::Interactive,
        ),
        (
            Limits {
                total: 199,
                background: 199,
                interactive_reserve: 0,
                per_request: 199,
            },
            Priority::Interactive,
        ),
        (
            Limits {
                total: 500,
                background: 199,
                interactive_reserve: 0,
                per_request: 250,
            },
            Priority::Background,
        ),
        (
            Limits {
                total: 500,
                background: 500,
                interactive_reserve: 301,
                per_request: 250,
            },
            Priority::Background,
        ),
    ] {
        let f = Fixture::new();
        let c = config();
        let c = Configuration {
            policy: Policy { limits, ..c.policy },
            ..c
        };
        let mut b = AtomicReplay::initialize(&f.0, c).unwrap();
        let r = CompiledRequest::compile(env(1), 7, 1, Request { priority, ..req() }).unwrap();
        assert_eq!(b.run_sse(&r, 4, NeverRead, || false), Err(Error::Budget));
        assert_eq!(b.next_request_id(), 1);
        assert_eq!(b.persistence_counts(), (0, 0));
    }
    let f = Fixture::new();
    let c = config();
    let c = Configuration {
        policy: Policy {
            limits: Limits {
                total: 240,
                background: 240,
                interactive_reserve: 0,
                per_request: 200,
            },
            ..c.policy
        },
        ..c
    };
    let mut b = AtomicReplay::initialize(&f.0, c).unwrap();
    b.run_sse(&compiled(1), 4, SSE, || false).unwrap();
    assert_eq!(
        b.run_sse(&compiled(2), 4, NeverRead, || false),
        Err(Error::Budget)
    );
    assert_eq!(b.charged(), 47);
}
#[test]
fn interactive_can_use_reserved_capacity() {
    let f = Fixture::new();
    let c = config();
    let c = Configuration {
        policy: Policy {
            limits: Limits {
                total: 500,
                background: 500,
                interactive_reserve: 400,
                per_request: 250,
            },
            ..c.policy
        },
        ..c
    };
    let mut b = AtomicReplay::initialize(&f.0, c).unwrap();
    assert!(b.run_sse(&compiled(1), 4, SSE, || false).is_ok());
}
#[test]
fn exclusive_writer_and_existing_namespace_never_overwrite() {
    let f = Fixture::new();
    let b = AtomicReplay::initialize(&f.0, config()).unwrap();
    assert!(matches!(
        AtomicReplay::open(&f.0, config()),
        Err(Error::Storage)
    ));
    assert!(matches!(
        AtomicReplay::initialize(&f.0, config()),
        Err(Error::Storage)
    ));
    drop(b);
    assert!(AtomicReplay::open(&f.0, config()).is_ok());
}
#[test]
fn missing_and_incompatible_journal_are_not_recreated() {
    let f = Fixture::new();
    assert!(AtomicReplay::open(&f.0, config()).is_err());
    drop(AtomicReplay::initialize(&f.0, config()).unwrap());
    let old = fs::read(f.journal()).unwrap();
    fs::write(f.journal(), b"AA2 incompatible\n").unwrap();
    assert!(AtomicReplay::open(&f.0, config()).is_err());
    fs::remove_file(f.journal()).unwrap();
    assert!(AtomicReplay::open(&f.0, config()).is_err());
    assert!(!f.journal().exists());
    assert!(!old.is_empty());
}
#[test]
fn truncated_duplicate_malformed_and_over_limit_records_fail_closed() {
    let original = {
        let f = Fixture::new();
        let mut b = AtomicReplay::initialize(&f.0, config()).unwrap();
        let _ = b.run_sse(&compiled(1), 4, &b""[..], || false);
        fs::read_to_string(f.journal()).unwrap()
    };
    let header = original.lines().next().unwrap();
    let record = original.lines().nth(1).unwrap();
    for text in [
        original[..original.len() - 1].to_string(),
        format!("{original}{record}\n"),
        format!("{original}S 1 1000000001 1\n"),
        format!("{original}H\nS 1 47 0\n"),
        format!("{header}\nA malformed\n"),
        format!("{header}\n{}\n", "x".repeat(513)),
        String::new(),
        format!("{header}\n{}\n", "x".repeat(1024 * 1024)),
    ] {
        let f = Fixture::new();
        drop(AtomicReplay::initialize(&f.0, config()).unwrap());
        fs::write(f.journal(), text).unwrap();
        assert!(AtomicReplay::open(&f.0, config()).is_err());
    }
}
#[test]
fn revocation_is_durable_even_with_outstanding_reservation() {
    let f = Fixture::new();
    let mut b = AtomicReplay::initialize(&f.0, config()).unwrap();
    let _ = b.run_sse(&compiled(1), 4, &b""[..], || false);
    b.revoke().unwrap();
    drop(b);
    let mut b = AtomicReplay::open(&f.0, config()).unwrap();
    assert!(b.suspended());
    assert_eq!(b.charged(), 200);
    assert_eq!(
        b.run_sse(&compiled(2), 4, NeverRead, || false),
        Err(Error::Suspended)
    );
}
#[test]
fn local_commands_remain_available_without_reservation() {
    let f = Fixture::new();
    let mut b = AtomicReplay::initialize(&f.0, config()).unwrap();
    b.revoke().unwrap();
    for text in ["help", "what is your name?"] {
        let r = CompiledRequest::compile(env(1), 999, 999, Request { text, ..req() }).unwrap();
        assert!(!b.run_sse(&r, 4, NeverRead, || false).unwrap().simulated);
    }
    assert_eq!(b.charged(), 0);
    assert_eq!(b.dispatches(), 0);
    assert_eq!(b.persistence_counts(), (1, 1));
}
#[test]
fn private_permissions_symlinks_and_hardlinks_reject() {
    let f = Fixture::new();
    drop(AtomicReplay::initialize(&f.0, config()).unwrap());
    fs::set_permissions(f.journal(), fs::Permissions::from_mode(0o644)).unwrap();
    assert!(AtomicReplay::open(&f.0, config()).is_err());
    fs::set_permissions(f.journal(), fs::Permissions::from_mode(0o600)).unwrap();
    fs::hard_link(f.journal(), f.0.join("linked")).unwrap();
    assert!(AtomicReplay::open(&f.0, config()).is_err());
    fs::remove_file(f.0.join("linked")).unwrap();
    fs::rename(f.journal(), f.0.join("original")).unwrap();
    std::os::unix::fs::symlink(f.0.join("original"), f.journal()).unwrap();
    assert!(AtomicReplay::open(&f.0, config()).is_err());
}
#[test]
fn reference_candidate_successful_accounting_parity() {
    let f = Fixture::new();
    let c = config();
    let mut reference = AuthorizedReplay::initialize(&f.0, c.policy).unwrap();
    let mut candidate = AtomicReplay::initialize(&f.0, c).unwrap();
    for id in 1..=12 {
        let priority = if id % 2 == 0 {
            Priority::Background
        } else {
            Priority::Interactive
        };
        let a = reference
            .run_sse(env(id), Request { priority, ..req() }, 4, SSE, || false)
            .unwrap();
        let r = CompiledRequest::compile(env(id), 7, 1, Request { priority, ..req() }).unwrap();
        assert_eq!(candidate.run_sse(&r, 4, SSE, || false).unwrap(), a);
        assert_eq!(candidate.charged(), id * 47);
        assert_eq!(candidate.charged(), reference.charged());
    }
    assert_eq!(candidate.background_charged(), 6 * 47);
}
#[test]
fn journal_capacity_failure_never_reads_or_dispatches() {
    let f = Fixture::new();
    let mut b = AtomicReplay::initialize(&f.0, config()).unwrap();
    let file = fs::OpenOptions::new()
        .write(true)
        .open(f.journal())
        .unwrap();
    file.set_len(1024 * 1024).unwrap();
    assert_eq!(
        b.run_sse(&compiled(1), 4, NeverRead, || false),
        Err(Error::Storage)
    );
    assert_eq!(b.dispatches(), 0);
    assert_eq!(b.charged(), 0);
    assert!(b.suspended());
}
#[test]
fn no_response_read_until_durable_identity_and_reservation() {
    struct Inspect(PathBuf);
    impl Read for Inspect {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            let log = fs::read_to_string(&self.0).unwrap();
            assert!(log.contains(compiled(1).identity()));
            assert!(log.ends_with(" 200\n"));
            Ok(0)
        }
    }
    let f = Fixture::new();
    let mut b = AtomicReplay::initialize(&f.0, config()).unwrap();
    assert_eq!(
        b.run_sse(&compiled(1), 4, Inspect(f.journal()), || false),
        Err(Error::UnknownUsage)
    );
}
#[test]
fn cancellation_retains_reservation() {
    let f = Fixture::new();
    let mut b = AtomicReplay::initialize(&f.0, config()).unwrap();
    assert_eq!(
        b.run_sse(&compiled(1), 4, SSE, || true),
        Err(Error::Cancelled)
    );
    assert_eq!(b.charged(), 200);
    assert!(b.suspended());
}
#[test]
fn invalid_compiled_requests_never_exist() {
    for r in [
        Request { text: "", ..req() },
        Request { turn: 0, ..req() },
        Request { task: 0, ..req() },
        Request {
            max_output: u64::MAX,
            ..req()
        },
        Request {
            text: "bad\n",
            ..req()
        },
    ] {
        assert!(matches!(
            CompiledRequest::compile(env(1), 7, 1, r),
            Err(Error::Invalid)
        ));
    }
}
#[test]
fn settlement_overrun_suspends_durably() {
    // Mutate synthetic fixture only; no recorded provider content.
    let text = std::str::from_utf8(SSE).unwrap();
    let over = text
        .replace("\"input_tokens\":35", "\"input_tokens\":240")
        .replace("\"total_tokens\":47", "\"total_tokens\":252");
    assert_ne!(over, text);
    let f = Fixture::new();
    let mut b = AtomicReplay::initialize(&f.0, config()).unwrap();
    assert_eq!(
        b.run_sse(&compiled(1), 4, over.as_bytes(), || false),
        Err(Error::Overrun)
    );
    assert_eq!(b.charged(), 252);
    drop(b);
    assert!(AtomicReplay::open(&f.0, config()).unwrap().suspended());
}

#[test]
fn provider_rate_and_authorization_failures_suspend_durably() {
    for code in ["rate_limit_exceeded", "invalid_api_key"] {
        let fixture = std::str::from_utf8(SSE).unwrap().replace("\r\n", "\n");
        let created = fixture
            .split("\n\n")
            .next()
            .unwrap();
        let stream=format!("{created}\r\n\r\nevent: error\r\ndata: {{\"type\":\"error\",\"sequence_number\":1,\"code\":\"{code}\",\"message\":\"synthetic\"}}\r\n\r\n");
        let f = Fixture::new();
        let mut b = AtomicReplay::initialize(&f.0, config()).unwrap();
        assert!(matches!(
            b.run_sse(&compiled(1), 4, stream.as_bytes(), || false),
            Err(Error::Provider(_))
        ));
        drop(b);
        let mut b = AtomicReplay::open(&f.0, config()).unwrap();
        assert!(b.suspended());
        assert_eq!(b.charged(), 200);
        assert_eq!(
            b.run_sse(&compiled(2), 4, NeverRead, || false),
            Err(Error::Suspended)
        );
    }
}
