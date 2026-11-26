//! Audio contract and bounded simulation; not a physical audio driver.
use std::collections::VecDeque;

use crate::turn::{TurnId, TurnManager};

pub const MAX_FRAME_SAMPLES: usize = 960; // 20 ms at 48 kHz mono.

#[derive(Debug, PartialEq, Eq)]
pub enum AudioError {
    Full,
    Stale,
    InvalidFrame,
}

pub trait SpeechRecognizer {
    fn push_frame(&mut self, pcm: &[i16]);
    fn finish(&mut self) -> Option<String>;
    fn cancel(&mut self);
}

pub trait SpeechSynthesizer {
    fn start(&mut self, id: TurnId, text: &str);
    fn cancel(&mut self);
}

pub trait AudioSink {
    fn stop_and_flush(&mut self);
}

pub struct PlaybackQueue {
    frames: VecDeque<(TurnId, Vec<i16>)>,
    capacity: usize,
}

impl PlaybackQueue {
    pub fn new(capacity: usize) -> Self {
        Self {
            frames: VecDeque::new(),
            capacity: capacity.clamp(1, 32),
        }
    }

    pub fn enqueue(
        &mut self,
        turns: &TurnManager,
        id: TurnId,
        pcm: Vec<i16>,
    ) -> Result<(), AudioError> {
        if !turns.accepts_output(id) {
            return Err(AudioError::Stale);
        }
        if pcm.is_empty() || pcm.len() > MAX_FRAME_SAMPLES {
            return Err(AudioError::InvalidFrame);
        }
        if self.frames.len() >= self.capacity {
            return Err(AudioError::Full);
        }
        self.frames.push_back((id, pcm));
        Ok(())
    }

    pub fn next(&mut self, turns: &TurnManager) -> Option<Vec<i16>> {
        while let Some((id, pcm)) = self.frames.pop_front() {
            if turns.accepts_output(id) {
                return Some(pcm);
            }
        }
        None
    }
}

impl AudioSink for PlaybackQueue {
    fn stop_and_flush(&mut self) {
        self.frames.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_is_bounded_and_rejects_stale_frames() {
        let mut turns = TurnManager::default();
        let old = turns.wake().unwrap();
        turns.commit(old, "hello").unwrap();
        let mut queue = PlaybackQueue::new(1);
        assert_eq!(queue.enqueue(&turns, old, vec![1]), Ok(()));
        assert_eq!(queue.enqueue(&turns, old, vec![2]), Err(AudioError::Full));
        let new = turns.interrupt().unwrap();
        queue.stop_and_flush();
        assert!(queue.next(&turns).is_none());
        turns.commit(new, "wait").unwrap();
        assert_eq!(queue.enqueue(&turns, old, vec![3]), Err(AudioError::Stale));
        assert_eq!(queue.enqueue(&turns, new, vec![4]), Ok(()));
        assert_eq!(queue.next(&turns), Some(vec![4]));
    }

    #[test]
    fn stale_buffer_is_discarded_even_without_explicit_flush() {
        let mut turns = TurnManager::default();
        let id = turns.wake().unwrap();
        turns.commit(id, "hello").unwrap();
        let mut queue = PlaybackQueue::new(2);
        queue.enqueue(&turns, id, vec![1]).unwrap();
        turns.mute().unwrap();
        assert_eq!(queue.next(&turns), None);
    }

    #[test]
    fn oversized_frame_rejected() {
        let mut turns = TurnManager::default();
        let id = turns.wake().unwrap();
        turns.commit(id, "hello").unwrap();
        let mut queue = PlaybackQueue::new(1);
        assert_eq!(queue.enqueue(&turns, id, vec![0; MAX_FRAME_SAMPLES + 1]), Err(AudioError::InvalidFrame));
    }
}
