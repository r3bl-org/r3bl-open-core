// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Extension traits for atomic types ([`AtomicBool`], [`AtomicU8`]) with ergonomic
//! methods for common operations. See [`AtomicBoolExt`] and [`AtomicU8Ext`] for details.
//!
//! [`AtomicBool`]: std::sync::atomic::AtomicBool
//! [`AtomicU8`]: std::sync::atomic::AtomicU8

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

/// Ergonomic helpers for [`AtomicBool`] that hide [`SeqCst`] boilerplate.
///
/// All operations use [`SeqCst`] ordering so callers never have to choose.
///
/// [`AtomicBool`]: std::sync::atomic::AtomicBool
/// [`SeqCst`]: std::sync::atomic::Ordering::SeqCst
pub trait AtomicBoolExt {
    /// Reads the current value.
    fn get(&self) -> bool;

    /// Writes `value`.
    fn set(&self, value: bool);

    /// Attempts to acquire a single-instance lease or gate by atomically transitioning
    /// the flag from `false` to `true`.
    ///
    /// - Returns `Some(())` if the lease was successfully acquired (value was `false`).
    /// - Returns `None` if the lease was already held (value was already `true`).
    ///
    /// This prevents Time-Of-Check to Time-Of-Use ([TOCTOU]) race conditions by combining
    /// the check and the state transition into a single, indivisible hardware atomic
    /// operation.
    ///
    /// [TOCTOU]: https://en.wikipedia.org/wiki/Time-of-check_to_time-of-use
    fn try_acquire(&self) -> Option<()>;

    /// Releases a previously acquired lease or gate by atomically resetting the flag to
    /// `false`.
    fn release(&self);
}

impl AtomicBoolExt for AtomicBool {
    fn get(&self) -> bool { self.load(Ordering::SeqCst) }

    fn set(&self, value: bool) { self.store(value, Ordering::SeqCst) }

    fn try_acquire(&self) -> Option<()> {
        let was_already_held = self.swap(true, Ordering::SeqCst);
        if was_already_held { None } else { Some(()) }
    }

    fn release(&self) { self.set(false); }
}

/// Ergonomic helpers for [`AtomicU8`] that hide [`SeqCst`] boilerplate and the
/// [`fetch_add`] return-value quirk.
///
/// All operations use [`SeqCst`] ordering so callers never have to choose.
///
/// ## The `fetch_add` quirk
///
/// [`AtomicU8::fetch_add`] atomically adds to the stored value but returns the **old**
/// value, not the new one. [`increment`] works around this by deriving the new value
/// locally via [`u8::wrapping_add`] on the old value - rather than issuing a second load
/// with [`get`]. A separate load would race with other threads' increments and could
/// return someone else's value.
///
/// ```text
///              Thread A              Thread B          Stored
///              --------              --------          ------
///                                                        5
///  fetch_add(1) -> old=5                                 6
///                              fetch_add(1) -> old=6     7
///
///  // Bad: self.get() returns 7 (Thread B's increment leaked in)
///  // Good: old.wrapping_add(1) returns 6 (derived from own old value)
/// ```
///
/// [`AtomicU8::fetch_add`]: std::sync::atomic::AtomicU8::fetch_add
/// [`AtomicU8`]: std::sync::atomic::AtomicU8
/// [`fetch_add`]: std::sync::atomic::AtomicU8::fetch_add
/// [`get`]: Self::get
/// [`increment`]: Self::increment
/// [`SeqCst`]: std::sync::atomic::Ordering::SeqCst
pub trait AtomicU8Ext {
    /// Atomically increments the counter and returns the **new** value.
    ///
    /// Wraps from `255` to `0`.
    fn increment(&self) -> u8;

    /// Reads the current value.
    fn get(&self) -> u8;

    /// Writes `value`.
    fn set(&self, value: u8);
}

impl AtomicU8Ext for AtomicU8 {
    /// See [the `fetch_add` quirk][quirk] for why this avoids a
    /// second load.
    ///
    /// [quirk]: AtomicU8Ext#the-fetch_add-quirk
    fn increment(&self) -> u8 { self.fetch_add(1, Ordering::SeqCst).wrapping_add(1) }

    fn get(&self) -> u8 { self.load(Ordering::SeqCst) }

