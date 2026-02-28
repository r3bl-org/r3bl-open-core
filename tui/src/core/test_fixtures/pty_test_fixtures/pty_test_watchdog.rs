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

/// Safety-net timer that kills the child process if a [`PTY`] test controller hangs.
///
/// Spawns a background thread that sleeps for `timeout`, then checks a cancellation
/// flag. If the flag is still `false` (controller did not finish), the watchdog calls
/// [`kill()`] on the child — unblocking any blocked reads and converting an infinite
/// hang into a bounded test failure.
///
/// When the watchdog is dropped (controller finished normally), the `Drop` impl sets
/// the cancellation flag to `true`. The sleeping thread wakes at timeout, sees the flag,
/// and exits without killing anything. No `join()` — dropping never blocks.
///
/// [`kill()`]: portable_pty::ChildKiller::kill
/// [`PTY`]: crate::core::pty
#[allow(missing_debug_implementations)]
pub struct PtyTestWatchdog {
    cancelled: Arc<AtomicBool>,
}

impl PtyTestWatchdog {
    #[must_use]
    pub fn new(mut killer: ControlledChildTerminationHandle, timeout: Duration) -> Self {
        let cancelled = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&cancelled);

        thread::spawn(move || {
            thread::sleep(timeout);
            if !flag.load(Ordering::SeqCst) {
                eprintln!(
                    "⏱️  PtyTestWatchdog: timeout ({timeout:?}) expired, killing child"
                );
                drop(killer.kill());
            }
        });

        Self { cancelled }
    }
}

impl Drop for PtyTestWatchdog {
    fn drop(&mut self) { self.cancelled.store(true, Ordering::SeqCst); }
}
