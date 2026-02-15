// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [RAII] subscription guard for the Resilient Reactor Thread pattern. See
//! [`SubscriberGuard`].
//!
//! [RAII]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization

use super::{RRTEvent, RRTWaker};
use std::sync::{Arc, Mutex};
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
/// # Shared Waker and Correctness
///
/// The [`waker`] field holds an [`Arc<Mutex<Option<W>>>`] that is shared with *all*
/// subscribers (old and new) and the [`TerminationGuard`].
///
/// Due to [two-phase setup], the [`RRTWaker`] and [`RRTWorker`] are created together from
/// the same [`mio::Poll`] registry. This shared wrapper ensures every subscriber always
/// reads the **current** [`RRTWaker`] trait implementation, even after a thread relaunch
/// - preventing a **zombie thread bug** where old subscribers would call a stale
/// [`RRTWaker`] trait implementation targeting a dead [`mio::Poll`].
///
/// When the thread dies, [`TerminationGuard::drop()`] clears the [`RRTWaker`] to
/// [`None`]. If a subscriber drops after the thread has already exited, the [`wake()`]
/// call is skipped (the [`Option`] is [`None`]), which is correct - there's no thread to
/// wake.
///
/// # Race Condition and Correctness
///
/// There is a [race window] between when the receiver is dropped and when the dedicated
/// thread checks [`receiver_count()`]. This is the **fast-path thread reuse** scenario -
/// if a new subscriber appears during the window, the thread correctly continues serving
/// it instead of exiting.
///
/// # Example
///
/// See [`DirectToAnsiInputDevice::next()`] for real usage.
///
/// [RAII]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
/// [`DirectToAnsiInputDevice::next()`]: crate::terminal_lib_backends::DirectToAnsiInputDevice::next
/// [`RRTWorker`]: super::RRTWorker
/// [`Sender`]: tokio::sync::broadcast::Sender
/// [`TerminationGuard::drop()`]: super::TerminationGuard
/// [`TerminationGuard`]: super::TerminationGuard
/// [`mio::Poll`]: mio::Poll
/// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
/// [`receiver`]: Self::receiver
/// [`wake()`]: RRTWaker::wake
/// [`waker.wake()`]: RRTWaker::wake
/// [`waker`]: Self::waker
/// [race window]: super#the-inherent-race-condition
/// [two-phase setup]: super#two-phase-setup
#[allow(missing_debug_implementations)]
pub struct SubscriberGuard<W, E>
where
    W: RRTWaker,
    E: Clone + Send + 'static,
{
    /// The actual broadcast receiver for events.
    ///
    /// Receives [`RRTEvent<E>`] to support the two-tier event model: domain events
    /// ([`RRTEvent::Worker`]) and framework infrastructure events
    /// ([`RRTEvent::Shutdown`]).
    ///
    /// Wrapped in [`Option`] so we can [`take()`] it in [`Drop`] to ensure the receiver
    /// is dropped before we call [`wake()`]. This guarantees the [`receiver_count()`]
    /// decrement happens first.
    ///
    /// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    /// [`take()`]: Option::take
    /// [`wake()`]: RRTWaker::wake
    pub receiver: Option<Receiver<RRTEvent<E>>>,

    /// Shared [`RRTWaker`] - always reads the current [`RRTWaker`] trait implementation
    /// via [`Arc<Mutex<Option<W>>>`].
    ///
    /// All subscribers (across all generations) hold a clone of the same [`Arc`], so
    /// dropping any subscriber wakes the *current* thread, not a stale one.
    ///
    /// [`Arc`]: std::sync::Arc
    pub waker: Arc<Mutex<Option<W>>>,
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
        // Lock the shared waker and call the current waker (if any).
        // If the thread has already exited, the waker is None (cleared by
        // TerminationGuard::drop()), so we skip the wake call.
        if let Ok(guard) = self.waker.lock() {
            if let Some(w) = guard.as_ref() {
                // Ignore errors - the thread may have already exited.
                drop(w.wake());
            }
        }
    }
}
