// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [RAII] subscription guard for the Resilient Reactor Thread pattern.
//!
//! [`SubscriberGuard`] holds a [`broadcast`] subscription and signals the worker thread
//! when dropped. This ensures proper cleanup even on [panic] (via [stack unwinding]).
//!
//! [RAII]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
//! [`broadcast`]: tokio::sync::broadcast
//! [panic]: https://doc.rust-lang.org/std/macro.panic.html
//! [stack unwinding]: https://doc.rust-lang.org/nomicon/unwinding.html

use super::{ThreadState, ThreadWaker};
use std::sync::Arc;
use tokio::sync::broadcast::Receiver;

/// RAII guard that wakes the worker thread on drop.
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
/// ```no_run
/// # use r3bl_tui::core::resilient_reactor_thread::*;
/// # use r3bl_tui::Continuation;
/// # use tokio::sync::broadcast::Sender;
/// #
/// # #[derive(Clone)]
/// # struct MyEvent;
/// # #[derive(Debug)]
/// # struct MyError;
/// # impl std::fmt::Display for MyError {
/// #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "error") }
/// # }
/// # impl std::error::Error for MyError {}
/// # struct MyWaker;
/// # impl ThreadWaker for MyWaker {
/// #     fn wake(&self) -> std::io::Result<()> { Ok(()) }
/// # }
/// # struct MyWorker;
/// # impl ThreadWorker for MyWorker {
/// #     type Event = MyEvent;
/// #     fn poll_once(&mut self, _tx: &Sender<Self::Event>) -> Continuation { todo!() }
/// # }
/// # struct MyFactory;
/// # impl ThreadWorkerFactory for MyFactory {
/// #     type Event = MyEvent;
/// #     type Worker = MyWorker;
/// #     type Waker = MyWaker;
/// #     type SetupError = MyError;
/// #     fn setup() -> Result<(Self::Worker, Self::Waker), Self::SetupError> { todo!() }
/// # }
/// # static GLOBAL: ThreadSafeGlobalState<MyWaker, MyEvent> = ThreadSafeGlobalState::new();
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Get a subscription
/// let mut guard = GLOBAL.allocate::<MyFactory>()?;
///
/// // Receive events while guard is held
/// while let Ok(event) = guard.receiver.as_mut().unwrap().recv().await {
///     // Process event...
/// }
///
/// // When guard drops, worker thread is notified
/// drop(guard);
/// # Ok(())
/// # }
/// ```
///
/// [`Sender`]: tokio::sync::broadcast::Sender
/// [`ThreadState`]: super::ThreadState
/// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
/// [`receiver`]: Self::receiver
/// [`waker.wake()`]: ThreadWaker::wake
#[allow(missing_debug_implementations)]
pub struct SubscriberGuard<W, E>
where
    W: ThreadWaker,
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
    /// [`waker.wake()`]: ThreadWaker::wake
    pub state: Arc<ThreadState<W, E>>,
}

impl<W, E> Drop for SubscriberGuard<W, E>
where
    W: ThreadWaker,
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
