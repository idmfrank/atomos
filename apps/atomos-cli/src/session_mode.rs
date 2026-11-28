//! Thin terminal adapter. No signals, network, credentials or history files.
use crate::conversation::{Channel, Conversation};
use atomos_core::session::{self, Event, Session};
use std::{
    ffi::OsString,
    io::{self, BufRead, Read, Write},
    path::PathBuf,
};

struct Options {
    memory: Option<(PathBuf, String)>,
    replay: Option<String>,
    ledger: Option<PathBuf>,
    init: bool,
}
const NAMES: &[&str] = &[
    "text",
    "auxiliary",
    "reasoning",
    "refusal",
    "failed-known",
    "failed-unknown",
    "rate-limit",
    "malformed",
];
fn parse(args: &[OsString]) -> Result<Options, ()> {
    let mut o = Options {
        memory: None,
        replay: None,
        ledger: None,
        init: false,
    };
    let mut i = 0;
    if args.first().is_some_and(|s| s == "--memory") {
        if args.len() < 3 {
            return Err(());
        }
        o.memory = Some((PathBuf::from(&args[1]), args[2].to_str().ok_or(())?.into()));
        i = 3;
    }
    let mut session = false;
    while i < args.len() {
        match args[i].to_str().ok_or(())? {
            "--session" if !session => session = true,
            "--init-ledger" if !o.init => o.init = true,
            "--replay" if o.replay.is_none() => {
                i += 1;
                let name = args.get(i).and_then(|s| s.to_str()).ok_or(())?;
                if !NAMES.contains(&name) {
                    return Err(());
                }
                o.replay = Some(name.into());
            }
            "--ledger" if o.ledger.is_none() => {
                i += 1;
                let value = args.get(i).ok_or(())?;
                if value.is_empty() || value.to_str().is_some_and(|v|v.starts_with("--")) { return Err(()); }
                o.ledger = Some(PathBuf::from(value));
            }
            _ => return Err(()),
        }
        i += 1;
    }
    if !session
        || o.replay.is_some() != o.ledger.is_some()
        || (o.init && o.replay.is_none())
        || (o.memory.is_some() && o.replay.is_some())
    {
        return Err(());
    }
    if o.replay.is_some() && !cfg!(all(unix, feature = "session-replay")) {
        return Err(());
    }
    Ok(o)
}
pub fn run_if_requested(args: &[OsString]) -> Option<i32> {
    let start = if args.first().is_some_and(|s| s == "--memory") && args.len() >= 3 {
        3
    } else {
        0
    };
    if !args[start..].iter().any(|s| {
        matches!(
            s.to_str(),
            Some("--session" | "--replay" | "--ledger" | "--init-ledger")
        )
    }) {
        return None;
    }
    let options = match parse(args) {
        Ok(o) => o,
        Err(()) => {
            eprintln!("Invalid session options; replay requires a supported build, --replay NAME and --ledger DIR.");
            return Some(2);
        }
    };
    Some(match run(options) {
        Ok(code) => code,
        Err(()) => {
            eprintln!("Session stopped: input, output or storage unavailable.");
            1
        }
    })
}

