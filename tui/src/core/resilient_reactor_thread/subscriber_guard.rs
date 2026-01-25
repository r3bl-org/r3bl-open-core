// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [RAII] subscription guard for the Resilient Reactor Thread pattern. See
//! [`SubscriberGuard`].
//!
//! [RAII]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization

use super::{RRTWaker, ThreadState};
use std::sync::Arc;
use tokio::sync::broadcast::Receiver;

/// [RAII] guard that wakes the worker thread on drop.
///
/// # Purpose
///
/// Holding a [`SubscriberGuard`] keeps you subscribed to events from the worker thread.
/// Dropping it triggers the cleanup protocol that may cause the thread to exit.
///
/// # Drop Behavior
///
/// When this guard is dropped:
/// 1. [`receiver`] is dropped first, which causes Tokio's broadcast channel to atomically
///    decrement the [`Sender`]'s internal [`receiver_count()`].
/// 2. Then [`waker.wake()`] interrupts the worker's blocking call.
/// 3. The worker wakes and checks [`receiver_count()`] to decide if it should exit (when
///    count reaches `0`).
///
/// # Race Condition and Correctness
///
/// There is a race window between when the receiver is dropped and when the worker
/// checks [`receiver_count()`]. This is the **fast-path thread reuse** scenario — if a
/// new subscriber appears during the window, the thread correctly continues serving it
/// instead of exiting.
///
/// See [`ThreadState`] for comprehensive documentation on the race condition.
///
/// # Example
///
/// See [`DirectToAnsiInputDevice::next()`] for real usage.
///
/// [RAII]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
/// [`DirectToAnsiInputDevice::next()`]: crate::terminal_lib_backends::direct_to_ansi::input::DirectToAnsiInputDevice::next
/// [`Sender`]: tokio::sync::broadcast::Sender
/// [`ThreadState`]: super::ThreadState
/// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
/// [`receiver`]: Self::receiver
/// [`waker.wake()`]: RRTWaker::wake
#[allow(missing_debug_implementations)]
pub struct SubscriberGuard<W, E>
where
    W: RRTWaker,
    E: Clone + Send + 'static,
{
    /// The actual broadcast receiver for events.
    ///
    /// Wrapped in [`Option`] so we can [`take()`] it in [`Drop`] to ensure the receiver
    /// is dropped before we call `wake()`. This guarantees the [`receiver_count()`]
    /// decrement happens first.
    ///
    /// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    /// [`take()`]: Option::take
    pub receiver: Option<Receiver<E>>,

    /// Shared state including waker to signal the worker thread.
    ///
    /// We hold an [`Arc`] reference to keep the [`ThreadState`] alive. When this guard
    /// drops, we call [`waker.wake()`] to notify the worker thread.
    ///
    /// [`Arc`]: std::sync::Arc
    /// [`ThreadState`]: super::ThreadState
    /// [`waker.wake()`]: RRTWaker::wake
    pub state: Arc<ThreadState<W, E>>,
}

impl<W, E> Drop for SubscriberGuard<W, E>
where
    W: RRTWaker,
    E: Clone + Send + 'static,
{
    /// Drops receiver then wakes thread.
    ///
    /// See [Drop Behavior] for the full mechanism.
    ///
    /// [Drop Behavior]: SubscriberGuard#drop-behavior
    fn drop(&mut self) {
        // Drop receiver first so Sender::receiver_count() decrements.
        drop(self.receiver.take());

        // Wake the thread so it can check if it should exit.
        // Ignore errors — the thread may have already exited.
        drop(self.state.waker.wake());
    }
}
