#![forbid(unsafe_code)]
//! Private stdio supervisor, not a server or general command runner.
use atomos_tasks::{
    worker::{Command, Handle, Response, Worker, POLICY},
    Runtime,
};
use std::io::{self, BufRead, Read, Write};
use std::os::fd::AsFd;
use std::path::PathBuf;
use std::time::Duration;

const LINE: usize = 16_640;
fn unhex(s: &str) -> Option<String> {
    if s.is_empty() || s.len() > 16_384 || s.len() % 2 != 0 {
        return None;
    }
    let bytes = s
        .as_bytes()
        .chunks_exact(2)
        .map(|p| {
            let h = (p[0] as char).to_digit(16)?;
            let l = (p[1] as char).to_digit(16)?;
            Some((h * 16 + l) as u8)
        })
        .collect::<Option<Vec<_>>>()?;
    String::from_utf8(bytes).ok()
}
fn parse(s: &str) -> Option<Command> {
    let parts: Vec<_> = s.split_ascii_whitespace().collect();
    match parts.as_slice() {
        ["SUBMIT", key, goal] => Some(Command::Submit {
            key: (*key).into(),
            goal: unhex(goal)?,
        }),
        ["STATUS", id] => Some(Command::Status(id.parse().ok()?)),
        ["CANCEL", id] => Some(Command::Cancel(id.parse().ok()?)),
        ["RESUME", id] => Some(Command::Resume(id.parse().ok()?)),
        ["EVENTS", after, limit] => Some(Command::Events {
            after: after.parse().ok()?,
            limit: limit.parse().ok()?,
        }),
        _ => None,
    }
}
fn input(handle: Handle) {
    let mut stdin = io::stdin().lock();
    let result = (|| -> io::Result<()> {
        // A private duplicate FD avoids holding the global stdout lock while
        // reading input or blocking on a slow consumer. Main never writes here
        // after READY; successful process exit is the shutdown acknowledgement.
        let mut stdout = std::fs::File::from(io::stdout().as_fd().try_clone_to_owned()?);
        loop {
            let mut line = Vec::new();
            let n = (&mut stdin)
                .take((LINE + 1) as u64)
                .read_until(b'\n', &mut line)?;
            if n == 0 {
                break;
            }
            if line.len() > LINE {
                writeln!(stdout, "ERROR Invalid")?;
                stdout.flush()?;
                break;
            }
            let text = match std::str::from_utf8(&line) {
                Ok(v) => v.trim_end_matches(['\r', '\n']),
                Err(_) => {
                    writeln!(stdout, "ERROR Invalid")?;
                    stdout.flush()?;
                    continue;
                }
            };
            if text == "STOP" {
                break;
            }
            if let Some(command) = parse(text) {
                match handle.request(command) {
                    Ok(Response::Task(t)) => writeln!(
                        stdout,
                        "TASK {} {:?} dispatches={} failure={}",
                        t.id,
                        t.state,
                        t.dispatches,
                        t.failure.as_deref().unwrap_or("none")
                    )?,
                    Ok(Response::Events(events)) => {
                        for e in &events {
                            writeln!(stdout, "EVENT {} {} {:?}", e.cursor, e.task_id, e.state)?;
                        }
                        writeln!(stdout, "END {}", events.len())?;
                    }
                    Err(e) => writeln!(stdout, "ERROR {e:?}")?,
                }
            } else {
                writeln!(stdout, "ERROR Invalid")?;
            }
            stdout.flush()?;
        }
        Ok(())
    })();
    let _ = result; // No raw input/path or OS diagnostic in logs.
    handle.request_stop();
}
fn run() -> Result<(), ()> {
    let a: Vec<_> = std::env::args_os().skip(1).collect();
    // Validate all options before opening files. No implicit path/owner/store init.
    if a.len() < 4 || a[0] != "--store" || a[2] != "--owner" {
        return Err(());
    }
    let path = PathBuf::from(&a[1]);
    let owner = a[3].to_str().ok_or(())?;
    let mut init = None;
    let mut daemon = false;
    let mut delay = 0;
    let mut i = 4;
    while i < a.len() {
        match a[i].to_str().ok_or(())? {
            "--init-bytes" if init.is_none() && i + 1 < a.len() => {
                i += 1;
                init = Some(a[i].to_str().ok_or(())?.parse::<u64>().map_err(|_| ())?);
            }
            "--daemon" if !daemon => daemon = true,
            "--fixture-delay-ms" if delay == 0 && i + 1 < a.len() => {
                i += 1;
                delay = a[i].to_str().ok_or(())?.parse::<u64>().map_err(|_| ())?;
                if delay == 0 || delay > 30_000 {
                    return Err(());
                }
            }
            _ => return Err(()),
        }
        i += 1;
    }
    if let Some(bytes) = init {
        drop(Runtime::initialize(&path, owner, POLICY, bytes).map_err(|_| ())?);
    }
    let worker = Worker::start(&path, owner, Duration::from_millis(delay)).map_err(|_| ())?;
    let handle = worker.handle();
    let stop = handle.stop_flag();
    let term =
        signal_hook::flag::register(signal_hook::consts::SIGTERM, stop.clone()).map_err(|_| ())?;
    let int = match signal_hook::flag::register(signal_hook::consts::SIGINT, stop) {
        Ok(id) => id,
        Err(_) => {
            signal_hook::low_level::unregister(term);
            return Err(());
        }
    };
    println!("READY synthetic=1");
    io::stdout().flush().map_err(|_| ())?;
    if !daemon {
        std::thread::spawn(move || input(handle));
    }
    let result = worker.join().map_err(|_| ());
    signal_hook::low_level::unregister(term);
    signal_hook::low_level::unregister(int);
    result?;
    Ok(())
}
fn main() {
    if run().is_err() {
        eprintln!("Task service failed; inspect private configuration/state without sharing its contents.");
        std::process::exit(1);
    }
}