enum Backend {
    Mock,
    #[cfg(all(unix, feature = "session-replay"))]
    Replay {
        broker: atomos_core::atomic_admission::AtomicReplay,
        wire: Vec<u8>,
    },
}
impl Backend {
    fn open(o: &Options) -> Result<Self, ()> {
        if o.replay.is_none() {
            return Ok(Self::Mock);
        }
        #[cfg(all(unix, feature = "session-replay"))]
        {
            use atomos_core::atomic_admission::AtomicReplay;
            let root = o.ledger.as_ref().ok_or(())?;
            let b = if o.init {
                AtomicReplay::initialize(root, configuration())
            } else {
                AtomicReplay::open(root, configuration())
            }
            .map_err(|_| ())?;
            Ok(Self::Replay {
                broker: b,
                wire: fixture(o.replay.as_ref().ok_or(())?),
            })
        }
        #[cfg(not(all(unix, feature = "session-replay")))]
        Err(())
    }
    fn mode(&self) -> &'static str {
        match self {
            Self::Mock => "local/mock",
            #[cfg(all(unix, feature = "session-replay"))]
            Self::Replay { .. } => "synthetic-replay",
        }
    }
    fn blocked(&self) -> bool {
        match self {
            Self::Mock => false,
            #[cfg(all(unix, feature = "session-replay"))]
            Self::Replay { broker, .. } => broker.suspended(),
        }
    }
    fn status(&self, out: &mut impl Write) -> io::Result<()> {
        match self {
            Self::Mock => writeln!(out, "accounting=not-applicable synthetic_dispatches=0"),
            #[cfg(all(unix, feature = "session-replay"))]
            Self::Replay { broker, .. } => {
                let s = broker.status();
                if !s.healthy {
                    return writeln!(
                        out,
                        "accounting=unavailable blocked=true synthetic_dispatches={}",
                        broker.dispatches()
                    );
                }
                writeln!(out,"accounting=fixture-units charged={} background={} pending={} halted={} healthy=true next_id={} blocked={} synthetic_dispatches={}",s.confirmed_total,s.confirmed_background,s.pending_units.unwrap_or(0),s.halted,s.next_request_id,broker.suspended(),broker.dispatches())
            }
        }
    }
    fn reply(
        &mut self,
        s: &mut Session,
        text: &str,
        out: &mut impl Write,
    ) -> Result<(), session::Error> {
        match self {
            Self::Mock => s.mock(text, |e| publish(out, e)),
            #[cfg(all(unix, feature = "session-replay"))]
            Self::Replay { broker, wire } => {
                use atomos_core::{
                    authorization::Envelope, governor::Priority, session::replay::Parameters,
                };
                let p = Parameters {
                    envelope: Envelope {
                        account: 1,
                        host: 2,
                        grant: 3,
                        request_id: broker.next_request_id(),
                        now: 200,
                        expires_at: 230,
                    },
                    model: 7,
                    version: 1,
                    task: 5,
                    now: 200,
                    priority: Priority::Interactive,
                    max_output: 64,
                };
                s.replay(
                    broker,
                    text,
                    p,
                    wire.as_slice(),
                    || false,
                    |e| publish(out, e),
                )
            }
        }
    }
}
#[allow(unreachable_patterns)]
fn publish(out: &mut impl Write, e: &Event) -> io::Result<()> {
    match e {
        Event::Local(t) => writeln!(out, "[local]\n{t}")?,
        Event::Mock(t) => writeln!(out, "[mock completed]\n{t}")?,
        #[cfg(all(unix, feature = "session-replay"))]
        Event::Replay(result) => {
            use atomos_core::atomic_admission::{Error as E, ReplayOutcome as O};
            match result {
                Ok(O::Local(a)) => writeln!(out, "[local]\n{}", a.text)?,
                Ok(O::Completed(a)) => writeln!(out, "[synthetic completed]\n{}", a.text)?,
                Ok(O::Refused { usage, .. }) => writeln!(
                    out,
                    "[synthetic refused] input={} output={}; no answer or retry",
                    usage.input, usage.output
                )?,
                Ok(O::Failed { failure, usage, .. }) => writeln!(
                    out,
                    "[synthetic failed] kind={} usage={}",
                    failure_label(*failure),
                    if usage.is_some() { "known" } else { "unknown" }
                )?,
                Err(e) => writeln!(
                    out,
                    "[synthetic error] category={}",
                    match e {
                        E::Storage => "storage-unavailable",
                        E::Cancelled => "cancelled",
                        E::Overrun => "overrun",
                        E::UnknownUsage => "usage-unknown",
                        E::Suspended => "suspended",
                        E::Budget => "budget",
                        E::Scope => "scope",
                        E::Expired => "expired",
                        E::ClockRollback => "clock-rollback",
                        E::Replay => "request-id",
                        E::StaleTurn | E::StaleTask => "stale",
                        E::Evidence => "evidence",
                        E::Provider(_) => "unsupported",
                        E::Stream => "invalid-stream",
                        E::Invalid => "invalid",
                        E::Corrupt => "corrupt",
                    }
                )?,
            }
        }
        _ => return Err(io::ErrorKind::Unsupported.into()),
    }
    out.flush()
}
#[cfg(all(unix, feature = "session-replay"))]
fn failure_label(f: atomos_core::broker::Failure) -> &'static str {
    use atomos_core::broker::Failure::*;
    match f {
        RateLimited => "rate-limited",
        Authorization => "authorization",
        Unavailable => "unavailable",
        Unsupported => "unsupported",
        Incomplete => "incomplete",
    }
}
const HELP:&str="/help /status /clear /quit; exact help/name and local recall/context are supported.\nMock echoes; replay uses finite synthetic fixtures. Provider disabled; working turns are local-only, no model context is sent.\nMemory mutations use atomos-memory. No /cancel, retry or reset. Ctrl-C terminates; an admitted replay may remain charged with a stale lock.\n/clear does not erase terminal scrollback, securely wipe RAM, change facts or reset accounting.";
fn run(o: Options) -> Result<i32, ()> {
    // Complete parse/feature validation has already happened, before data access.
    let mut backend = Backend::open(&o)?;
    let memory = o.memory.is_some();
    let reader = match &o.memory {
        Some((dir, owner)) => Some(
            Box::new(atomos_memory::open_reader(dir, owner).map_err(|_| ())?)
                as Box<dyn atomos_memory::MemoryReader>,
        ),
        None => None,
    };
    let mut conversation = Conversation::new(reader, Channel::Text);
    let mut s = Session::default();
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    writeln!(
        out,
        "AtomOS session mode={} provider=disabled context=local-only",
        backend.mode()
    )
    .map_err(|_| ())?;
    out.flush().map_err(|_| ())?;
    loop {
        let mut bytes = Vec::with_capacity(session::MAX_INPUT + 3);
        let n = (&mut input)
            .take((session::MAX_INPUT + 3) as u64)
            .read_until(b'\n', &mut bytes)
            .map_err(|_| ())?;
        if n == 0 {
            break;
        }
        let text = session::input(&bytes).map_err(|_| ())?;
        if text.is_empty() {
            continue;
        }
        if text.starts_with('/') || text.eq_ignore_ascii_case("help") {
            conversation.clear();
            match text {
                "/quit"=>break,
                "/help"=>writeln!(out,"{HELP}"),
                "/status"=>{
                    writeln!(out,"mode={} provider=disabled memory={} context=local-only retained_pairs={} retained_bytes={} local_calls={} mock_calls={}",backend.mode(),if memory{"enabled"}else{"disabled"},s.history_pairs(),s.history_bytes(),s.local_calls(),s.mock_calls()).map_err(|_|())?;
                    backend.status(&mut out)
                }
                "/clear"=>{s.clear();writeln!(out,"[local] Working turns cleared; facts and accounting unchanged.")}
                _ if text.eq_ignore_ascii_case("help")=>writeln!(out,"{HELP}"),
                _=>writeln!(out,"[local error] Unknown command; use /help."),
            }.map_err(|_|())?;
            out.flush().map_err(|_| ())?;
            continue;
        }
        let result = if let Some(answer) = conversation.session_local(text) {
            let answer = if session::safe_output(&answer, session::MAX_LOCAL) {
                answer
            } else {
                "Local response unavailable.".into()
            };
            s.local(text, answer, |e| publish(&mut out, e))
        } else {
            backend.reply(&mut s, text, &mut out)
        };
        if result.is_err() {
            conversation.clear();
            return Err(());
        }
    }
    Ok(if backend.blocked() { 1 } else { 0 })
}

