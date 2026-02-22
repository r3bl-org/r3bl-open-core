// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Newtype wrappers for compile-time read/write access enforcement on
//! [`SharedWakerSlot`]. See [`WakerSlotReader`] and [`WakerSlotWriter`] for details.

use super::RRTWaker;
use std::sync::{Arc, Mutex};

/// Shared waker slot - an [`Arc<Mutex<Option<K>>>`] that serves as the liveness
/// signal. See [`TerminationGuard`] for lifecycle details.
///
/// [`TerminationGuard`]: super::TerminationGuard
pub type SharedWakerSlot<K> = Arc<Mutex<Option<K>>>;

/// Read-only view of a [`SharedWakerSlot`]. See [`SubscriberGuard`] for lifecycle
/// details.
///
/// [`SubscriberGuard`]: super::SubscriberGuard
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

/// Mutable access to a [`SharedWakerSlot`]. See [`TerminationGuard`] for lifecycle
/// details.
///
/// [`TerminationGuard`]: super::TerminationGuard
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

    /// Converts a [`SharedWakerSlot`] reference into [`WakerSlotReader`]
    impl<K: RRTWaker> From<&SharedWakerSlot<K>> for WakerSlotReader<K> {
        fn from(slot: &SharedWakerSlot<K>) -> Self {
            Self {
                inner: Arc::clone(slot),
            }
        }
    }

    /// Converts a [`SharedWakerSlot`] (owned) into a [`WakerSlotReader`].
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

    /// Converts a [`SharedWakerSlot`] reference into a [`WakerSlotWriter`]
    impl<K: RRTWaker> From<&SharedWakerSlot<K>> for WakerSlotWriter<K> {
        fn from(shared_waker_slot: &SharedWakerSlot<K>) -> Self {
            Self {
                inner: Arc::clone(shared_waker_slot),
            }
        }
    }

    /// Converts a [`SharedWakerSlot`] (owned) into a [`WakerSlotWriter`].
    impl<K: RRTWaker> From<SharedWakerSlot<K>> for WakerSlotWriter<K> {
        fn from(shared_waker_slot: SharedWakerSlot<K>) -> Self {
            Self {
                inner: shared_waker_slot,
            }
        }
    }
}
