// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Newtype wrappers for compile-time access control on [`SharedWakerSlot`]. See
//! [`WakerSlotReader`] and [`WakerSlotWriter`] for details.

use super::RRTWaker;
use std::sync::{Arc, Mutex};

/// Shared waker slot - an [`Arc<Mutex<Option<K>>>`] that serves as the liveness signal.
///
/// - `Some(waker)` = thread is [running].
/// - `None` = thread is [terminated or not started].
///
/// [running]: super::LivenessState::Running
/// [terminated or not started]: super::LivenessState::TerminatedOrNotStarted
pub type SharedWakerSlot<K> = Arc<Mutex<Option<K>>>;

/// Read-only view of a [`SharedWakerSlot`] - can lock and read the waker, but not mutate
/// the slot. Used by [`WakeOnDrop`] to wake the dedicated thread.
///
/// Both this and [`WakerSlotWriter`] wrap the same [`Arc<Mutex<Option<K>>>`]. The
/// newtypes provide compile-time access control, not lock semantics.
///
/// Convert from a [`SharedWakerSlot`] via its [`From` impl].
///
/// [`From` impl]: WakerSlotReader#impl-From<%26Arc<Mutex<Option<K>>>>-for-WakerSlotReader<K>
///
/// [`WakeOnDrop`]: super::rrt_subscriber_guard::WakeOnDrop
#[allow(missing_debug_implementations)]
pub struct WakerSlotReader<K: RRTWaker> {
    pub inner: SharedWakerSlot<K>,
}

mod waker_slot_reader_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl<K: RRTWaker> WakerSlotReader<K> {
        /// Locks the slot, reads the waker, and calls
        /// [`wake_and_unblock_dedicated_thread()`] if present.
        ///
        /// If the thread has already exited, the waker is [`None`] (cleared by
        /// [`TerminationGuard::drop()`]), so the wake call is skipped.
        ///
        /// [`TerminationGuard::drop()`]: super::TerminationGuard#method.drop
        /// [`wake_and_unblock_dedicated_thread()`]:
        ///     super::RRTWaker::wake_and_unblock_dedicated_thread
        pub fn wake_if_present(&self) {
            let Ok(guard) = self.inner.lock() else {
                return;
            };
            let Some(waker) = guard.as_ref() else { return };
            waker.wake_and_unblock_dedicated_thread();
        }
    }
}

/// Mutable access to a [`SharedWakerSlot`] - can set or clear the waker. Used by
/// [`TerminationGuard`] (clear on exit) and [`run_worker_loop()`] (set on restart).
///
/// Both this and [`WakerSlotReader`] wrap the same [`Arc<Mutex<Option<K>>>`]. The
/// newtypes provide compile-time access control, not lock semantics.
///
/// Convert from a [`SharedWakerSlot`] via its [`From` impl].
///
/// [`From` impl]: WakerSlotWriter#impl-From<%26Arc<Mutex<Option<K>>>>-for-WakerSlotWriter<K>
///
/// [`TerminationGuard`]: super::TerminationGuard
/// [`run_worker_loop()`]: super::run_worker_loop
#[allow(missing_debug_implementations)]
pub struct WakerSlotWriter<K: RRTWaker> {
    pub inner: SharedWakerSlot<K>,
}

mod waker_slot_writer_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl<K: RRTWaker> WakerSlotWriter<K> {
        /// Clears the waker to [`None`].
        pub fn clear(&self) {
            if let Ok(mut guard) = self.inner.lock() {
                *guard = None;
            }
        }

        /// Sets the waker to `Some(waker)`.
        pub fn set(&self, waker: K) {
            if let Ok(mut guard) = self.inner.lock() {
                *guard = Some(waker);
            }
        }
    }
}

mod convert_slot_to_reader {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl<K: RRTWaker> Clone for WakerSlotReader<K> {
        fn clone(&self) -> Self {
            Self {
                inner: Arc::clone(&self.inner),
            }
        }
    }

    /// Converts a reference to [`SharedWakerSlot`] into a [`WakerSlotReader`] by
    /// cloning the [`Arc`] internally. This is the primary conversion used at call
    /// sites - no `.clone()` needed by the caller.
    impl<K: RRTWaker> From<&SharedWakerSlot<K>> for WakerSlotReader<K> {
        fn from(slot: &SharedWakerSlot<K>) -> Self {
            Self {
                inner: Arc::clone(slot),
            }
        }
    }

    /// Converts an owned [`SharedWakerSlot`] into a [`WakerSlotReader`].
    impl<K: RRTWaker> From<SharedWakerSlot<K>> for WakerSlotReader<K> {
        fn from(slot: SharedWakerSlot<K>) -> Self { Self { inner: slot } }
    }
}

mod convert_slot_to_writer {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl<K: RRTWaker> Clone for WakerSlotWriter<K> {
        fn clone(&self) -> Self {
            Self {
                inner: Arc::clone(&self.inner),
            }
        }
    }

    /// Converts a reference to [`SharedWakerSlot`] into a [`WakerSlotWriter`] by
    /// cloning the [`Arc`] internally.
    impl<K: RRTWaker> From<&SharedWakerSlot<K>> for WakerSlotWriter<K> {
        fn from(shared_waker_slot: &SharedWakerSlot<K>) -> Self {
            Self {
                inner: Arc::clone(shared_waker_slot),
            }
        }
    }

    /// Converts an owned [`SharedWakerSlot`] into a [`WakerSlotWriter`].
    impl<K: RRTWaker> From<SharedWakerSlot<K>> for WakerSlotWriter<K> {
        fn from(shared_waker_slot: SharedWakerSlot<K>) -> Self {
            Self {
                inner: shared_waker_slot,
            }
        }
    }
}
