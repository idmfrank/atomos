//! Bounded synchronous offline session. Working turns never enter admission.
use crate::turn::{TurnId, TurnManager};
use std::{collections::VecDeque, io};

pub const MAX_INPUT: usize = 1024;
pub const MAX_PAIRS: usize = 4;
pub const MAX_HISTORY: usize = 16 * 1024;
pub const MAX_LOCAL: usize = 8192;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Input,
    Exhausted,
    Output,
    Stale,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    Local(String),
    Mock(String),
    #[cfg(all(unix, feature = "atomic-admission"))]
    Replay(Result<crate::atomic_admission::ReplayOutcome, crate::atomic_admission::Error>),
}

/// Validate bounded raw payload before trimming; CR is stripped only with LF.
pub fn input(bytes: &[u8]) -> Result<&str, Error> {
    let payload = if let Some(b) = bytes.strip_suffix(b"\n") {
        b.strip_suffix(b"\r").unwrap_or(b)
    } else {
        bytes
    };
    if payload.len() > MAX_INPUT {
        return Err(Error::Input);
    }
    let text = std::str::from_utf8(payload).map_err(|_| Error::Input)?;
    if text.chars().any(char::is_control) {
        return Err(Error::Input);
    }
    Ok(text.trim())
}
pub fn safe_output(text: &str, max: usize) -> bool {
    text.len() <= max
        && !text.trim().is_empty()
        && !text
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\t'))
}

#[derive(Default)]
pub struct Session {
    turns: TurnManager,
    sequence: u64,
    pairs: VecDeque<(String, String)>,
    bytes: usize,
    local: u64,
    mock: u64,
}
impl Session {
    pub fn history_pairs(&self) -> usize {
        self.pairs.len()
    }
    pub fn history_bytes(&self) -> usize {
        self.bytes
    }
    pub fn local_calls(&self) -> u64 {
        self.local
    }
    pub fn mock_calls(&self) -> u64 {
        self.mock
    }
    pub fn clear(&mut self) {
        self.pairs.clear();
        self.bytes = 0;
    }
    fn begin(&mut self, text: &str) -> Result<TurnId, Error> {
        if text.is_empty() || input(text.as_bytes())? != text {
            return Err(Error::Input);
        }
        let next = self.sequence.checked_add(1).ok_or(Error::Exhausted)?;
        let id = self.turns.wake().map_err(|_| Error::Exhausted)?;
        self.turns.commit(id, text).map_err(|_| Error::Stale)?;
        self.sequence = next;
        Ok(id)
    }
    fn deliver(
        &mut self,
        id: TurnId,
        question: &str,
        event: Event,
        publish: impl FnOnce(&Event) -> io::Result<()>,
    ) -> Result<(), Error> {
        if !self.turns.accepts_output(id) {
            return Err(Error::Stale);
        }
        let answer = match &event {
            Event::Mock(text) => Some(text.as_str()),
            #[cfg(all(unix, feature = "atomic-admission"))]
            Event::Replay(Ok(crate::atomic_admission::ReplayOutcome::Completed(a))) => {
                Some(a.text.as_str())
            }
            _ => None,
        };
        if let Some(text) = answer {
            if !safe_output(text, 4096) || question.len() + text.len() > MAX_HISTORY {
                return Err(Error::Input);
            }
        }
        // Publication includes flush. Failed or partial output never enters history.
        let written = publish(&event);
        self.turns.finish(id).map_err(|_| Error::Stale)?;
        written.map_err(|_| Error::Output)?;
        if let Some(answer) = answer {
            let size = question.len() + answer.len();
            while self.pairs.len() >= MAX_PAIRS || self.bytes + size > MAX_HISTORY {
                let (q, a) = self.pairs.pop_front().ok_or(Error::Input)?;
                self.bytes -= q.len() + a.len();
            }
            self.bytes += size;
            self.pairs.push_back((question.into(), answer.into()));
        }
        Ok(())
    }
    pub fn local(
        &mut self,
        question: &str,
        answer: String,
        publish: impl FnOnce(&Event) -> io::Result<()>,
    ) -> Result<(), Error> {
        if !safe_output(&answer, MAX_LOCAL) {
            return Err(Error::Input);
        }
        let next = self.local.checked_add(1).ok_or(Error::Exhausted)?;
        let id = self.begin(question)?;
        self.local = next;
        self.deliver(id, question, Event::Local(answer), publish)
    }
    pub fn mock(
        &mut self,
        question: &str,
        publish: impl FnOnce(&Event) -> io::Result<()>,
    ) -> Result<(), Error> {
        let next = self.mock.checked_add(1).ok_or(Error::Exhausted)?;
        let id = self.begin(question)?;
        self.mock = next;
        self.deliver(
            id,
            question,
            Event::Mock(format!("Mock response: {question}")),
            publish,
        )
    }
}

