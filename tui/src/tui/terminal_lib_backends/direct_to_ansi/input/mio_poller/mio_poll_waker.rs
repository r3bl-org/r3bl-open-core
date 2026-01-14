// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! mio-specific waker implementation for the Resilient Reactor Thread pattern.
//!
//! [`MioPollWaker`] wraps [`mio::Waker`] and implements [`ThreadWaker`] to integrate with
//! the generic RRT infrastructure.
//!
//! [`ThreadWaker`]: crate::core::resilient_reactor_thread::ThreadWaker

use crate::core::resilient_reactor_thread::ThreadWaker;
use mio::Waker;

/// mio-specific waker that interrupts a blocked [`mio::Poll::poll()`] call.
///
/// This is a thin wrapper around [`mio::Waker`] that implements [`ThreadWaker`] for use
/// with the generic RRT infrastructure.
///
/// # How It Works
///
/// When [`wake()`] is called:
/// 1. The underlying [`mio::Waker::wake()`] triggers an event on the poll instance
/// 2. The blocked [`mio::Poll::poll()`] returns with a [`ReceiverDropWaker`] token
/// 3. The worker's event handler checks [`receiver_count()`] to decide whether to exit
///
/// # Coupling With Poll
///
/// The waker is **tightly coupled** to its [`mio::Poll`] instance — it was created from
/// that poll's registry. If the poll is dropped, calling [`wake()`] will fail or have no
/// effect.
///
/// This is why [`ThreadWorkerFactory::setup()`] must create the poll, waker, and worker
/// together — they share an OS-level bond.
///
/// [`ReceiverDropWaker`]: super::SourceKindReady::ReceiverDropWaker
/// [`ThreadWorkerFactory::setup()`]: crate::core::resilient_reactor_thread::ThreadWorkerFactory::setup
/// [`mio::Poll::poll()`]: mio::Poll::poll
/// [`mio::Poll`]: mio::Poll
/// [`mio::Waker::wake()`]: mio::Waker::wake
/// [`mio::Waker`]: mio::Waker
/// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
/// [`wake()`]: Self::wake
#[allow(missing_debug_implementations)]
pub struct MioPollWaker(pub Waker);

impl ThreadWaker for MioPollWaker {
    /// Wakes the mio poller thread by triggering a wake event.
    ///
    /// This causes the blocked [`mio::Poll::poll()`] call to return, allowing the thread
    /// to check if it should exit.
    ///
    /// [`mio::Poll::poll()`]: mio::Poll::poll
    fn wake(&self) -> std::io::Result<()> { self.0.wake() }
}
