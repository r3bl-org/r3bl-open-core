// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`RAII`] subscription guard for the Resilient Reactor Thread (RRT) pattern. See
//! [`SubscriberGuard`] for details.
//!
//! [`RAII`]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization

use super::{BroadcastSender, RRTEvent, RRTWorker, SubscribeError, ThreadLifecycleMonitor};
use std::sync::Arc;
use tokio::sync::broadcast::Receiver;

/// An [`RAII`] guard that interrupts the dedicated thread on drop.
///
/// Holding a [`SubscriberGuard`] keeps async consumers in your [`TUI`] and
/// [`readline_async`] app subscribed to events from the dedicated thread. Dropping it
/// triggers the cleanup mechanism that may cause the thread to exit (see [Thread
/// Lifecycle]).
///
/// # Drop Behavior
///
/// **Do not reorder the fields in this struct.** But don't worry if you do, since the
/// drop order unit test in this file will fail.
///
/// Field drop order is guaranteed by the Rust language ([RFC 1857]). Fields drop in
/// declaration order, first to last, which allows us to use struct composition to control
/// the drop order. No need for the messy use of `Option` wrappers or `unsafe
/// ManuallyDrop`. Here's the drop sequence for our guard, in the order of field
/// declaration:
///
/// 1. [`receiver`] is dropped first. This causes the [broadcast channel] to decrement the
///    [`Sender`]-end's internal [`receiver_count()`] in a thread-safe manner.
/// 2. [`interrupt_on_drop`] is dropped next. Its [`Drop`] impl calls
///    [`ThreadLifecycleMonitor::interrupt_if_running()`], which acquires the [`state`]
///    lock and (if the state is [`Running`]) calls the [`InterruptHandle`] inside the
///    variant. This interrupts the dedicated thread's blocking I/O call (e.g.,
///    `epoll_wait` inside [`mio::Poll`]).
/// 3. [`sender`] is dropped last. Dropping a [`Sender`] clone does not affect
///    [`receiver_count()`], so this has no impact on lifecycle interrupt-check logic.
///
/// # What Happens After the Interrupt
///
/// When the dedicated thread is interrupted from its blocking I/O call, it returns to the
/// top of [`run_worker_loop()`] and checks (under the [`state`] lock) whether
/// [`receiver_count()`] has reached zero. If so, the thread transitions its [`state`]
/// from [`Running`] → [`Stopping`] and begins teardown.
///
/// # Example
///
/// See [`DirectToAnsiInputDevice::next()`] for real usage.
///
///
/// # Poison Safety
///
/// This struct is **poison-safe**. Its [`Drop`] implementation triggers an interrupt
/// signal to the dedicated thread via [`ThreadLifecycleMonitor::interrupt_if_running()`],
/// which is poison-safe. This ensures that terminal restoration is never blocked by a
/// **Double Panic Abort**.
///
/// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section in the
/// crate root documentation for details.
///
/// [`DirectToAnsiInputDevice::next()`]:
///     crate::terminal_lib_backends::DirectToAnsiInputDevice::next
/// [`interrupt_on_drop`]: Self::interrupt_on_drop
/// [`InterruptHandle`]: super::InterruptHandle
/// [`mio::Poll`]: mio::Poll
/// [`RAII`]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
/// [`readline_async`]: crate::readline_async::ReadlineAsyncContext::try_new
/// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
/// [`receiver`]: Self::receiver
/// [`run_worker_loop()`]: super::run_worker_loop
/// [`Running`]: super::ThreadState::Running
/// [`sender`]: Self::sender
/// [`Sender`]: tokio::sync::broadcast::Sender
/// [`state`]: super::ThreadLifecycleMonitor::lock()
/// [`Stopping`]: super::ThreadState::Stopping
/// [`ThreadLifecycleMonitor::interrupt_if_running()`]:
///     super::ThreadLifecycleMonitor::interrupt_if_running
/// [`TUI`]: crate::tui::TerminalWindow::main_event_loop
/// [broadcast channel]: tokio::sync::broadcast
/// [RFC 1857]: https://rust-lang.github.io/rfcs/1857-stabilize-drop-order.html
/// [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety]:
///     crate#terminal-restoration-panic-drop-and-mutex-poison-safety
/// [Thread Lifecycle]: super::RRT#thread-lifecycle
#[allow(missing_debug_implementations, dead_code)]
pub struct SubscriberGuard<W: RRTWorker> {
    /// The broadcast receiver for events. Do not reorder - see [Drop Behavior].
    ///
    /// [Drop Behavior]: SubscriberGuard#drop-behavior
    pub receiver: Receiver<RRTEvent<W::Event>>,

    /// Interrupts the dedicated thread on drop. Do not reorder - see [Drop Behavior].
    ///
    /// [Drop Behavior]: SubscriberGuard#drop-behavior
    pub interrupt_on_drop: InterruptOnDrop<W>,

    /// Clone of the broadcast sender, used to create additional subscriptions from an
    /// existing guard.
    pub sender: BroadcastSender<W::Event>,
}