#[cfg(all(unix, feature = "atomic-admission"))]
pub mod replay {
    use super::*;
    use crate::{
        atomic_admission::{AtomicReplay, CompiledRequest, Context},
        authorization::Envelope,
        broker::Request,
        governor::Priority,
    };
    /// Trusted synthetic caller parameters; no credentials, context or permit.
    pub struct Parameters {
        pub envelope: Envelope,
        pub model: u64,
        pub version: u64,
        pub task: u64,
        pub now: u64,
        pub priority: Priority,
        pub max_output: u64,
    }
    impl Session {
        pub fn replay(
            &mut self,
            broker: &mut AtomicReplay,
            question: &str,
            p: Parameters,
            source: impl io::Read,
            mut cancelled: impl FnMut() -> bool,
            publish: impl FnOnce(&Event) -> io::Result<()>,
        ) -> Result<(), Error> {
            let id = self.begin(question)?;
            let r = CompiledRequest::compile(
                p.envelope,
                p.model,
                p.version,
                Request {
                    turn: self.sequence,
                    task: p.task,
                    text: question,
                    priority: p.priority,
                    max_output: p.max_output,
                },
            );
            let result = match r {
                Ok(r) => broker.run_sse_extended(
                    &r,
                    Context {
                        now: p.now,
                        active_turn: self.sequence,
                        task: p.task,
                    },
                    source,
                    &mut cancelled,
                ),
                Err(e) => Err(e),
            };
            // Delivery cancellation after a committed S cannot undo accounting.
            if cancelled() {
                self.turns.mute().map_err(|_| Error::Exhausted)?;
                self.turns.unmute().map_err(|_| Error::Exhausted)?;
                return Err(Error::Stale);
            }
            self.deliver(id, question, Event::Replay(result), publish)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn session_input_boundaries_are_raw_bytes_and_safe_utf8() {
        assert_eq!(input(b" \r\n"), Ok(""));
        assert!(input(&vec![b'x'; 1024]).is_ok());
        assert!(input(&vec![b' '; 1025]).is_err());
        assert!(input("é".repeat(512).as_bytes()).is_ok());
        for b in [b"a\r".as_slice(), b"a\tb", b"a\x1b", b"\xff"] {
            assert!(input(b).is_err());
        }
        assert!(input("\u{0085}".as_bytes()).is_err());
        assert_eq!(input(b" q\r\n"), Ok("q"));
    }
    #[test]
    fn session_history_evicts_whole_pairs_and_clear_keeps_counts() {
        let mut s = Session::default();
        for i in 0..5 {
            s.mock(&format!("q{i}"), |_| Ok(())).unwrap();
        }
        assert_eq!(s.history_pairs(), 4);
        assert_eq!(s.pairs.front().unwrap().0, "q1");
        assert_eq!(s.history_bytes(), 4 * (2 + 17));
        s.clear();
        assert_eq!(s.history_bytes(), 0);
        assert_eq!(s.mock_calls(), 5);
        assert_eq!(Session::default().history_pairs(), 0);
    }
    #[test]
    fn session_payload_limit_and_local_exclusion_are_independent() {
        let mut s = Session::default();
        for _ in 0..4 {
            let q = "q".repeat(1024);
            let id = s.begin(&q).unwrap();
            s.deliver(id, &q, Event::Mock("a".repeat(4096)), |_| Ok(()))
                .unwrap();
        }
        assert_eq!(s.history_pairs(), 3);
        assert_eq!(s.history_bytes(), 15360);
        s.local("help", "safe local fact".into(), |_| Ok(()))
            .unwrap();
        assert_eq!(s.history_bytes(), 15360);
        assert_eq!(s.local_calls(), 1);
    }
    #[test]
    fn session_output_failure_and_stale_turn_never_retain_text() {
        let mut s = Session::default();
        assert_eq!(
            s.mock("q", |_| Err(io::ErrorKind::BrokenPipe.into())),
            Err(Error::Output)
        );
        assert_eq!(s.history_pairs(), 0);
        assert_eq!(s.mock_calls(), 1);
        let id = s.begin("q").unwrap();
        s.turns.mute().unwrap();
        assert_eq!(
            s.deliver(id, "q", Event::Mock("a".into()), |_| panic!(
                "stale delivery"
            )),
            Err(Error::Stale)
        );
    }
    #[test]
    fn session_checked_counter_exhaustion_never_publishes() {
        let mut s = Session::default();
        s.mock = u64::MAX;
        assert_eq!(
            s.mock("q", |_| panic!("overflow output")),
            Err(Error::Exhausted)
        );
        s.sequence = u64::MAX;
        assert_eq!(
            s.local("help", "local".into(), |_| panic!("overflow output")),
            Err(Error::Exhausted)
        );
        assert!(!safe_output("\x1b[2J", 8192));
        assert!(!safe_output("\u{009b}", 8192));
    }
}
