// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Types that implement deadlock prevention policies for [`ScopedMutex`]. For details,
//! see:
//! - "The What" - [`DeadlockPreventionPolicy`].
//! - "The How" - [`DeadlockPreventionGuard`], [`SharedLedger`], [`THREAD_LOCAL_LEDGER`]
//!
//! [`SharedLedger`]: super::SharedLedger

use super::{THREAD_LOCAL_LEDGER,
            constants::{ANY_SCORE, OPT_OUT_SCORE, SPECIFIC_SCORE}};
use crate::ScopedMutex;
use std::marker::ConstParamTy;

/// Declarative strategy used to prevent deadlocks in [`ScopedMutex`].
///
/// Each variant of this enum acts as a **coordinate** in a **type family**, identifying a
/// unique, disjoint member of that family. See the [Parameters] section in
/// [`ScopedMutex`] for more info.
///
/// In order to "enforce" the declared deadlock prevention, the [`ScopedMutex`] [`read()`]
/// and [`write()`] methods use [`DeadlockPreventionGuard`] when acquiring and releasing
/// locks.
///
/// See the [shared ledger] section in [`ScopedMutex`] for details on how different
/// policies interact, and are implemented via a [state machine].
///
/// The `POLICY` [const generic] in [`ScopedMutex`] controls how deadlocks (specifically
/// recursive locks on the same thread) are handled.
///
/// [`read()`]: crate::ScopedMutex::read
/// [`write()`]: crate::ScopedMutex::write
/// [const generic]: crate::ScopedMutex#design-decision-why-const-generics
/// [Parameters]: crate::ScopedMutex#parameters
/// [shared ledger]: crate::ScopedMutex#the-shared-ledger
/// [state machine]: crate::SharedLedger#the-state-machine
#[derive(Debug, Clone, Copy, PartialEq, Eq, ConstParamTy)]
pub enum DeadlockPreventionPolicy {
    /// Strict policy that enforces no lock nesting among "participating" [`ScopedMutex`]
    /// instances on the same thread.
    ///
    /// See [The State Machine] section in [`SharedLedger`] for details on how this policy
    /// affects lock acquisition.
    ///
    /// [`SharedLedger`]: crate::SharedLedger
    /// [The State Machine]: crate::SharedLedger#the-state-machine
    PanicOnAnyLockNesting,

    /// Policy that allows nesting of different "participating" [`ScopedMutex`] instances
    /// but panics on recursive locking of the same instance.
    ///
    /// See [The State Machine] section in [`SharedLedger`] for details on how this policy
    /// affects lock acquisition.
    ///
    /// [`SharedLedger`]: crate::SharedLedger
    /// [The State Machine]: crate::SharedLedger#the-state-machine
    PanicOnSpecificLockNesting,

    /// Completely opts out of deadlock prevention and state machine tracking.
    ///
    /// This is the fastest variant as it performs no [`thread_local!`] lookups or state
    /// transitions. However, it provides no protection against deadlocks and its
    /// acquisition (from a [`ScopedMutex`]) is invisible to other participating locks on
    /// the same thread.
    OptOut,
}

impl DeadlockPreventionPolicy {
    /// Returns the specificity score for the policy.
    ///
    /// Higher scores represent more restrictive policies. This score is used by
    /// [`SharedLedger`] to determine valid state transitions and enforce deadlock
    /// prevention rules.
    ///
    /// CSS Specificity and how cascading styles are computed to a single style, is the
    /// inspiration for the scores.
    ///
    /// [`SharedLedger`]: crate::SharedLedger
    #[must_use]
    pub fn specificity(self) -> u8 {
        match self {
            Self::PanicOnAnyLockNesting => ANY_SCORE,
            Self::PanicOnSpecificLockNesting => SPECIFIC_SCORE,
            Self::OptOut => OPT_OUT_SCORE,
        }
    }
}

/// Private [`RAII`] guard used by [`ScopedMutex::read()`] and [`ScopedMutex::write()`] to
/// automatically acquire and release locks from the [`SharedLedger`].
///
/// - This guard enforces the chosen policy by using the thread-local [`SharedLedger`]
///   during its lifetime.
/// - Like its parent [`ScopedMutex`], this guard is **generic over the policy value**
///   (variant of the [`DeadlockPreventionPolicy`] enum).
/// - It automatically handles both lock acquisition (in [`new()`]) and release (in
///   [`drop()`]).
///
/// # Parameters
///
/// - `POLICY`: A `const` generic parameter holding a variant of the
///   [`DeadlockPreventionPolicy`] enum. This controls the behavior of the
///   [`SharedLedger`] while this guard is alive. See the [Parameters] section in
///   [`ScopedMutex`] for more info.
///
/// [`drop()`]: #method.drop
/// [`new()`]: Self::new()
/// [`RAII`]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
/// [`SharedLedger`]: crate::SharedLedger
/// [`write()`]: crate::ScopedMutex::write
/// [Parameters]: crate::ScopedMutex#parameters
#[derive(Debug)]
pub struct DeadlockPreventionGuard<'a, S, const POLICY: DeadlockPreventionPolicy> {
    /// Reference to the [`ScopedMutex`] being guarded.
    pub scoped_mutex: &'a ScopedMutex<S, POLICY>,
}

use DeadlockPreventionPolicy::OptOut;

impl<'a, S, const POLICY: DeadlockPreventionPolicy>
    DeadlockPreventionGuard<'a, S, POLICY>
{
    /// Creates a new recursion guard for the given [`ScopedMutex`] and updates the
    /// thread-local [`SharedLedger`].
    ///
    /// # Panics
    ///
    /// Panics if the [`DeadlockPreventionPolicy`] is violated (e.g., a recursive lock is
    /// detected).
    ///
    /// [`SharedLedger`]: crate::SharedLedger
    pub fn new(scoped_mutex: &'a ScopedMutex<S, POLICY>) -> Self {
        // 1. Guard clause: early return for OptOut.
        if matches!(POLICY, OptOut) {
            return Self { scoped_mutex };
        }

        // 2. Acquisition attempt (modifies the TLS in-place).
        THREAD_LOCAL_LEDGER
            .with_borrow_mut(|shared_ledger| shared_ledger.try_acquire(scoped_mutex))
            .unwrap_or_else(|err| {
                tracing::error!("{err}");
                panic!("{err}");
            });

        // 3. Success.
        Self { scoped_mutex }
    }
}

impl<S, const POLICY: DeadlockPreventionPolicy> Drop
    for DeadlockPreventionGuard<'_, S, POLICY>
{
    /// Releases the lock from the thread-local [`SharedLedger`].
    ///
    /// This ensures that the ledger state is correctly updated when the guard goes out
    /// of scope, allowing subsequent lock acquisitions on the same thread.
    ///
    /// [`SharedLedger`]: crate::SharedLedger
    fn drop(&mut self) {
        // 1. Guard clause: early return for OptOut.
        if matches!(POLICY, OptOut) {
            return;
        }

        // 2. Release the lock from the ledger.
        THREAD_LOCAL_LEDGER.with_borrow_mut(|shared_ledger| {
            shared_ledger.release(self.scoped_mutex);
        });
    }
}
