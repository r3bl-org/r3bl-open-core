// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [RAII] subscription guard for the Resilient Reactor Thread pattern. See
//! [`SubscriberGuard`].
//!
//! [RAII]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization

use super::{RRTEvent, RRTWaker, RRTWorker, SharedWakerSlot};
use tokio::sync::broadcast::Receiver;

/// An [RAII] guard that wakes the dedicated thread on drop.
///
/// Holding a [`SubscriberGuard`] keeps async consumers in your [TUI] and
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
/// 1. [`receiver`]-end of the [broadcast channel] is dropped first. This causes the
///    channel to decrement the [`Sender`]-end's internal [`receiver_count()`] in a
///    thread-safe manner.
/// 2. [`wake_on_drop`] is dropped next. Its [Drop implementation] calls
///    [`RRTWaker::wake_and_unblock_dedicated_thread()`] to interrupt the dedicated
///    thread's blocking call.
///
/// The dedicated thread then wakes and checks [`receiver_count()`] to decide if it should
/// exit (when it reaches `0`). This is step 4 of the [Thread Lifecycle] - see that
/// section for the full spawn/reuse/terminate sequence.
///
/// # Shared Waker Prevents the Zombie Thread Bug
///
/// Each [`WakeOnDrop`] holds a clone of the [`SharedWakerSlot<W::Waker>`]. This slot is
/// shared across *all* subscribers (old and new) and the [`TerminationGuard`].
/// - Read-only access - All subscribers read the inner waker through this slot.
/// - Read-write access - [`TerminationGuard`] is the sole writer that clears it to
///   [`None`] on drop.
///
/// Due to [two-phase setup], the [`RRTWaker`] and [`RRTWorker`] are created together from
/// the same [`mio::Poll`] registry. This shared wrapper ensures every subscriber always
/// reads the **current** [`RRTWaker`], even after a thread relaunch - preventing a
/// **zombie thread bug** where old subscribers would call a stale waker targeting a dead
/// [`mio::Poll`].
///
/// When the thread dies, [`TerminationGuard::drop()`] clears the [`RRTWaker`] to [`None`]
/// (see [Thread Lifecycle] step 4). If a subscriber drops after the thread has already
/// exited, the wake call is skipped (the [`Option`] is [`None`]), which is correct -
/// there's no thread to wake.
///
/// # Race Condition and Correctness
///
/// There is a [race window] between when the receiver is dropped and when the dedicated
/// thread checks [`receiver_count()`]. This is the **fast-path thread reuse** scenario
/// (see [Thread Lifecycle] step 5) - if a new subscriber appears during the window, the
/// thread correctly continues serving it instead of exiting.
///
/// # Example
///
/// See [`DirectToAnsiInputDevice::next()`] for real usage.
///
/// [Drop implementation]: WakeOnDrop#method.drop
/// [RAII]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
/// [RFC 1857]: https://rust-lang.github.io/rfcs/1857-stabilize-drop-order.html
/// [TUI]: crate::tui::TerminalWindow::main_event_loop
/// [Thread Lifecycle]: super::RRT#thread-lifecycle
/// [`DirectToAnsiInputDevice::next()`]:
///     crate::terminal_lib_backends::DirectToAnsiInputDevice::next
/// [`RRTWaker::wake_and_unblock_dedicated_thread()`]:
///     super::RRTWaker::wake_and_unblock_dedicated_thread
/// [`RRTWaker`]: super::RRTWaker
/// [`RRTWorker`]: super::RRTWorker
/// [`Sender`]: tokio::sync::broadcast::Sender
/// [`SharedWakerSlot<W::Waker>`]: super::SharedWakerSlot
/// [`TerminationGuard::drop()`]: super::TerminationGuard#method.drop
/// [`TerminationGuard`]: super::TerminationGuard
/// [`mio::Poll`]: mio::Poll
/// [`readline_async`]: crate::readline_async::ReadlineAsyncContext::try_new
/// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
/// [`receiver`]: Self::receiver
/// [`wake_on_drop`]: Self::wake_on_drop
/// [broadcast channel]: tokio::sync::broadcast
/// [race window]: super#the-inherent-race-condition
/// [two-phase setup]: super#two-phase-setup
#[allow(missing_debug_implementations, dead_code)]
pub struct SubscriberGuard<W: RRTWorker> {
    /// The broadcast receiver for events. Do not reorder - see
    /// [Drop Behavior].
    ///
    /// [Drop Behavior]: SubscriberGuard#drop-behavior
    pub receiver: Receiver<RRTEvent<W::Event>>,

    /// Wakes the dedicated thread on drop. Do not reorder - see
    /// [Drop Behavior].
    ///
    /// [Drop Behavior]: SubscriberGuard#drop-behavior
    pub wake_on_drop: WakeOnDrop<W::Waker>,
}

