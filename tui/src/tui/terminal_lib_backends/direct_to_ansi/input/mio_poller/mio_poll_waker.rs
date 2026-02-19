// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! mio-specific waker for the Resilient Reactor Thread pattern.
//!
//! [`MioPollWaker`] wraps a [`mio::Waker`] and implements [`RRTWaker`] to interrupt the
//! dedicated thread's [`mio::Poll::poll()`] call. The waker is created from the same
//! [`mio::Poll`] registry as the worker (see [two-phase setup]) and is tightly coupled to
//! it - if the poll is dropped, calling [`wake_and_unblock_dedicated_thread()`] has no
//! effect.
//!
//! [`RRTWaker`]: crate::core::resilient_reactor_thread::RRTWaker
//! [`mio::Poll::poll()`]: mio::Poll::poll
//! [`mio::Poll`]: mio::Poll
//! [`mio::Waker`]: mio::Waker
//! [`wake_and_unblock_dedicated_thread()`]: MioPollWaker::wake_and_unblock_dedicated_thread
//! [two-phase setup]: crate::core::resilient_reactor_thread#two-phase-setup

use crate::core::resilient_reactor_thread::RRTWaker;

/// Newtype wrapping [`mio::Waker`] to implement [`RRTWaker`].
///
/// Created from the same [`mio::Poll`] registry as the [`MioPollWorker`] it is paired
/// with. Calling [`wake_and_unblock_dedicated_thread()`] triggers an event on the poll,
/// causing [`mio::Poll::poll()`] to return.
///
/// # How It Works
///
/// See the [Poll -> Registry -> Waker Chain] diagram on [`RRTWaker`].
///
/// [Poll -> Registry -> Waker Chain]: RRTWaker#poll---registry---waker-chain
/// [`MioPollWorker`]: super::MioPollWorker
/// [`mio::Poll::poll()`]: mio::Poll::poll
/// [`mio::Poll`]: mio::Poll
/// [`mio::Waker`]: mio::Waker
/// [`wake_and_unblock_dedicated_thread()`]: Self::wake_and_unblock_dedicated_thread
#[derive(Debug)]
pub struct MioPollWaker(pub mio::Waker);

impl RRTWaker for MioPollWaker {
    /// Triggers an event on the paired [`mio::Poll`], causing its blocking
    /// [`poll()`] call to return.
    ///
    /// The return value of [`mio::Waker::wake()`] is intentionally discarded - if the
    /// poll has already been dropped (thread exited), the wake is a no-op.
    ///
    /// [`mio::Poll`]: mio::Poll
    /// [`mio::Waker::wake()`]: mio::Waker::wake
    /// [`poll()`]: mio::Poll::poll
    fn wake_and_unblock_dedicated_thread(&self) { let _unused = self.0.wake(); }
}
