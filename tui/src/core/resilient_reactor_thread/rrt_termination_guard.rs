// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [RAII] guard that clears the waker on thread exit. See [`TerminationGuard`] for
//! details.
//!
//! [RAII]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization

use super::{RRTWorker, SharedWakerSlot};

/// [RAII] guard that clears the waker to [`None`] when the dedicated thread's work loop
/// exits.
///
/// The waker's [`Option`] state IS the liveness signal: `Some(waker)` means the thread is
/// running, `None` means it has terminated. Clearing it to `None` is the only cleanup
/// needed - [`subscribe()`] checks `is_none()` to detect termination.
///
/// [RAII]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
/// [`subscribe()`]: super::RRT::subscribe
#[allow(missing_debug_implementations)]
pub struct TerminationGuard<W: RRTWorker> {
    pub(super) shared_waker_slot: SharedWakerSlot<W::Waker>,
}

impl<W: RRTWorker> Drop for TerminationGuard<W> {
    /// Clears the [waker] to [`None`], which serves two purposes:
    /// 1. Prevents any [`SubscriberGuard`] from calling a stale
    ///    [`wake_and_unblock_dedicated_thread()`] on a dead thread.
    /// 2. Lets [`subscribe()`] detect termination via [`is_none()`] and trigger a
    ///    relaunch.
    ///
    /// See step 4 of the [Thread Lifecycle] for where this fits in the exit
    /// sequence.
    ///
    /// [Thread Lifecycle]: super::RRT#thread-lifecycle
    /// [`SubscriberGuard`]: super::SubscriberGuard
    /// [`is_none()`]: Option::is_none
    /// [`subscribe()`]: super::RRT::subscribe
    /// [`wake_and_unblock_dedicated_thread()`]:
    ///     super::RRTWaker::wake_and_unblock_dedicated_thread
    /// [waker]: super::RRTWaker
    fn drop(&mut self) {
        if let Ok(mut guard) = self.shared_waker_slot.lock() {
            *guard = None;
        }
    }
}
