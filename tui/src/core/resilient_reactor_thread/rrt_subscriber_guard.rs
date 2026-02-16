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
/// 2. Then the [`RRTWaker::wake()`] method interrupts the dedicated thread's blocking
///    call.
/// 3. The dedicated thread wakes and checks [`receiver_count()`] to decide if it should
///    exit (when count reaches `0`).
///
/// # Shared Waker and Correctness
///
/// The [`waker`] field holds an `Arc<Mutex<Option<Box<dyn RRTWaker>>>>` that is shared
/// with *all* subscribers (old and new) and the [`TerminationGuard`].
///
/// Due to [two-phase setup], the waker and [`RRTWorker`] are created together from
/// the same [`mio::Poll`] registry. This shared wrapper ensures every subscriber always
/// reads the **current** waker, even after a thread relaunch - preventing a **zombie
/// thread bug** where old subscribers would call a stale waker targeting a dead
/// [`mio::Poll`].
///
/// When the thread dies, [`TerminationGuard::drop()`] clears the waker to [`None`].
/// If a subscriber drops after the thread has already exited, the wake call is skipped
/// (the [`Option`] is [`None`]), which is correct - there's no thread to wake.
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
/// [`RRTWaker::wake()`]: super::RRTWaker::wake
/// [`RRTWorker`]: super::RRTWorker
/// [`Sender`]: tokio::sync::broadcast::Sender
/// [`TerminationGuard::drop()`]: super::TerminationGuard
/// [`TerminationGuard`]: super::TerminationGuard
/// [`mio::Poll`]: mio::Poll
/// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
/// [`receiver`]: Self::receiver
/// [`waker`]: Self::waker
/// [race window]: super#the-inherent-race-condition
/// [two-phase setup]: super#two-phase-setup
#[allow(missing_debug_implementations)]
pub struct SubscriberGuard<E>
where
    E: Clone + Send + 'static,
{
    /// The actual broadcast receiver for events.
    ///
    /// Receives [`RRTEvent<E>`] to support the two-tier event model: domain events
    /// ([`RRTEvent::Worker`]) and framework infrastructure events
    /// ([`RRTEvent::Shutdown`]).
    ///
    /// Wrapped in [`Option`] so we can [`take()`] it in [`Drop`] to ensure the receiver
    /// is dropped before we call [`RRTWaker::wake()`]. This guarantees the
    /// [`receiver_count()`] decrement happens first.
    ///
    /// [`RRTWaker::wake()`]: super::RRTWaker::wake
    /// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    /// [`take()`]: Option::take
    pub receiver: Option<Receiver<RRTEvent<E>>>,

    /// Shared waker - each layer of `Arc<Mutex<Option<Box<dyn RRTWaker>>>>` serves a
    /// purpose:
    ///
    /// - [`Arc`] - shared ownership across subscribers and [`TerminationGuard`].
    /// - [`Mutex`] - write access for swapping (on relaunch) and clearing (on death).
    /// - [`Option`] - [`None`] means the thread is dead; [`Drop`] skips the wake call.
    /// - [`Box<dyn RRTWaker>`] - type erasure avoids a second generic for the concrete
    ///   waker type (e.g., `SubscriberGuard<E, W: RRTWaker>`).
    ///
    /// [`Arc`]: std::sync::Arc
    /// [`Mutex`]: std::sync::Mutex
    /// [`TerminationGuard`]: super::TerminationGuard
    /// [`Box<dyn RRTWaker>`]: super::RRTWaker
    pub waker: Arc<Mutex<Option<Box<dyn RRTWaker>>>>,
}

impl<E> Drop for SubscriberGuard<E>
where
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
        // Lock the shared waker and call the current waker's wake() method (if any).
        // If the thread has already exited, the waker is None (cleared by
        // TerminationGuard::drop()), so we skip the wake call.
        if let Ok(guard) = self.waker.lock() {
            if let Some(waker) = guard.as_ref() {
                waker.wake();
            }
        }
    }
}
