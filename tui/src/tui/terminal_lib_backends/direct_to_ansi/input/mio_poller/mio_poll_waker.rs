// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! mio-specific waker for the Resilient Reactor Thread pattern.
//!
//! [`MioPollWaker`] wraps a [`mio::Waker`] and implements [`RRTWaker`] to interrupt the
//! dedicated thread's [`mio::Poll::poll()`] call. The waker is created from the same
//! [`mio::Poll`] registry as the worker (see [two-phase setup]) and is tightly coupled to
//! it - if the poll is dropped, calling [`wake()`] has no effect.
//!
//! [`RRTWaker`]: crate::core::resilient_reactor_thread::RRTWaker
//! [`mio::Poll::poll()`]: mio::Poll::poll
//! [`mio::Poll`]: mio::Poll
//! [`mio::Waker`]: mio::Waker
//! [`wake()`]: MioPollWaker::wake
//! [two-phase setup]: crate::core::resilient_reactor_thread#two-phase-setup

use crate::core::resilient_reactor_thread::RRTWaker;

/// Newtype wrapping [`mio::Waker`] to implement [`RRTWaker`].
///
/// Created from the same [`mio::Poll`] registry as the [`MioPollWorker`] it is paired
/// with. Calling [`wake()`] triggers an event on the poll, causing
/// [`mio::Poll::poll()`] to return.
///
/// # How It Works
///
/// ```text
/// mio::Poll::new()      // Creates OS event mechanism (epoll fd / kqueue)
///       │
///       ▼
/// poll.registry()       // Handle to register interest
///       │
///       ▼
/// Waker::new(registry)  // Registers with THIS Poll's mechanism
///       │
///       ▼
/// MioPollWaker(waker)   // Newtype for RRTWaker trait
///       │
///       ▼
/// waker.wake()          // Triggers event → poll.poll() returns
/// ```
///
/// [`MioPollWorker`]: super::MioPollWorker
/// [`mio::Poll::poll()`]: mio::Poll::poll
/// [`mio::Poll`]: mio::Poll
/// [`mio::Waker`]: mio::Waker
/// [`wake()`]: Self::wake
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
    fn wake(&self) { let _unused = self.0.wake(); }
}