#[cfg(all(unix, feature = "session-replay"))]
fn configuration() -> atomos_core::atomic_admission::Configuration {
    use atomos_core::{atomic_admission::Configuration, authorization::Policy, governor::Limits};
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
#[cfg(all(unix, feature = "session-replay"))]
fn fixture(name: &str) -> Vec<u8> {
    let text = include_bytes!("../../../crates/atomos-core/tests/fixtures/responses-text.sse");
    match name {
        "text" => text.to_vec(),
        "auxiliary" => {
            include_bytes!("../../../crates/atomos-core/tests/fixtures/responses-compat.sse")
                .to_vec()
        }
        "reasoning" => {
            include_bytes!("../../../crates/atomos-core/tests/fixtures/responses-reasoning.sse")
                .to_vec()
        }
        "refusal" => {
            include_bytes!("../../../crates/atomos-core/tests/fixtures/responses-refusal.sse")
                .to_vec()
        }
        "malformed" => b"event: response.created\ndata: invalid\n\n".to_vec(),
        _ => {
            let t = std::str::from_utf8(text)
                .expect("synthetic fixture")
                .replace("\r\n", "\n");
            let created = t.split("\n\n").next().expect("synthetic created event");
            let usage = if name == "failed-known" {
                r#"{"input_tokens":35,"output_tokens":12,"total_tokens":47,"input_tokens_details":{"cached_tokens":10},"output_tokens_details":{"reasoning_tokens":4}}"#
            } else {
                "null"
            };
            let code = if name == "rate-limit" {
                "rate_limit_exceeded"
            } else {
                "server_error"
            };
            format!("{created}\n\nevent: response.failed\ndata: {{\"type\":\"response.failed\",\"sequence_number\":1,\"response\":{{\"id\":\"resp_synthetic\",\"object\":\"response\",\"status\":\"failed\",\"error\":{{\"code\":\"{code}\"}},\"incomplete_details\":null,\"usage\":{usage}}}}}\n\n").into_bytes()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn session_options_reject_conflicts_without_opening_data() {
        for args in [
            "--session --session",
            "--session --single-turn",
            "--session --voice-events",
            "--session --ledger missing",
            "--session --init-ledger",
            "--session --replay secret --ledger missing",
            "--memory missing owner --session --replay text --ledger missing",
            "--session --replay text",
        ] {
            let a: Vec<OsString> = args.split_whitespace().map(Into::into).collect();
            assert!(parse(&a).is_err());
        }
        assert!(parse(&["--session".into()]).is_ok());
    }
}
