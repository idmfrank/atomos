//! Persistent, single-owner voice coordinator. No network or audio driver.
//! A frame is exactly 20 ms of 16 kHz mono PCM. Sample-time gates are not
//! measurements of physical playback or real-world conversational latency.
use crate::{respond, MockInference};
use crate::turn::MAX_TRANSCRIPT_BYTES;
use std::collections::VecDeque;

pub const FRAME_SAMPLES: usize = 320;
pub type Frame = [i16; FRAME_SAMPLES];
pub type VoiceId = u64;
const ONSET: u32 = 3;
const END_SILENCE: u32 = 15;
const PREROLL: usize = 5;
const FOLLOWUP: u32 = 400;
const MAX_TURN: u32 = 1500;
const FINAL_TIMEOUT: u32 = 250;
const SPEAK_TIMEOUT: u32 = 1500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Idle,
    Armed,
    Listening(VoiceId),
    Finalizing(VoiceId),
    Speaking(VoiceId),
    Muted,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Start(VoiceId),
    Feed(VoiceId, Box<Frame>),
    Finalize(VoiceId),
    Speak(VoiceId, String),
    Cancel(VoiceId),
    Idle,
    Armed,
    Muted,
}

pub struct VoiceSession {
    state: State,
    next: VoiceId,
    threshold: u32,
    aec_verified: bool,
    elapsed: u32,
    voiced: u32,
    silent: u32,
    pre: VecDeque<Frame>,
    provider: MockInference,
}

impl VoiceSession {
    pub fn new(threshold: u32, aec_verified: bool) -> Self {
        Self {
            state: State::Idle,
            next: 1,
            threshold: threshold.clamp(1, 32768),
            aec_verified,
            elapsed: 0,
            voiced: 0,
            silent: 0,
            pre: VecDeque::with_capacity(PREROLL),
            provider: MockInference::default(),
        }
    }

    pub fn state(&self) -> State { self.state }
    pub fn mock_calls(&self) -> usize { self.provider.calls }

    fn set(&mut self, state: State) {
        self.state = state;
        self.elapsed = 0;
        self.voiced = 0;
        self.silent = 0;
        self.pre.clear();
    }

    fn active_id(&self) -> Option<VoiceId> {
        match self.state {
            State::Listening(id) | State::Finalizing(id) | State::Speaking(id) => Some(id),
            _ => None,
        }
    }

    fn reset_to(&mut self, state: State, notification: Action) -> Vec<Action> {
        let mut out = Vec::new();
        if let Some(id) = self.active_id() { out.push(Action::Cancel(id)); }
        self.set(state);
        out.push(notification);
        out
    }

    /// Explicit activation only. Does not unmute an owner-muted device.
    pub fn arm(&mut self) -> Vec<Action> {
        if self.state != State::Idle { return Vec::new(); }
        self.set(State::Armed);
        vec![Action::Armed]
    }
    pub fn mute(&mut self) -> Vec<Action> {
        self.reset_to(State::Muted, Action::Muted)
    }
    pub fn unmute(&mut self) -> Vec<Action> {
        if self.state != State::Muted { return Vec::new(); }
        self.set(State::Idle);
        vec![Action::Idle]
    }
    /// Lost samples, device errors, owner stop: discard incomplete turn.
    pub fn stop(&mut self) -> Vec<Action> {
        if self.state == State::Muted { return Vec::new(); }
        self.reset_to(State::Idle, Action::Idle)
    }

    fn loud(&self, frame: &Frame) -> bool {
        let power: u64 = frame.iter().map(|&v| {
            let n = i64::from(v);
            (n * n) as u64
        }).sum();
        power >= u64::from(self.threshold).pow(2) * FRAME_SAMPLES as u64
    }

    fn begin(&mut self) -> Vec<Action> {
        let Some(next) = self.next.checked_add(1) else { return self.stop(); };
        let id = self.next;
        self.next = next;
        let mut out = Vec::with_capacity(PREROLL + 2);
        if let Some(old) = self.active_id() { out.push(Action::Cancel(old)); }
        let pre = std::mem::take(&mut self.pre);
        self.set(State::Listening(id));
        out.push(Action::Start(id));
        out.extend(pre.into_iter().map(|f| Action::Feed(id, Box::new(f))));
        out
    }

    pub fn frame(&mut self, frame: Frame) -> Vec<Action> {
        if matches!(self.state, State::Idle | State::Muted) { return Vec::new(); }
        self.elapsed += 1;
        match self.state {
            State::Armed | State::Speaking(_) => {
                if self.state == State::Armed && self.elapsed >= FOLLOWUP {
                    return self.stop();
                }
                if matches!(self.state, State::Speaking(_)) {
                    if self.elapsed >= SPEAK_TIMEOUT { return self.stop(); }
                    // Do not interpret speaker echo as near-end speech by default.
                    if !self.aec_verified { return Vec::new(); }
                }
                self.voiced = if self.loud(&frame) { self.voiced + 1 } else { 0 };
                if self.pre.len() == PREROLL { self.pre.pop_front(); }
                self.pre.push_back(frame);
                if self.voiced >= ONSET { self.begin() } else { Vec::new() }
            }
            State::Listening(id) => {
                if self.elapsed >= MAX_TURN { return self.stop(); }
                self.silent = if self.loud(&frame) { 0 } else { self.silent + 1 };
                let mut out = vec![Action::Feed(id, Box::new(frame))];
                if self.silent >= END_SILENCE {
                    self.set(State::Finalizing(id));
                    out.push(Action::Finalize(id));
                }
                out
            }
            State::Finalizing(_) => {
                if self.elapsed >= FINAL_TIMEOUT { self.stop() } else { Vec::new() }
            }
            _ => Vec::new(),
        }
    }

    /// Only the final transcript from the committed endpoint is accepted.
    /// Partials and late finals cannot invoke inference.
    pub fn final_text(&mut self, id: VoiceId, text: &str) -> Vec<Action> {
        self.final_text_with(id, text, respond)
    }

    /// Invoke the host responder only after validating the final turn.
    /// The core does not depend on a database or provider implementation.
    pub fn final_text_with(
        &mut self, id: VoiceId, text: &str,
        reply: impl FnOnce(&mut MockInference, &str) -> String,
    ) -> Vec<Action> {
        if self.state != State::Finalizing(id) { return Vec::new(); }
        if text.len() > MAX_TRANSCRIPT_BYTES || text.trim().is_empty()
            || text.chars().any(|c| c.is_control())
        {
            return self.reset_to(State::Armed, Action::Armed);
        }
        let answer = reply(&mut self.provider, text);
        self.set(State::Speaking(id));
        vec![Action::Speak(id, answer)]
    }

    pub fn playback_done(&mut self, id: VoiceId) -> Vec<Action> {
        if self.state != State::Speaking(id) { return Vec::new(); }
        self.set(State::Armed);
        vec![Action::Armed]
    }
}
