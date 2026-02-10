// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{ControlledChild, PtyPair};

use super::normalize_pty_output::normalize_pty_line;

/// Result of reading PTY output lines until a sentinel.
#[derive(Debug)]
pub struct ReadLinesResult {
    /// Collected output lines (normalized, filtered).
    pub lines: Vec<String>,
    /// Whether the sentinel string was found in the output.
    pub found_sentinel: bool,
}

/// Reads lines from a PTY controller until a sentinel string is found, then
/// cleans up and waits for the child to exit.
///
/// Both platforms use a simple blocking `read_line()` loop — no background
/// threads, polling, or timeouts are needed. The **only** platform difference
/// is a handshake that Windows (ConPTY) requires before any output flows:
///
/// ## The ConPTY DSR handshake (Windows only)
///
/// When a ConPTY session starts, the `OpenConsole.exe` broker sends a **Device
/// Status Report** request (`\x1b[6n` — "report cursor position") to the host
/// and **blocks ALL child stdout forwarding** until the host replies with
/// `\x1b[row;colR`. In production code, crossterm's terminal emulator handles
/// this transparently. In raw PTY tests (which bypass crossterm), we must
/// respond manually. Without the response, zero bytes arrive on the reader —
/// the child's output is buffered inside ConPTY indefinitely.
///
/// Once the DSR response is sent, ConPTY behaves like a Unix PTY: blocking
/// reads work, and EOF is delivered when the child exits.
///
/// There is one additional ConPTY quirk: the **writer (input pipe) must stay
/// alive** until reading completes. ConPTY treats input and output as a single
/// console session — dropping the writer closes the session and stops stdout
/// forwarding even though the child is still running.
///
/// ## Platform-specific cleanup
///
/// - **Unix**: Calls [`drain_pty_and_wait`] after reading to prevent macOS
///   PTY buffer deadlock (the PTY master must be drained before `waitpid`).
/// - **Windows**: Drops writer and controller to close the ConPTY session,
///   then reaps the child with `wait()`.
///
/// # Parameters
///
/// - `pty_pair` — Owned PTY pair (consumed for cleanup).
/// - `child` — The controlled child process handle.
/// - `sentinel` — A substring to watch for. Reading stops when found.
/// - `line_filter` — Predicate on each normalized line. Lines returning
///   `false` are skipped.
///
/// [`drain_pty_and_wait`]: crate::drain_pty_and_wait
pub fn read_lines_and_drain(
    pty_pair: PtyPair,
    child: &mut ControlledChild,
    sentinel: &str,
    line_filter: impl Fn(&str) -> bool,
) -> ReadLinesResult {
    #[cfg(not(target_os = "windows"))]
    {
        read_lines_unix(pty_pair, child, sentinel, line_filter)
    }
    #[cfg(target_os = "windows")]
    {
        read_lines_windows(pty_pair, child, sentinel, line_filter)
    }
}

// ── Unix implementation ──────────────────────────────────────────────

#[cfg(not(target_os = "windows"))]
fn read_lines_unix(
    pty_pair: PtyPair,
    child: &mut ControlledChild,
    sentinel: &str,
    line_filter: impl Fn(&str) -> bool,
) -> ReadLinesResult {
    use crate::drain_pty_and_wait;
    use std::io::BufReader;

    let reader = pty_pair
        .controller()
        .try_clone_reader()
        .expect("Failed to clone reader");
    let mut buf_reader = BufReader::new(reader);

    let (lines, found_sentinel) = read_until_sentinel(&mut buf_reader, sentinel, &line_filter);

    drain_pty_and_wait(buf_reader, pty_pair, child);

    ReadLinesResult {
        lines,
        found_sentinel,
    }
}

// ── Windows implementation ───────────────────────────────────────────

#[cfg(target_os = "windows")]
fn read_lines_windows(
    pty_pair: PtyPair,
    child: &mut ControlledChild,
    sentinel: &str,
    line_filter: impl Fn(&str) -> bool,
) -> ReadLinesResult {
    use std::io::{BufReader, Read, Write};

    // Split the PtyPair. The controlled (slave) half must be dropped after
    // spawning; the child retains its own console handle.
    let (controller, controlled) = pty_pair.split();
    drop(controlled);

    let mut reader = controller
        .try_clone_reader()
        .expect("Failed to clone reader");

    // ConPTY DSR handshake: ConPTY sends `\x1b[6n` (Report Cursor Position)
    // during initialization and blocks ALL child stdout forwarding until the
    // host responds with `\x1b[row;colR`. Without this, zero bytes arrive.
    //
    // The writer must stay alive until after reading completes — ConPTY
    // treats input+output as a single console session, so dropping the
    // writer (input pipe) closes the session and stops stdout forwarding.
    let _writer = {
        let mut writer = controller
            .take_writer()
            .expect("Failed to take writer");
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(n) if n > 0 => {
                    if buf[..n].windows(4).any(|w| w == b"\x1b[6n") {
                        let _unused = writer.write_all(b"\x1b[1;1R");
                        let _unused = writer.flush();
                        break;
                    }
                }
                _ => break,
            }
        }
        writer
    };

    let mut buf_reader = BufReader::new(reader);

    let (lines, found_sentinel) = read_until_sentinel(&mut buf_reader, sentinel, &line_filter);

    // Drop writer and controller to close the ConPTY session, then reap child.
    drop(_writer);
    drop(controller);
    match child.wait() {
        Ok(status) => eprintln!("✅ read_lines_and_drain: child exited: {status:?}"),
        Err(e) => eprintln!("read_lines_and_drain: wait error: {e}"),
    }

    ReadLinesResult {
        lines,
        found_sentinel,
    }
}

// ── Shared read loop ─────────────────────────────────────────────────

fn read_until_sentinel(
    buf_reader: &mut impl std::io::BufRead,
    sentinel: &str,
    line_filter: &impl Fn(&str) -> bool,
) -> (Vec<String>, bool) {
    let mut lines = Vec::new();
    let mut found_sentinel = false;

    loop {
        let mut line = String::new();
        match buf_reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = normalize_pty_line(&line);
                eprintln!("  <- Controlled output: {trimmed:?}");

                if trimmed.contains(sentinel) {
                    found_sentinel = true;
                    break;
                }

                if !trimmed.is_empty() && line_filter(&trimmed) {
                    lines.push(trimmed);
                }
            }
            Err(e) => {
                eprintln!("read_lines_and_drain: read error: {e}");
                break;
            }
        }
    }

    (lines, found_sentinel)
}
