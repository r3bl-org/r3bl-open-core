// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::ControlledChildTerminationHandle;
use std::{sync::{Arc,
                 atomic::{AtomicBool, Ordering}},
          thread,
          time::Duration};

/// Default timeout before the watchdog kills a hung [`PTY`] test child.
///
/// [`PTY`]: crate::core::pty
pub const PTY_TEST_WATCHDOG_TIMEOUT: Duration = Duration::from_secs(30);

/// Safety-net timer that terminates the child process if a [`PTY`] test controller hangs.
///
/// Spawns a background thread that sleeps for [`timeout`], then checks a cancellation
/// flag. If the flag is still `false` (controller did not finish), the watchdog calls
/// [`kill()`] on the `termination_handle` passed to [`new()`], to terminate the child
/// process - unblocking any blocked reads and converting an infinite hang into a bounded
/// test failure.
///
/// When the watchdog is dropped (controller finished normally), the [`Drop`] impl sets
/// the [`cancelled`] flag to `true`. The sleeping thread wakes at timeout, sees the flag,
/// and exits without terminating the child process.
///
/// [`cancelled`]: Self::cancelled
/// [`Drop`]: Self#method.drop
/// [`kill()`]: portable_pty::ChildKiller::kill
/// [`new()`]: Self::new
/// [`PTY`]: crate::core::pty
/// [`timeout`]: crate::PTY_TEST_WATCHDOG_TIMEOUT
#[allow(missing_debug_implementations)]
pub struct PtyTestWatchdog {
    /// See the [struct docs] for details.
    ///
    /// [struct docs]: Self
    pub cancelled: Arc<AtomicBool>,
}

impl PtyTestWatchdog {
    /// See the [struct docs] for details.
    ///
    /// [struct docs]: Self
    #[must_use]
    pub fn new(mut termination_handle: ControlledChildTerminationHandle) -> Self {
        let timeout = PTY_TEST_WATCHDOG_TIMEOUT;
        let cancelled = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&cancelled);

        thread::spawn(move || {
            thread::sleep(timeout);
            if !flag.load(Ordering::SeqCst) {
                eprintln!(
                    "⏱️  PtyTestWatchdog: timeout ({timeout:?}) expired, killing child"
                );
                drop(termination_handle.kill());
            }
        });

        Self { cancelled }
    }
}

impl Drop for PtyTestWatchdog {
    /// See the [struct docs] for details.
    ///
    /// [struct docs]: Self
    fn drop(&mut self) { self.cancelled.store(true, Ordering::SeqCst); }
}
