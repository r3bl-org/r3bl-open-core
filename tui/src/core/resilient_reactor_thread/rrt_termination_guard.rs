// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`RAII`] thread-exit guard that transitions the dedicated thread's state to
//! [`Stopped`] when the work loop exits. See [`TerminationGuard`] for details.
//!
//! [`RAII`]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
//! [`Stopped`]: super::ThreadState::Stopped

use super::{RRTWorker, ThreadLifecycleMonitor, ThreadState};
use crate::DEBUG_TUI_SHOW_RESILIENT_REACTOR_THREAD;
use std::sync::Arc;

/// [`RAII`] guard that transitions the dedicated thread to [`ThreadState::Stopped`] and
/// wakes any blocked subscribers when the work loop exits.
///
/// [`RAII`]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
/// [`ThreadState::Stopped`]: super::ThreadState::Stopped
#[allow(missing_debug_implementations)]
pub struct TerminationGuard<W: RRTWorker> {
    pub shared_state: Arc<ThreadLifecycleMonitor<W>>,
}

impl<W: RRTWorker> From<Arc<ThreadLifecycleMonitor<W>>> for TerminationGuard<W> {
    fn from(shared_state: Arc<ThreadLifecycleMonitor<W>>) -> Self {
        Self { shared_state }
    }
}

impl<W: RRTWorker> Drop for TerminationGuard<W> {
    /// # Poison Safety
    ///
    /// This implementation is **poison-safe**. It uses
    /// [`ThreadLifecycleMonitor::lock_raw()`] to handle poisoning without panicking,
    /// ensuring that even if a panic occurs, the state transition to [`Stopped`] and
    /// subsequent notification are still attempted. We prioritize **Resilience over
    /// Integrity** here to ensure that subsequent attempts to restart the thread (via
    /// [`try_subscribe()`]) are not blocked.
    ///
    /// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section in
    /// the crate root documentation for details.
    ///
    /// [`Stopped`]: crate::resilient_reactor_thread::ThreadState::Stopped
    /// [`try_subscribe()`]: crate::resilient_reactor_thread::RRT::try_subscribe
    /// [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety]:
    ///     crate#terminal-restoration-panic-drop-and-mutex-poison-safety
    fn drop(&mut self) {
        DEBUG_TUI_SHOW_RESILIENT_REACTOR_THREAD.then(|| {
            tracing::info!(
                message =
                    "RRT: TerminationGuard dropping, transitioning state to Stopped."
            );
        });

        // Poison-safe lock: attempt transition even if dirty.
        let state_guard = match self.shared_state.lock_raw() {
            Ok(guard) => guard,
            Err(poisoned) => {
                // % is Display, ? is Debug.
                tracing::error!(
                    message = "TerminationGuard: state lock poisoned, attempting transition anyway",
                    error = ?poisoned
                );
                poisoned.into_inner()
            }
        };

        drop(
            self.shared_state
                .set_state(state_guard, ThreadState::Stopped),
        );

        self.shared_state.notify_all();
    }
}