    fn set(&self, value: u8) { self.store(value, Ordering::SeqCst) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LossyConvertToByte;
    use rustc_hash::FxHashSet;
    use std::{sync::Arc, thread};

    #[test]
    fn bool_get_returns_initial_value() {
        let flag = AtomicBool::new(false);
        assert!(!flag.get());

        let flag_true = AtomicBool::new(true);
        assert!(flag_true.get());
    }

    #[test]
    fn bool_set_updates_value() {
        let flag = AtomicBool::new(false);
        flag.set(true);
        assert!(flag.get());
        flag.set(false);
        assert!(!flag.get());
    }

    #[test]
    fn bool_try_acquire_and_release() {
        let flag = AtomicBool::new(false);

        // 1. Initial acquire succeeds.
        assert_eq!(flag.try_acquire(), Some(()));
        assert!(flag.get());

        // 2. Subsequent acquire fails while held.
        assert_eq!(flag.try_acquire(), None);
        assert!(flag.get());

        // 3. Release resets flag to false.
        flag.release();
        assert!(!flag.get());

        // 4. Can acquire again after release.
        assert_eq!(flag.try_acquire(), Some(()));
        assert!(flag.get());
    }

    #[test]
    fn get_returns_initial_value() {
        let counter = AtomicU8::new(42);
        assert_eq!(counter.get(), 42);
    }

    #[test]
    fn set_updates_value() {
        let counter = AtomicU8::new(0);
        counter.set(99);
        assert_eq!(counter.get(), 99);
    }

    #[test]
    fn increment_returns_new_value() {
        let counter = AtomicU8::new(0);
        assert_eq!(counter.increment(), 1);
        assert_eq!(counter.increment(), 2);
        assert_eq!(counter.get(), 2);
    }

    #[test]
    fn increment_wraps_at_255() {
        let counter = AtomicU8::new(255);
        assert_eq!(counter.increment(), 0);
        assert_eq!(counter.get(), 0);
    }

    /// Exercises the [`fetch_add` quirk][AtomicU8Ext#the-fetch_add-quirk]: when multiple
    /// threads call [`increment`] concurrently, every return value must be unique. A
    /// naive implementation using a second `get()` would let two threads observe the same
    /// "new" value.
    ///
    /// [`increment`]: AtomicU8Ext::increment
    #[test]
    fn concurrent_increments_return_unique_values() {
        const MAX_THREAD_COUNT: usize = 8;
        const INCREMENTS_PER_THREAD: usize = 30;
        // 8 * 30 = 240, fits in u8 without wrapping so every value is distinct.
        const TOTAL: usize = MAX_THREAD_COUNT * INCREMENTS_PER_THREAD;

        let counter = Arc::new(AtomicU8::new(0));

        let handles: Vec<_> = (0..MAX_THREAD_COUNT)
            .map(|_| {
                thread::spawn({
                    let counter = Arc::clone(&counter);
                    move || {
                        let mut seen = Vec::with_capacity(INCREMENTS_PER_THREAD);
                        for _ in 0..INCREMENTS_PER_THREAD {
                            seen.push(counter.increment());
                        }
                        seen
                    }
                })
            })
            .collect();

        let all_values: Vec<u8> = handles
            .into_iter()
            .flat_map(|h| h.join().expect("conversion error"))
            .collect();

        // Every returned value must be unique - this is the core guarantee that
        // the wrapping_add approach provides over a separate load.
        let unique: FxHashSet<u8> = all_values.iter().copied().collect();
        assert_eq!(
            unique.len(),
            TOTAL,
            "duplicate return values detected: got {} unique out of {} total",
            unique.len(),
            TOTAL,
        );

        // The final stored value must equal the total number of increments.
        let expected_total: u8 = TOTAL.to_u8_lossy();
        assert_eq!(counter.get(), expected_total);
    }

    /// Verifies that the final counter is consistent after concurrent increments that
    /// wrap past `u8::MAX`.
    #[test]
    fn concurrent_increments_wrap_correctly() {
        const MAX_THREAD_COUNT: usize = 4;
        const INCREMENTS_PER_THREAD: usize = 100;
        // 4 * 100 = 400, wraps: 400 % 256 = 144.
        let expected_final: u8 =
            (MAX_THREAD_COUNT * INCREMENTS_PER_THREAD % 256).to_u8_lossy();

        let counter = Arc::new(AtomicU8::new(0));

        let handles: Vec<_> = (0..MAX_THREAD_COUNT)
            .map(|_| {
                thread::spawn({
                    let counter = Arc::clone(&counter);
                    move || {
                        for _ in 0..INCREMENTS_PER_THREAD {
                            counter.increment();
                        }
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().expect("conversion error");
        }

        assert_eq!(counter.get(), expected_final);
    }
}
