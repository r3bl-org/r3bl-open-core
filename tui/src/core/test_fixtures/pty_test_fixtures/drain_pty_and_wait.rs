// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{ControlledChild, ControllerReader, PtyPair};
use std::io::{BufReader, Read};

/// Drains the PTY and waits for the child process to exit, preventing deadlocks.
///
/// This function solves a PTY buffer deadlock that occurs on macOS (and occasionally
/// on Linux) when a controlled process writes to stderr after the controller has
/// stopped reading. The sequence that causes the deadlock:
///
/// 1. Controller reads PTY until a marker (e.g., `SUCCESS`, `CONTROLLED_DONE`)
/// 2. Controller stops reading and calls `child.wait()`
/// 3. Child writes more `eprintln!()` after the marker, then calls
///    `std::process::exit(0)`
/// 4. `exit()` flushes stdio, which **blocks** because the PTY buffer is full (nobody is
///    reading the controller side)
/// 5. Deadlock: controller waits for child, child waits for buffer space
///
/// macOS PTY buffers are ~1 KB (vs ~4 KB on Linux), making this trigger frequently.
///
/// # Solution
///
/// 1. **Drop `pty_pair`** — closes the parent's handle to the controlled fd. The
///    `buf_reader`'s cloned controller fd remains valid.
/// 2. **Drain `buf_reader` until EOF** — unblocks the child's `exit()` flush. Once the
///    child exits, the controlled fd closes, which gives the controller EOF.
/// 3. **`child.wait()`** — the child has already exited, so this reaps the zombie
///    immediately.
///
/// # Parameters
///
/// - `buf_reader` - The buffered reader wrapping a cloned controller reader. Must be the
///   same reader used during the test's read loop (so buffered data is consumed).
/// - `pty_pair` - The PTY pair to drop (closes parent's controlled fd).
/// - `child` - The controlled child process to wait on.
///
/// # Panics
///
/// Panics if `child.wait()` fails.
pub fn drain_pty_and_wait(
    mut buf_reader: BufReader<ControllerReader>,
    pty_pair: PtyPair,
    child: &mut ControlledChild,
) {
    // Step 1: Drop pty_pair to close the parent's handle to the controlled fd.
    // The buf_reader's cloned controller fd remains valid.
    drop(pty_pair);

    // Step 2: Drain buf_reader until EOF. This unblocks the child's exit() flush.
    // Once the child exits and its controlled fd closes, we get EOF here.
    let mut discard_buf = [0u8; 1024];
    loop {
        match buf_reader.read(&mut discard_buf) {
            Ok(0) => break,    // EOF — child exited and controlled fd closed.
            Ok(_) => continue, // Discard remaining output.
            Err(e) => {
                // EIO is expected on some platforms when the controlled side closes.
                if e.raw_os_error() == Some(5) {
                    break;
                }
                // Other errors are unexpected but not fatal — the child may have
                // already exited.
                eprintln!("drain_pty_and_wait: read error during drain: {e}");
                break;
            }
        }
    }

    // Step 3: Reap the child process. It has already exited, so this returns
    // immediately.
    match child.wait() {
        Ok(status) => {
            eprintln!("✅ drain_pty_and_wait: child exited: {status:?}");
        }
        Err(e) => {
            panic!("drain_pty_and_wait: failed to wait for child: {e}");
        }
    }
}
