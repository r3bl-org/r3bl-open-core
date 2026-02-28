// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{ControlledChild, ControlledChildTerminationHandle, ControllerReader,
            PtyPair, drain_pty_and_wait};
use std::io::BufReader;

/// Wraps [`ControlledChild`] for test controllers that cannot read and wait concurrently.
///
/// The [`generate_pty_test!`] macro runs the controller function on the test's single
/// thread. That thread reads child output sequentially, then waits for the child to exit.
/// If it calls bare [`wait()`] without first draining the [`PTY`] buffer, a deadlock
/// occurs:
///
/// 1. Controller stops reading and calls [`wait()`] - blocks on child exit
/// 2. Child's `exit()` flushes buffered output - blocks because the [`PTY`] buffer is
///    full (nobody is reading the controller side)
/// 3. Deadlock: controller waits for child, child waits for buffer space
///
/// This wrapper prevents the deadlock by hiding [`wait()`] entirely. The only exit path
/// is [`drain_and_wait()`], which drains the buffer before waiting.
///
/// **Not needed in production code.** Production [`PTY`] sessions run [`wait()`] inside
/// [`tokio::task::spawn_blocking`] while separate [`tokio`] tasks concurrently drain the
/// buffer. Because reading and waiting happen on different tasks (and threads), the
/// buffer never fills up and the deadlock cannot occur.
///
/// For the design rationale behind adding this newtype wrapper, see
/// [check-fix-hung-test-proc.md].
///
/// [`drain_and_wait()`]: Self::drain_and_wait
/// [`generate_pty_test!`]: crate::generate_pty_test
/// [`PTY`]: crate::core::pty
/// [`tokio`]: tokio
/// [`wait()`]: portable_pty::Child::wait
#[allow(missing_debug_implementations)]
pub struct SingleThreadSafeControlledChild {
    child: ControlledChild,
}

impl SingleThreadSafeControlledChild {
    #[must_use]
    pub fn new(child: ControlledChild) -> Self { Self { child } }

    /// Returns a handle that can kill the child process from another thread.
    ///
    /// The [`generate_pty_test!`] macro passes this handle to [`PtyTestWatchdog`], which
    /// kills the child if the controller hangs past the timeout.
    ///
    /// [`generate_pty_test!`]: crate::generate_pty_test
    /// [`PtyTestWatchdog`]: crate::PtyTestWatchdog
    #[must_use]
    pub fn clone_termination_handle(&self) -> ControlledChildTerminationHandle {
        self.child.clone_killer()
    }

    /// Drains remaining output from the [`PTY`] buffer, then waits for the child
    /// process to exit.
    ///
    /// Takes ownership of `self`, the `buf_reader`, and the `pty_pair` so that all
    /// [`PTY`] resources are closed and the child is reaped in one call. See
    /// [`drain_pty_and_wait`] for the underlying implementation.
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