impl<W: RRTWorker> SubscriberGuard<W> {
    /// Creates a new `SubscriberGuard` from a receiver and shared waker.
    pub fn new(
        receiver: Receiver<RRTEvent<W::Event>>,
        shared_waker_slot: SharedWakerSlot<W::Waker>,
    ) -> Self {
        Self {
            receiver,
            wake_on_drop: WakeOnDrop::new(shared_waker_slot),
        }
    }
}

/// Calls [`RRTWaker::wake_and_unblock_dedicated_thread()`] when dropped. See
/// [Drop Behavior] for why field ordering matters.
///
/// [Drop Behavior]: SubscriberGuard#drop-behavior
/// [`RRTWaker::wake_and_unblock_dedicated_thread()`]: super::RRTWaker::wake_and_unblock_dedicated_thread
#[allow(missing_debug_implementations)]
pub struct WakeOnDrop<K: RRTWaker> {
    shared_waker_slot: SharedWakerSlot<K>,
}

impl<K: RRTWaker> WakeOnDrop<K> {
    /// Creates a new `WakeOnDrop` from a shared waker.
    pub fn new(shared_waker_slot: SharedWakerSlot<K>) -> Self {
        Self { shared_waker_slot }
    }
}

impl<K: RRTWaker> Drop for WakeOnDrop<K> {
    /// Wakes the dedicated thread so it can check whether it should exit.
    ///
    /// If the thread has already exited, the [waker] is [`None`] (cleared by
    /// [`TerminationGuard::drop()`]), so the wake call is skipped.
    ///
    /// See step 4 of the [Thread Lifecycle] for where this fits in the exit
    /// sequence.
    ///
    /// [Thread Lifecycle]: RRT#thread-lifecycle
    /// [`TerminationGuard::drop()`]: super::TerminationGuard#method.drop
    /// [waker]: super::RRTWaker
    fn drop(&mut self) {
        if let Ok(guard) = self.shared_waker_slot.lock() {
            if let Some(waker) = guard.as_ref() {
                waker.wake_and_unblock_dedicated_thread();
            }
        }
    }
}

#[cfg(test)]
mod drop_order_tests {
    use super::*;
    use crate::Continuation;
    use std::sync::{Arc, Mutex,
                    atomic::{AtomicBool, Ordering}};
    use tokio::sync::broadcast;

    /// A waker that records whether the receiver was already dropped when
    /// `wake_and_unblock_dedicated_thread()` fires.
    struct DropOrderWaker {
        sender: broadcast::Sender<RRTEvent<()>>,
        receiver_was_dropped_first: Arc<AtomicBool>,
    }

    impl RRTWaker for DropOrderWaker {
        fn wake_and_unblock_dedicated_thread(&self) {
            if self.sender.receiver_count() == 0 {
                self.receiver_was_dropped_first
                    .store(true, Ordering::SeqCst);
            }
        }
    }

    struct DropOrderTestWorker;

    impl RRTWorker for DropOrderTestWorker {
        type Event = ();
        type Waker = DropOrderWaker;

        fn create_and_register_os_sources() -> miette::Result<(Self, Self::Waker)> {
            unimplemented!("Not used in drop-order test")
        }

        fn block_until_ready_then_dispatch(
            &mut self,
            _sender: &broadcast::Sender<RRTEvent<Self::Event>>,
        ) -> Continuation {
            unimplemented!("Not used in drop-order test")
        }
    }

    /// Enforces the field drop order invariant documented in
    /// [`SubscriberGuard`'s Drop Behavior].
    /// If someone reorders the fields, this test fails.
    ///
    /// [`SubscriberGuard`'s Drop Behavior]: SubscriberGuard#drop-behavior
    #[test]
    fn subscriber_guard_drops_receiver_before_wake() {
        let (sender, receiver) = broadcast::channel::<RRTEvent<()>>(16);
        assert_eq!(sender.receiver_count(), 1);

        let receiver_was_dropped_first = Arc::new(AtomicBool::new(false));

        let waker = DropOrderWaker {
            sender: sender.clone(),
            receiver_was_dropped_first: receiver_was_dropped_first.clone(),
        };

        let shared_waker_slot: SharedWakerSlot<DropOrderWaker> =
            Arc::new(Mutex::new(Some(waker)));

        let guard: SubscriberGuard<DropOrderTestWorker> =
            SubscriberGuard::new(receiver, shared_waker_slot);

        drop(guard);

        assert!(
            receiver_was_dropped_first.load(Ordering::SeqCst),
            "BUG: wake_and_unblock_dedicated_thread() fired before receiver was dropped! \
             The `receiver` field MUST be declared before `wake_on_drop` \
             (RFC 1857 - fields drop in declaration order)."
        );
    }
}
