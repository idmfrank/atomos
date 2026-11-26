//! Bounded local stdio protocol for a trusted audio worker.
//! ASCII hex prevents newline/control injection. No network or credential access.
use atomos_core::voice::{Action, VoiceSession, FRAME_SAMPLES};
use std::io::{self, BufRead, Read, Write};

const MAX_LINE: usize = 16500;

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(DIGITS[(b >> 4) as usize] as char);
        out.push(DIGITS[(b & 15) as usize] as char);
    }
    out
}

fn unhex(text: &str) -> Option<Vec<u8>> {
    fn digit(b: u8) -> Option<u8> {
        match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            _ => None,
        }
    }
    if text.len() % 2 != 0 { return None; }
    text.as_bytes().chunks_exact(2).map(|p| Some(digit(p[0])? * 16 + digit(p[1])?)).collect()
}

fn invalid() -> io::Error { io::Error::new(io::ErrorKind::InvalidData, "Invalid voice event") }

fn dispatch(session: &mut VoiceSession, line: &str) -> io::Result<Vec<Action>> {
    let mut fields = line.split(' ');
    let cmd = fields.next().ok_or_else(invalid)?;
    let actions = match cmd {
        "ARM" => session.arm(),
        "MUTE" => session.mute(),
        "UNMUTE" => session.unmute(),
        "STOP" => session.stop(),
        "FRAME" => {
            let raw = unhex(fields.next().ok_or_else(invalid)?).ok_or_else(invalid)?;
            if raw.len() != FRAME_SAMPLES * 2 { return Err(invalid()); }
            let mut pcm = [0; FRAME_SAMPLES];
            for (sample, chunk) in pcm.iter_mut().zip(raw.chunks_exact(2)) {
                *sample = i16::from_le_bytes([chunk[0], chunk[1]]);
            }
            if fields.next().is_some() { return Err(invalid()); }
            return Ok(session.frame(pcm));
        }
        "FINAL" => {
            let id = fields.next().ok_or_else(invalid)?.parse().map_err(|_| invalid())?;
            let raw = unhex(fields.next().ok_or_else(invalid)?).ok_or_else(invalid)?;
            let text = std::str::from_utf8(&raw).map_err(|_| invalid())?;
            if fields.next().is_some() { return Err(invalid()); }
            return Ok(session.final_text(id, text));
        }
        "DONE" => {
            let id = fields.next().ok_or_else(invalid)?.parse().map_err(|_| invalid())?;
            if fields.next().is_some() { return Err(invalid()); }
            return Ok(session.playback_done(id));
        }
        _ => return Err(invalid()),
    };
    if fields.next().is_some() { return Err(invalid()); }
    Ok(actions)
}

fn emit(out: &mut impl Write, actions: Vec<Action>) -> io::Result<()> {
    for action in actions {
        match action {
            Action::Start(id) => writeln!(out, "START {id}")?,
            Action::Feed(id, frame) => {
                let bytes: Vec<_> = frame.iter().flat_map(|n| n.to_le_bytes()).collect();
                writeln!(out, "PCM {id} {}", hex(&bytes))?;
            }
            Action::Finalize(id) => writeln!(out, "FINALIZE {id}")?,
            Action::Speak(id, text) => writeln!(out, "SPEAK {id} {}", hex(text.as_bytes()))?,
            Action::Cancel(id) => writeln!(out, "CANCEL {id}")?,
            Action::Idle => writeln!(out, "IDLE")?,
            Action::Armed => writeln!(out, "ARMED")?,
            Action::Muted => writeln!(out, "MUTED")?,
        }
    }
    writeln!(out, ".")?;
    out.flush()
}

pub fn run(aec_verified: bool) -> io::Result<()> {
    let mut session = VoiceSession::new(500, aec_verified);
    let mut input = io::stdin().lock();
    let mut out = io::stdout().lock();
    loop {
        let mut bytes = Vec::new();
        let n = (&mut input).take((MAX_LINE + 1) as u64).read_until(b'\n', &mut bytes)?;
        if n == 0 { return Ok(()); }
        if n > MAX_LINE || bytes.last() != Some(&b'\n') { return Err(invalid()); }
        bytes.pop();
        let line = std::str::from_utf8(&bytes).map_err(|_| invalid())?;
        if line == "QUIT" {
            emit(&mut out, session.stop())?;
            return Ok(());
        }
        let actions = dispatch(&mut session, line)?;
        emit(&mut out, actions)?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip_and_invalid_input() {
        let bytes = [0, 1, 15, 16, 255];
        assert_eq!(unhex(&hex(&bytes)), Some(bytes.to_vec()));
        assert!(unhex("0").is_none());
        assert!(unhex("ZZ").is_none());
    }

    #[test]
    fn rejects_malformed_or_extra_fields() {
        let mut s = VoiceSession::new(500, false);
        assert!(dispatch(&mut s, "FRAME 00").is_err());
        assert!(dispatch(&mut s, "FINAL x 00").is_err());
        assert!(dispatch(&mut s, "DONE 1 extra").is_err());
        assert!(dispatch(&mut s, "ARM extra").is_err());
    }
}
