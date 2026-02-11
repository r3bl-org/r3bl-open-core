// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [RAII] subscription guard for the Resilient Reactor Thread pattern. See
//! [`SubscriberGuard`].
//!
//! [RAII]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization

use super::{RRTState, RRTWaker};
use std::sync::Arc;
use tokio::sync::broadcast::Receiver;

/// An [RAII] guard that wakes the dedicated thread on drop.
///
/// # Purpose
///
/// Holding a [`SubscriberGuard`] keeps you subscribed to events from the dedicated
/// thread. Dropping it triggers the cleanup protocol that may cause the thread to exit.
///
/// # Drop Behavior
///
/// When this guard is dropped:
/// 1. [`receiver`] is dropped first, which causes Tokio's broadcast channel to atomically
///    decrement the [`Sender`]'s internal [`receiver_count()`].
/// 2. Then [`waker.wake()`] interrupts the dedicated thread's blocking call.
/// 3. The dedicated thread wakes and checks [`receiver_count()`] to decide if it should
///    exit (when count reaches `0`).
///
/// # Race Condition and Correctness
///
/// There is a race window between when the receiver is dropped and when the dedicated
/// thread checks [`receiver_count()`]. This is the **fast-path thread reuse** scenario -
/// if a new subscriber appears during the window, the thread correctly continues serving
/// it instead of exiting.
///
/// See [`RRTState`] for comprehensive documentation on the race condition.
///
/// # Example
///
/// See [`DirectToAnsiInputDevice::next()`] for real usage.
///
/// [RAII]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
/// [`DirectToAnsiInputDevice::next()`]: crate::terminal_lib_backends::DirectToAnsiInputDevice::next
/// [`Sender`]: tokio::sync::broadcast::Sender
/// [`RRTState`]: super::RRTState
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

    /// Shared state including your [`RRTWaker`] implementation to signal the dedicated
    /// thread.
    ///
    /// We hold an [`Arc`] reference to keep the [`RRTState`] alive. When this guard
    /// drops, we call [`waker.wake()`] to notify the dedicated thread.
    ///
    /// [`Arc`]: std::sync::Arc
    /// [`RRTState`]: super::RRTState
    /// [`waker.wake()`]: RRTWaker::wake
    pub state: Arc<RRTState<W, E>>,
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
        // Ignore errors - the thread may have already exited.
        drop(self.state.waker.wake());
    }
}
