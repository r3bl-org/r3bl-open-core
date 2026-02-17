// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Extension trait for [`AtomicU8`] with ergonomic methods for common operations. See
//! [`AtomicU8Ext`] for details.
//!
//! [`AtomicU8`]: std::sync::atomic::AtomicU8

use std::sync::atomic::{AtomicU8, Ordering};

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
/// [`SeqCst`]: Ordering::SeqCst
/// [`fetch_add`]: std::sync::atomic::AtomicU8::fetch_add
/// [`get`]: Self::get
/// [`increment`]: Self::increment
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
    /// See [the `fetch_add` quirk][AtomicU8Ext#the-fetch_add-quirk] for why this avoids a
    /// second load.
    fn increment(&self) -> u8 { self.fetch_add(1, Ordering::SeqCst).wrapping_add(1) }

    fn get(&self) -> u8 { self.load(Ordering::SeqCst) }

    fn set(&self, value: u8) { self.store(value, Ordering::SeqCst) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::HashSet, sync::Arc, thread};

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
                let shared_counter = Arc::clone(&counter);
                thread::spawn(move || {
                    let mut seen = Vec::with_capacity(INCREMENTS_PER_THREAD);
                    for _ in 0..INCREMENTS_PER_THREAD {
                        seen.push(shared_counter.increment());
                    }
                    seen
                })
            })
            .collect();

        let all_values: Vec<u8> = handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect();

        // Every returned value must be unique - this is the core guarantee that
        // the wrapping_add approach provides over a separate load.
        let unique: HashSet<u8> = all_values.iter().copied().collect();
        assert_eq!(
            unique.len(),
            TOTAL,
            "duplicate return values detected: got {} unique out of {} total",
            unique.len(),
            TOTAL,
        );

        // The final stored value must equal the total number of increments.
        assert_eq!(counter.get(), TOTAL as u8);
    }

    /// Verifies that the final counter is consistent after concurrent increments that
    /// wrap past `u8::MAX`.
    #[test]
    fn concurrent_increments_wrap_correctly() {
        const MAX_THREAD_COUNT: usize = 4;
        const INCREMENTS_PER_THREAD: usize = 100;
        // 4 * 100 = 400, wraps: 400 % 256 = 144.
        const EXPECTED_FINAL: u8 = (MAX_THREAD_COUNT * INCREMENTS_PER_THREAD % 256) as u8;

        let counter = Arc::new(AtomicU8::new(0));

        let handles: Vec<_> = (0..MAX_THREAD_COUNT)
            .map(|_| {
                let counter = Arc::clone(&counter);
                thread::spawn(move || {
                    for _ in 0..INCREMENTS_PER_THREAD {
                        counter.increment();
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(counter.get(), EXPECTED_FINAL);
    }
}
