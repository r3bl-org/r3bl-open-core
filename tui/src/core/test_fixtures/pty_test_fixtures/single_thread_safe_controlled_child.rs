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

/// Drains the [`PTY`] until [`EIO`] or [`EOF`], then waits for the child process to exit.
/// Prevents deadlocks caused by unread [`PTY`] buffer data.
///
/// # Problem
///
/// See the [Two Types of Deadlocks] section in [`PtyPair`] for how this function handles
/// the secondary **buffer-full deadlock** that occurs in **contrived single-threaded
/// tests**.
///
/// This function solves a [`PTY`] buffer deadlock that occurs on macOS (and occasionally
/// on Linux) when a controlled process writes to stderr after the controller has stopped
/// reading. The sequence that causes the deadlock:
///
/// 1. Controller reads [`PTY`] until a marker (e.g., `SUCCESS`, `CONTROLLED_DONE`).
/// 2. Controller stops reading and calls [`ControlledChild::wait()`].
/// 3. Child writes more [`eprintln!()`] after the marker, then calls
///    [`std::process::exit(0)`].
/// 4. [`std::process::exit(0)`] flushes [`stdio`], which **blocks** because the [`PTY`]
///    buffer is full (nobody is reading the controller side).
/// 5. Deadlock happens: controller waits for child, child waits for buffer space.
///
/// macOS [`PTY`] buffers are ~1 KB (vs ~4 KB on Linux), making this trigger frequently.
///
/// # Solution
///
/// 1. **Drop `pty_pair`** — closes the parent's controller [`fd`]. The `buf_reader`'s
///    cloned controller [`fd`] remains valid. The controlled [`fd`] must already be
///    closed by the caller (via [`PtyPair::open_and_spawn()`]) before the controller's
///    reading phase begins; that ensures [`EIO`] (or [`EOF`] on some platforms) arrives
///    when the child process exits rather than only when [`drain_pty_and_wait()`] is
///    reached.
/// 2. **Drain `buf_reader` until [`EIO`] or [`EOF`]** — unblocks the child's
///    [`std::process::exit(0)`] flush. Once the child process exits and its controlled
///    [`fd`]s close, the controller gets [`EIO`] on Linux (or [`EOF`] on some platforms).
/// 3. **[`child.wait()`]** — the child has already exited, so this reaps the zombie
///    immediately.
///
/// # Platform behavior: POSIX [`EOF`] vs Linux [`EIO`]
///
/// When the controlled side is fully closed and the child process exits, the controller's
/// blocking [`read()`] returns different signals depending on the platform:
///
/// - **Linux** returns [`EIO`] (`errno` `5`) -- a Linux-specific kernel behavior where
///   the [`PTY`] controller signals that the controlled side has no remaining open
///   [`fd`]s.
/// - **BSD and other platforms** return a traditional [`EOF`] (blocking [`read()`]
///   returns `0` bytes).
///
/// This function handles both signals (see `Ok(0)` and the [`EIO`] check in the drain
/// loop below). Any code that reads from a [`PTY`] controller must handle both to be
/// cross-platform correct.
///
/// # Arguments
///
/// - `buf_reader` - The buffered reader wrapping a cloned controller reader. Must be the
///   same reader used during the test's reading phase (so buffered data is consumed).
/// - `pty_pair` - The [`PTY`] pair to drop. The controlled side should already have been
///   closed via [`PtyPair::open_and_spawn()`] before the controller's reading phase
///   started.
/// - `child` - The controlled child process to wait on.
///
/// # Panics
///
/// Panics if [`child.wait()`] fails.
///
/// [`child.wait()`]: portable_pty::Child::wait
/// [`ControlledChild::wait()`]: portable_pty::Child::wait
/// [`EIO`]: https://man7.org/linux/man-pages/man3/errno.3.html
/// [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
/// [`fd`]: https://man7.org/linux/man-pages/man2/open.2.html
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`PtyPair::open_and_spawn()`]: crate::PtyPair::open_and_spawn
/// [`PtyPair`]: crate::PtyPair
/// [`read()`]: https://man7.org/linux/man-pages/man2/read.2.html
/// [`std::process::exit(0)`]: std::process::exit
/// [`stdio`]: std::io
/// [Two Types of Deadlocks]: crate::PtyPair#two-types-of-pty-deadlocks
#[allow(clippy::needless_continue)]
pub fn drain_pty_and_wait(
    mut buf_reader: BufReader<ControllerReader>,
    pty_pair: PtyPair,
    child: &mut ControlledChild,
) {
    // Step 1: Drop pty_pair to release the controller side's main handle.
    // The parent's copy of the controlled fd is already closed (via
    // PtyPair::open_and_spawn), which is the primary resource leak deadlock
    // safeguard. Dropping the pair here ensures the parent is in a clean state
    // where only the buf_reader (a clone) is actively reading.
    drop(pty_pair);

    // Step 2: Drain buf_reader until EIO or EOF. This prevents the termination
    // deadlock where the child's exit() flush blocks on a full PTY buffer.
    // Once the child process's own copies of the controlled fd close, the
    // controller receives EIO on Linux (or EOF on some platforms) here.
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
