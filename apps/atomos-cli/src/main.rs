#![forbid(unsafe_code)]
use atomos_core::turn::{TurnManager, MAX_TRANSCRIPT_BYTES};
use atomos_core::{respond, MockInference};
use std::io::{self, BufRead, Read, Write};

fn main() -> io::Result<()> {
    let mut provider = MockInference::default();
    let mut turns = TurnManager::default();
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut out = io::stdout().lock();
    writeln!(out, "AtomOS M1: offline mock (no OpenAI calls)")?;
    loop {
        // Bound input allocation even if stdin is a pipe with no line terminator.
        let mut bytes = Vec::new();
        let read = (&mut input).take((MAX_TRANSCRIPT_BYTES + 2) as u64).read_until(b'\n', &mut bytes)?;
        if read == 0 {
            break;
        }
        if bytes.len() > MAX_TRANSCRIPT_BYTES && !bytes.ends_with(b"\n") {
            writeln!(out, "Input too long; ending session.")?;
            break;
        }
        let line = match std::str::from_utf8(&bytes) {
            Ok(s) => s.trim(),
            Err(_) => {
                writeln!(out, "Invalid UTF-8.")?;
                continue;
            }
        };
        if line == "/quit" {
            break;
        }
        if line.is_empty() {
            continue;
        }
        if line.len() > MAX_TRANSCRIPT_BYTES {
            writeln!(out, "Input too long.")?;
            continue;
        }
        let id = turns.wake().expect("valid demo transition");
        turns.commit(id, line).expect("validated transcript");
        writeln!(out, "{}", respond(&mut provider, line))?;
        turns.finish(id).expect("valid demo transition");
        out.flush()?;
    }
    writeln!(out, "Simulated inference calls: {}", provider.calls)?;
    Ok(())
}