impl<W: RRTWorker> SubscriberGuard<W> {
    pub fn new(
        sender: BroadcastSender<W::Event>,
        receiver: Receiver<RRTEvent<W::Event>>,
        shared_state: Arc<ThreadLifecycleMonitor<W>>,
    ) -> Self {
        Self {
            receiver,
            interrupt_on_drop: InterruptOnDrop { shared_state },
            sender,
        }
    }

    /// Creates another subscriber guard from this guard.
    ///
    /// This method delegates to the shared [`try_subscribe()`] logic. It returns an
    /// existing subscription if the dedicated thread is already [`Running`], or
    /// automatically spawns a fresh thread if it is [`Stopped`].
    ///
    /// # Errors
    ///
    /// Returns [`SubscribeError::MutexPoisoned`] if the internal state mutex is
    /// poisoned.
    ///
    /// # Implementation Details
    ///
    /// This method uses [`ThreadLifecycleMonitor::block_until_stable_state_reached()`] to
    /// wait for transient states to resolve.
    ///
    /// [`Running`]: crate::resilient_reactor_thread::ThreadState::Running
    /// [`Stopped`]: crate::resilient_reactor_thread::ThreadState::Stopped
    /// [`try_subscribe()`]: crate::resilient_reactor_thread::try_subscribe
    pub fn try_subscribe(&self) -> Result<Self, SubscribeError> {
        super::try_subscribe(&self.sender, &self.interrupt_on_drop.shared_state)
    }
}

/// Calls [`ThreadLifecycleMonitor::interrupt_if_running()`] when dropped. See
/// [`SubscriberGuard`]'s [Drop Behavior] section for why field ordering matters.
///
/// [`ThreadLifecycleMonitor::interrupt_if_running()`]:
///     super::ThreadLifecycleMonitor::interrupt_if_running
/// [Drop Behavior]: SubscriberGuard#drop-behavior
#[allow(missing_debug_implementations)]
pub struct InterruptOnDrop<W: RRTWorker> {
    pub shared_state: Arc<ThreadLifecycleMonitor<W>>,
}

impl<W: RRTWorker> Drop for InterruptOnDrop<W> {
    fn drop(&mut self) {
        // This handles its own poisoning and never fails.
        self.shared_state.interrupt_if_running();
    }
}

#[cfg(test)]
mod drop_order_tests {
    use super::*;
    use crate::{Continuation,
                core::resilient_reactor_thread::{InterruptHandle, RRTSoftwareInterrupt,
                                                 ThreadState}};
    use std::sync::Arc;
    use tokio::sync::broadcast;

    /// Enforces the field drop order invariant documented in [`SubscriberGuard`]'s [Drop
    /// Behavior]. If someone reorders the fields, this test fails.
    ///
    /// [Drop Behavior]: SubscriberGuard#drop-behavior
    #[test]
    fn subscriber_guard_drops_receiver_before_interrupt() {
        let (sender, _) = broadcast::channel::<RRTEvent<()>>(16);

        assert_eq!(
            sender.receiver_count(),
            0,
            "Precondition failed: sender should start with 0 receivers"
        );

        let interrupt = DropOrderInterrupt {
            sender: sender.clone(),
        };

        // Build the monitor with state in Running(InterruptHandle::new(...)).
        let shared_state = Arc::new(ThreadLifecycleMonitor::<DropOrderTestWorker>::new(
            ThreadState::Running(InterruptHandle::new(interrupt)),
        ));

        let receiver = sender.subscribe();
        let guard: SubscriberGuard<DropOrderTestWorker> =
            SubscriberGuard::new(sender.clone(), receiver, Arc::clone(&shared_state));

        assert_eq!(
            sender.receiver_count(),
            1,
            "Precondition failed: subscribing should increase receiver count to 1"
        );

        drop(guard); // Panics if fields are dropped in the wrong order.
    }

    /// An interrupt handle that asserts the receiver was already dropped when
    /// `trigger_software_interrupt()` fires.
    #[derive(Debug)]
    struct DropOrderInterrupt {
        sender: broadcast::Sender<RRTEvent<()>>,
    }

    impl RRTSoftwareInterrupt for DropOrderInterrupt {
        fn trigger_software_interrupt(&self) {
            assert_eq!(
                self.sender.receiver_count(),
                0,
                "BUG: trigger_software_interrupt() fired before receiver \
                 was dropped! The `receiver` field MUST be declared before \
                 `interrupt_on_drop` (RFC 1857 - fields drop in declaration order)."
            );
        }
    }

    #[derive(Debug)]
    struct DropOrderTestWorker;

    impl RRTWorker for DropOrderTestWorker {
        type Event = ();
        type Interrupt = DropOrderInterrupt;

        fn create_and_register_os_sources() -> miette::Result<(Self, Self::Interrupt)> {
            unimplemented!("Not used in drop-order test")
        }

        fn block_until_ready_then_dispatch(
            &mut self,
            _sender: &broadcast::Sender<RRTEvent<Self::Event>>,
        ) -> Continuation {
            unimplemented!("Not used in drop-order test")
        }
    }
}
