use atomos_core::voice::{Action, Frame, State, VoiceId, VoiceSession, FRAME_SAMPLES};

fn loud() -> Frame { [2000; FRAME_SAMPLES] }
fn quiet() -> Frame { [0; FRAME_SAMPLES] }

fn onset(s: &mut VoiceSession) -> (VoiceId, Vec<Action>) {
    assert!(s.frame(loud()).is_empty());
    assert!(s.frame(loud()).is_empty());
    let out = s.frame(loud());
    let id = out.iter().find_map(|a| if let Action::Start(id) = a { Some(*id) } else { None }).unwrap();
    (id, out)
}
fn endpoint(s: &mut VoiceSession, id: VoiceId) {
    for _ in 0..14 {
        assert_eq!(s.frame(quiet()), vec![Action::Feed(id, Box::new(quiet()))]);
    }
    assert_eq!(s.frame(quiet()), vec![Action::Feed(id, Box::new(quiet())), Action::Finalize(id)]);
}
fn speak(s: &mut VoiceSession) -> VoiceId {
    s.arm();
    let (id, _) = onset(s);
    endpoint(s, id);
    assert_eq!(s.final_text(id, "what is your name"), vec![Action::Speak(id, "I am AtomOS.".into())]);
    id
}

#[test]
fn idle_and_muted_frames_do_not_trigger_recognition() {
    let mut s = VoiceSession::new(500, true);
    for _ in 0..500 { assert!(s.frame(loud()).is_empty()); }
    s.mute();
    assert!(s.arm().is_empty());
    assert!(s.frame(loud()).is_empty());
    s.unmute();
    assert_eq!(s.state(), State::Idle);
    assert_eq!(s.mock_calls(), 0);
}

#[test]
fn preroll_is_bounded_and_endpoint_is_300ms() {
    let mut s = VoiceSession::new(500, false);
    s.arm();
    for _ in 0..10 { s.frame(quiet()); }
    let (id, actions) = onset(&mut s);
    assert_eq!(actions.len(), 6);
    assert_eq!(actions[0], Action::Start(id));
    assert_eq!(actions[1], Action::Feed(id, Box::new(quiet())));
    assert_eq!(actions[5], Action::Feed(id, Box::new(loud())));
    assert!(s.final_text(id, "premature").is_empty());
    endpoint(&mut s, id);
    assert_eq!(s.mock_calls(), 0);
    assert_eq!(s.final_text(id, "what is your name"), vec![Action::Speak(id, "I am AtomOS.".into())]);
    assert_eq!(s.mock_calls(), 0);
    assert!(s.final_text(id, "duplicate").is_empty());
}

#[test]
fn short_pause_does_not_commit() {
    let mut s = VoiceSession::new(500, false);
    s.arm();
    let (id, _) = onset(&mut s);
    for _ in 0..14 { s.frame(quiet()); }
    s.frame(loud());
    assert_eq!(s.state(), State::Listening(id));
    endpoint(&mut s, id);
    assert_eq!(s.final_text(id, "explain entropy"), vec![Action::Speak(id, "Mock response: explain entropy".into())]);
    assert_eq!(s.mock_calls(), 1);
}

#[test]
fn verified_aec_barge_in_cancels_old_epoch_before_starting_new() {
    let mut s = VoiceSession::new(500, true);
    let old = speak(&mut s);
    let (new, actions) = onset(&mut s);
    assert_ne!(old, new);
    assert_eq!(actions[0], Action::Cancel(old));
    assert_eq!(actions[1], Action::Start(new));
    assert!(s.playback_done(old).is_empty());
    assert!(s.final_text(old, "late").is_empty());
    endpoint(&mut s, new);
    assert_eq!(s.final_text(new, "what is your name"), vec![Action::Speak(new, "I am AtomOS.".into())]);
}

#[test]
fn unverified_echo_never_triggers_barge_in() {
    let mut s = VoiceSession::new(500, false);
    let id = speak(&mut s);
    for _ in 0..100 { assert!(s.frame(loud()).is_empty()); }
    assert_eq!(s.state(), State::Speaking(id));
    assert_eq!(s.playback_done(id), vec![Action::Armed]);
    let (next, _) = onset(&mut s);
    assert_ne!(id, next);
}

#[test]
fn empty_control_and_oversized_finals_are_discarded() {
    let mut s = VoiceSession::new(500, false);
    s.arm();
    for text in [" ".to_owned(), "x\u{1b}".into(), "x".repeat(8193)] {
        let (id, _) = onset(&mut s);
        endpoint(&mut s, id);
        assert_eq!(s.final_text(id, &text), vec![Action::Cancel(id), Action::Armed]);
        assert_eq!(s.mock_calls(), 0);
    }
}

#[test]
fn timeout_and_device_stop_never_commit_truncated_turn() {
    let mut s = VoiceSession::new(500, false);
    s.arm();
    let (id, _) = onset(&mut s);
    for _ in 0..1499 { s.frame(loud()); }
    assert_eq!(s.frame(loud()), vec![Action::Cancel(id), Action::Idle]);
    assert!(s.final_text(id, "late").is_empty());
    s.arm();
    let (id, _) = onset(&mut s);
    endpoint(&mut s, id);
    for _ in 0..249 { assert!(s.frame(quiet()).is_empty()); }
    assert_eq!(s.frame(quiet()), vec![Action::Cancel(id), Action::Idle]);
    s.arm();
    let (id, _) = onset(&mut s);
    assert_eq!(s.stop(), vec![Action::Cancel(id), Action::Idle]);
    assert_eq!(s.mock_calls(), 0);
}

#[test]
fn mute_invalidates_active_speech_and_followup_expires() {
    let mut s = VoiceSession::new(500, false);
    let id = speak(&mut s);
    assert_eq!(s.mute(), vec![Action::Cancel(id), Action::Muted]);
    assert!(s.playback_done(id).is_empty());
    assert!(s.frame(loud()).is_empty());
    s.unmute();
    s.arm();
    for _ in 0..399 { assert!(s.frame(quiet()).is_empty()); }
    assert_eq!(s.frame(quiet()), vec![Action::Idle]);
}

#[test]
fn loudness_calculation_handles_full_scale() {
    let mut s = VoiceSession::new(32768, false);
    s.arm();
    for _ in 0..2 { assert!(s.frame([i16::MIN; FRAME_SAMPLES]).is_empty()); }
    assert!(matches!(s.frame([i16::MIN; FRAME_SAMPLES])[0], Action::Start(_)));
}
