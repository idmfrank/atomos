//! Single-owner conversation state. Epochs invalidate stale asynchronous work.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Resting,
    Listening,
    Responding,
    Muted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurnId(u64);

#[derive(Debug, PartialEq, Eq)]
pub enum TurnError {
    InvalidState,
    Stale,
    Empty,
    TooLong,
    Exhausted,
}

pub const MAX_TRANSCRIPT_BYTES: usize = 8192;

pub struct TurnManager {
    epoch: u64,
    state: State,
}

impl Default for TurnManager {
    fn default() -> Self {
        Self {
            epoch: 0,
            state: State::Resting,
        }
    }
}

impl TurnManager {
    pub fn state(&self) -> State {
        self.state
    }

    fn transition(&mut self, state: State) -> Result<TurnId, TurnError> {
        self.epoch = self.epoch.checked_add(1).ok_or(TurnError::Exhausted)?;
        self.state = state;
        Ok(TurnId(self.epoch))
    }

    pub fn wake(&mut self) -> Result<TurnId, TurnError> {
        if self.state != State::Resting {
            return Err(TurnError::InvalidState);
        }
        self.transition(State::Listening)
    }

    pub fn commit(&mut self, id: TurnId, text: &str) -> Result<(), TurnError> {
        if id.0 != self.epoch {
            return Err(TurnError::Stale);
        }
        if self.state != State::Listening {
            return Err(TurnError::InvalidState);
        }
        if text.trim().is_empty() {
            return Err(TurnError::Empty);
        }
        if text.len() > MAX_TRANSCRIPT_BYTES {
            return Err(TurnError::TooLong);
        }
        self.state = State::Responding;
        Ok(())
    }

    pub fn accepts_output(&self, id: TurnId) -> bool {
        self.state == State::Responding && self.epoch == id.0
    }

    pub fn interrupt(&mut self) -> Result<TurnId, TurnError> {
        if self.state != State::Responding {
            return Err(TurnError::InvalidState);
        }
        self.transition(State::Listening)
    }

    pub fn finish(&mut self, id: TurnId) -> Result<(), TurnError> {
        if !self.accepts_output(id) {
            return Err(TurnError::Stale);
        }
        self.transition(State::Resting)?;
        Ok(())
    }

    pub fn mute(&mut self) -> Result<(), TurnError> {
        self.transition(State::Muted)?;
        Ok(())
    }

    pub fn unmute(&mut self) -> Result<(), TurnError> {
        if self.state != State::Muted {
            return Err(TurnError::InvalidState);
        }
        self.transition(State::Resting)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_turn() {
        let mut manager = TurnManager::default();
        let id = manager.wake().unwrap();
        assert!(!manager.accepts_output(id));
        manager.commit(id, "hello").unwrap();
        assert!(manager.accepts_output(id));
        manager.finish(id).unwrap();
        assert_eq!(manager.state(), State::Resting);
        assert!(!manager.accepts_output(id));
    }

    #[test]
    fn interrupted_output_is_stale() {
        let mut manager = TurnManager::default();
        let old = manager.wake().unwrap();
        manager.commit(old, "hello").unwrap();
        let new = manager.interrupt().unwrap();
        assert_eq!(manager.finish(old), Err(TurnError::Stale));
        manager.commit(new, "actually").unwrap();
        assert!(manager.accepts_output(new));
        assert!(!manager.accepts_output(old));
    }

    #[test]
    fn mute_invalidates_turn_and_blocks_wake() {
        let mut manager = TurnManager::default();
        let id = manager.wake().unwrap();
        manager.mute().unwrap();
        assert_eq!(manager.wake(), Err(TurnError::InvalidState));
        assert_eq!(manager.commit(id, "hello"), Err(TurnError::Stale));
        manager.unmute().unwrap();
        assert!(manager.wake().is_ok());
    }

    #[test]
    fn empty_and_oversized_transcripts_do_not_advance_state() {
        let mut manager = TurnManager::default();
        let id = manager.wake().unwrap();
        assert_eq!(manager.commit(id, " "), Err(TurnError::Empty));
        assert_eq!(manager.commit(id, &"x".repeat(MAX_TRANSCRIPT_BYTES + 1)), Err(TurnError::TooLong));
        assert_eq!(manager.state(), State::Listening);
    }
}
