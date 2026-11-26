#![forbid(unsafe_code)]
use atomos_core::turn::{TurnManager, MAX_TRANSCRIPT_BYTES};
use atomos_core::{respond, MockInference};
use std::io::{self, BufRead, Read, Write};

// Limit raw bytes before trimming. Reject control characters except normal
// whitespace to avoid returning terminal control sequences from a transcript.
fn transcript(bytes: &[u8]) -> io::Result<&str> {
    let bytes = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    let bytes = bytes.strip_suffix(b"\r").unwrap_or(bytes);
    if bytes.len() > MAX_TRANSCRIPT_BYTES {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Input too long"));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid UTF-8"))?;
    if text.chars().any(|c| c.is_control() && !matches!(c, '\t' | '\n' | '\r')) {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid control character"));
    }
    Ok(text.trim())
}

fn answer(provider: &mut MockInference, turns: &mut TurnManager, line: &str) -> io::Result<String> {
    let invalid = |_| io::Error::new(io::ErrorKind::InvalidData, "Invalid conversation turn");
    let id = turns.wake().map_err(invalid)?;
    turns.commit(id, line).map_err(invalid)?;
    let response = respond(provider, line);
    turns.finish(id).map_err(invalid)?;
    Ok(response)
}

fn main() -> io::Result<()> {
    let mut provider = MockInference::default();
    let mut turns = TurnManager::default();
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut out = io::stdout().lock();
    let args: Vec<_> = std::env::args_os().skip(1).collect();

    // Plain response only: the local audio harness uses this interface.
    if args.len() == 1 && args[0] == "--single-turn" {
        let mut bytes = Vec::new();
        (&mut input).take((MAX_TRANSCRIPT_BYTES + 3) as u64).read_to_end(&mut bytes)?;
        let line = transcript(&bytes)?;
        if line.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Empty transcript"));
        }
        writeln!(out, "{}", answer(&mut provider, &mut turns, line)?)?;
        return Ok(());
    }
    if !args.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Usage: atomos [--single-turn]"));
    }

    writeln!(out, "AtomOS M1: offline mock (no OpenAI calls)")?;
    loop {
        let mut bytes = Vec::new();
        let read = (&mut input).take((MAX_TRANSCRIPT_BYTES + 3) as u64).read_until(b'\n', &mut bytes)?;
        if read == 0 {
            break;
        }
        // An overlong or unterminated capped line is never interpreted in pieces.
        let line = match transcript(&bytes) {
            Ok(s) => s,
            Err(error) => {
                writeln!(out, "{error}; ending session.")?;
                break;
            }
        };
        if line == "/quit" {
            break;
        }
        if line.is_empty() {
            continue;
        }
        writeln!(out, "{}", answer(&mut provider, &mut turns, line)?)?;
        out.flush()?;
    }
    writeln!(out, "Simulated inference calls: {}", provider.calls)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_limit_is_enforced_before_trimming() {
        let mut long = vec![b' '; MAX_TRANSCRIPT_BYTES + 1];
        long.push(b'\n');
        assert!(transcript(&long).is_err());
        let mut exact = vec![b'x'; MAX_TRANSCRIPT_BYTES];
        exact.extend_from_slice(b"\r\n");
        assert_eq!(transcript(&exact).unwrap().len(), MAX_TRANSCRIPT_BYTES);
    }

    #[test]
    fn controls_and_invalid_utf8_are_rejected() {
        assert!(transcript(b"hi\x1b[2J").is_err());
        assert!(transcript(&[0xff]).is_err());
        assert_eq!(transcript(b" hello\r\n").unwrap(), "hello");
    }
}
