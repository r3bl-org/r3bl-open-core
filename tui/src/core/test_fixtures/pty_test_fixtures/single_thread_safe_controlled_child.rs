// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words errno

use crate::{ControlledChild, ControlledChildTerminationHandle, ControllerReader, PtyPair};
use std::io::{BufReader, Read};

/// Wraps [`ControlledChild`] to prevent the [`PTY`] buffer deadlock described in
/// [`drain_pty_and_wait`].
///
/// Hides [`wait()`] entirely. The only exit path is [`drain_and_wait()`], which
/// delegates to [`drain_pty_and_wait`] to drain the buffer before reaping the child.
///
/// [`drain_and_wait()`]: Self::drain_and_wait
/// [`drain_pty_and_wait`]: crate::drain_pty_and_wait
/// [`PTY`]: crate::core::pty
/// [`wait()`]: portable_pty::Child::wait
#[allow(missing_debug_implementations)]
pub struct SingleThreadSafeControlledChild {
    child: ControlledChild,
}

impl SingleThreadSafeControlledChild {
    #[must_use]
    pub fn new(child: ControlledChild) -> Self { Self { child } }

    /// Returns a handle that can terminate the child process from another thread.
    ///
    /// The [`generate_pty_test!`] macro passes this handle to [`PtyTestWatchdog`], which
    /// terminates the child process if the controller hangs past the timeout.
    ///
    /// [`generate_pty_test!`]: crate::generate_pty_test
    /// [`PtyTestWatchdog`]: crate::PtyTestWatchdog
    #[must_use]
    pub fn clone_termination_handle(&self) -> ControlledChildTerminationHandle {
        self.child.clone_killer()
    }

    /// Delegates to [`drain_pty_and_wait`], taking ownership of `self`, `buf_reader`,
    /// and `pty_pair` so all [`PTY`] resources are cleaned up in one call.
    ///
    /// [`drain_pty_and_wait`]: crate::drain_pty_and_wait
    /// [`PTY`]: crate::core::pty
    pub fn drain_and_wait(
        mut self,
        buf_reader: BufReader<ControllerReader>,
        pty_pair: PtyPair,
    ) {
        drain_pty_and_wait(buf_reader, pty_pair, &mut self.child);
    }
}

/// Drains the [`PTY`] until [`EIO`] or [`EOF`], then waits for the child process to
/// exit. Prevents deadlocks caused by unread [`PTY`] buffer data.
///
/// This function solves a [`PTY`] buffer deadlock that occurs on macOS (and occasionally
/// on Linux) when a controlled process writes to stderr after the controller has
/// stopped reading. The sequence that causes the deadlock:
///
/// 1. Controller reads [`PTY`] until a marker (e.g., `SUCCESS`, `CONTROLLED_DONE`)
/// 2. Controller stops reading and calls `child.wait()`
/// 3. Child writes more `eprintln!()` after the marker, then calls
///    `std::process::exit(0)`
/// 4. `exit()` flushes stdio, which **blocks** because the [`PTY`] buffer is full (nobody
///    is reading the controller side)
/// 5. Deadlock: controller waits for child, child waits for buffer space
///
/// macOS [`PTY`] buffers are ~1 KB (vs ~4 KB on Linux), making this trigger frequently.
///
/// # Solution
///
/// 1. **Drop `pty_pair`** — closes the parent's controller fd. The `buf_reader`'s cloned
///    controller fd remains valid. The controlled fd must already be closed by the caller
///    (via [`PtyPair::spawn_command_and_close_controlled`]) before the controller's read
///    loop begins; that ensures [`EIO`] (or [`EOF`] on some platforms) arrives when the
///    child process exits rather than only when `drain_pty_and_wait` is reached.
/// 2. **Drain `buf_reader` until [`EIO`] or [`EOF`]** — unblocks the child's `exit()`
///    flush. Once the child process exits and its controlled fds close, the controller
///    gets [`EIO`] on Linux (or [`EOF`] on some platforms).
/// 3. **`child.wait()`** — the child has already exited, so this reaps the zombie
///    immediately.
///
/// # Platform behavior: POSIX [`EOF`] vs Linux [`EIO`]
///
/// When the controlled side is fully closed and the child process exits, the
/// controller's [`read()`] returns different signals depending on the platform:
///
/// - **Linux** returns [`EIO`] (`errno` `5`) -- a Linux-specific kernel behavior where
///   the [`PTY`] controller signals that the controlled side has no remaining open
///   [`fd`]s.
/// - **BSDs and other platforms** return a traditional [`EOF`] ([`read()`] returns `0`
///   bytes).
///
/// This function handles both signals (see `Ok(0)` and the [`EIO`] check in the drain
/// loop below). Any code that reads from a [`PTY`] controller must handle both to be
/// cross-platform correct.
///
/// # Parameters
///
/// - `buf_reader` - The buffered reader wrapping a cloned controller reader. Must be the
///   same reader used during the test's read loop (so buffered data is consumed).
/// - `pty_pair` - The [`PTY`] pair to drop. The controlled side should already have been
///   closed via [`PtyPair::spawn_command_and_close_controlled`] before the controller's
///   read loop started.
/// - `child` - The controlled child process to wait on.
///
/// # Panics
///
/// Panics if `child.wait()` fails.
///
/// [`EIO`]: https://man7.org/linux/man-pages/man3/errno.3.html
/// [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
/// [`fd`]: https://man7.org/linux/man-pages/man2/open.2.html
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`PtyPair::spawn_command_and_close_controlled`]: crate::PtyPair::spawn_command_and_close_controlled
/// [`read()`]: https://man7.org/linux/man-pages/man2/read.2.html
#[allow(clippy::needless_continue)]
pub fn drain_pty_and_wait(
    mut buf_reader: BufReader<ControllerReader>,
    pty_pair: PtyPair,
    child: &mut ControlledChild,
) {
    // Step 1: Drop pty_pair to release the controller fd.
    // The controlled fd should already be None (closed by
    // PtyPair::spawn_command_and_close_controlled before the read loop). The
    // buf_reader's cloned controller fd remains valid.
    drop(pty_pair);

    // Step 2: Drain buf_reader until EIO or EOF. This unblocks the child's exit()
    // flush. Once the child exits and its controlled fd closes, we get EIO on Linux
    // (or EOF on some platforms) here.
    let mut discard_buf = [0u8; 1024];
    loop {
        match buf_reader.read(&mut discard_buf) {
            Ok(0) => break,    // EOF — some platforms signal closure this way.
            Ok(_) => continue, // Discard remaining output.
            Err(e) => {
                // EIO (errno 5) is how Linux signals that the controlled side closed.
                // See: https://lists.archive.carbon60.com/linux/kernel/1790583
                const EIO: i32 = 5;
                if e.raw_os_error() == Some(EIO) {
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
