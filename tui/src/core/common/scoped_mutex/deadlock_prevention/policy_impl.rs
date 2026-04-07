// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Types that implement deadlock prevention policies for [`ScopedMutex`]. For details,
//! see:
//! - "The What" - [`DeadlockPreventionPolicy`].
//! - "The How" - [`DeadlockPreventionGuard`], [`SharedLedger`], [`THREAD_LOCAL_LEDGER`]
//!
//! [`DeadlockPreventionGuard`]: super::DeadlockPreventionGuard

use super::{ADDRESS_SIZE, DeadlockPreventionPolicy, constants};
use crate::{ScopedMutex, ok};
use miette::Diagnostic;
use smallvec::SmallVec;
use std::cell::RefCell;

/// State machine (implementing the [shared ledger]) that enforces the safety rules for
/// [`DeadlockPreventionPolicy`] policies.
///
/// Each thread uses its own private (thread-local) [`SharedLedger`] to track every lock
/// it acquires from a [`ScopedMutex`] instance.
///
/// # The State Machine
///
/// 1. No locks held:
///   - **Initial state**
///   - `held_policy`=[`None`]
///   - Allows acquisition of any participating lock.
/// 2. Any lock held:
///   - **Strict exclusivity**
///   - `held_policy`=`Some(`[`PanicOnAnyLockNesting`]`)`
///   - Any further attempt to acquire a participating lock (including this one) will
///     panic.
/// 3. Specific locks held:
///   - **Cascading**
///   - `held_policy`=`Some(`[`PanicOnSpecificLockNesting`]`)`
///   - Allows acquiring other [`PanicOnSpecificLockNesting`] locks as long as the memory
///     address is unique. Panics if [`PanicOnAnyLockNesting`] is requested.
/// 4. Releasing locks:
///   - When a participating lock is released, its address is removed from the ledger.
///   - If no more addresses remain, the ledger resets to the **No locks held** state.
///
/// Participating [`ScopedMutex`] instances are decorated with a specific **coordinate**
/// (variant of [`DeadlockPreventionPolicy`] enum). See the [Parameters] section in
/// [`ScopedMutex`] for more info.
///
/// 1. [`PanicOnAnyLockNesting`].
/// 2. [`PanicOnSpecificLockNesting`].
///
/// [`OptOut`] instances do not participate in the state machine. If the current thread is
/// already holding a lock (from an opted out [`ScopedMutex`]), the ledger will remain in
/// the "No locks held" state and will NOT detect any subsequent nesting.
///
/// The state machine itself is implemented using "specificity" values to keep the logic
/// understandable and easy to follow.
///
/// [`OptOut`]: DeadlockPreventionPolicy::OptOut
/// [`PanicOnAnyLockNesting`]: crate::DeadlockPreventionPolicy::PanicOnAnyLockNesting
/// [`PanicOnSpecificLockNesting`]:
///     crate::DeadlockPreventionPolicy::PanicOnSpecificLockNesting
/// [`ScopedMutex`]: crate::ScopedMutex
/// [Parameters]: crate::ScopedMutex#parameters
/// [shared ledger]: crate::ScopedMutex#the-shared-ledger
#[derive(Debug, Default)]
pub struct SharedLedger {
    /// The policy of the currently held participating locks.
    pub held_policy: Option<DeadlockPreventionPolicy>,

    /// Memory addresses of held locks.
    ///
    /// This is a tiny set of addresses that need to be tracked. Even though
    /// mathematically we can use a set data structure (like [`HashSet`] or [`BTreeSet`])
    /// for the tiny array we manage, a linear search is very efficient for both memory
    /// and CPU.
    ///
    /// [`BTreeSet`]: std::collections::BTreeSet
    /// [`HashSet`]: std::collections::HashSet
    pub addresses: Option<SmallVec<[usize; ADDRESS_SIZE]>>,
}

// XMARK: Clever Rust use of thread_local! and const Thread Local Storage

thread_local! {
    /// The thread-local shared ledger instance that tracks all participating locks
    /// currently held by the current thread.
    ///
    /// See the [shared ledger] section in [`ScopedMutex`] for details on how this is used
    /// to enforce deadlock prevention policies.
    ///
    /// # Why [`RefCell`] and not [`Cell`]
    ///
    /// We use [`RefCell`] (instead of [`Cell`]) because [`SharedLedger`] is non-[`Copy`]
    /// and [`RefCell`] provides better ergonomics for mutable updates while maintaining
    /// state integrity during panics.
    ///
    /// # Why wrap initialization `const` without using `default()`?
    ///
    /// Using a `const` block ensures **constant initialization** of the thread-local
    /// storage (TLS). This allows the compiler to bake the initial state directly into
    /// the TLS template, eliminating the runtime "is-initialized" check on every access.
    /// In a high-frequency lock environment, this significantly reduces the overhead of
    /// deadlock prevention.
    ///
    /// [`Cell`]: std::cell::Cell
    /// [`RefCell`]: std::cell::RefCell
    /// [`ScopedMutex`]: crate::ScopedMutex
    /// [shared ledger]: crate::ScopedMutex#the-shared-ledger
    pub static THREAD_LOCAL_LEDGER: RefCell<SharedLedger> = const {
        RefCell::new(SharedLedger{ held_policy: None, addresses: None })
    };
}

impl SharedLedger {
    /// Attempts to transition the state of the shared ledger by acquiring a new lock.
    ///
    /// This method enforces the deadlock prevention rules based on the "specificity
    /// score" of the `mutex` policy compared to the current state of the ledger. See [The
    /// State Machine] section for details on valid transitions.
    ///
    /// # Errors
    /// Returns a [`SharedLedgerError`] if the transition is invalid
    ///
    /// # Panics
    /// Panics if an illegal state transition is detected. This represents an internal
    /// invariant violation.
    ///
    /// [`AnyLockAcquisitionWhileSpecificLocksHeld`]:
    ///     SharedLedgerError::AnyLockAcquisitionWhileSpecificLocksHeld
    /// [`RecursiveAnyLockAcquisition`]: SharedLedgerError::RecursiveAnyLockAcquisition
    /// [`RecursiveSpecificLockAcquisition`]:
    ///     SharedLedgerError::RecursiveSpecificLockAcquisition
    /// [`SpecificLockAcquisitionWhileAnyHeld`]:
    ///     SharedLedgerError::SpecificLockAcquisitionWhileAnyHeld
    /// [The State Machine]: SharedLedger#the-state-machine
    pub fn try_acquire<S: ?Sized, const POLICY: DeadlockPreventionPolicy>(
        &mut self,
        mutex: &ScopedMutex<S, POLICY>,
    ) -> Result<(), SharedLedgerError> {
        pub use constants::{ANY_SCORE, SPECIFIC_SCORE};

        let requested_score = POLICY.specificity();
        let address = mutex.get_address();

        // Requested OptOut is always allowed and invisible to the ledger.
        if matches!(POLICY, DeadlockPreventionPolicy::OptOut) {
            return ok!();
        }

        let Some(current_policy) = self.held_policy else {
            // Initial acquisition ("no locks held" -> Any or Specific).
            self.held_policy = Some(POLICY);
            // Store scoped mutex (Any or Specific) address. Used in release() later.
            self.addresses = Some(smallvec::smallvec!(address));
            return ok!();
        };

        let current_score = current_policy.specificity();

        match (current_score, requested_score) {
            // Current Any is exclusive. No other participating lock can be acquired.
            (ANY_SCORE, ANY_SCORE) => Err(SharedLedgerError::RecursiveAnyLockAcquisition),

            // Requested Any while holding Specific is forbidden.
            (ANY_SCORE, SPECIFIC_SCORE) => {
                Err(SharedLedgerError::SpecificLockAcquisitionWhileAnyHeld)
            }

            // Requested Any while holding Specific is forbidden.
            (SPECIFIC_SCORE, ANY_SCORE) => {
                Err(SharedLedgerError::AnyLockAcquisitionWhileSpecificLocksHeld)
            }

            // Requested Specific while holding Specific is allowed (nesting).
            (SPECIFIC_SCORE, SPECIFIC_SCORE) => {
                let Some(addresses) = self.addresses.as_mut() else {
                    unreachable!(
                        "Illegal state: addresses must be Some if held_policy is Some"
                    )
                };
                if addresses.contains(&address) {
                    Err(SharedLedgerError::RecursiveSpecificLockAcquisition { address })
                } else {
                    addresses.push(address);
                    ok!()
                }
            }

            _ => unreachable!("Illegal state transition in SharedLedger"),
        }
    }

    /// Transitions the state of the shared ledger back by releasing a lock.
    ///
    /// The ledger state is reset to the **no locks held** state only if the released lock
    /// was the last participating lock held by the thread. See [The State Machine]
    /// section for details.
    ///
    /// [`ScopedMutex`]: crate::ScopedMutex
    /// [The State Machine]: SharedLedger#the-state-machine
    #[allow(clippy::single_match)]
    pub fn release<S: ?Sized, const POLICY: DeadlockPreventionPolicy>(
        &mut self,
        scoped_mutex: &ScopedMutex<S, POLICY>,
    ) {
        // OptOut locks are invisible and not tracked in the ledger.
        if matches!(POLICY, DeadlockPreventionPolicy::OptOut) {
            return;
        }

        if let Some(addresses) = self.addresses.as_mut() {
            // Remove the address of the mutex being released. Keep all addresses except
            // the one we're releasing.
            addresses.retain(|addr| *addr != scoped_mutex.get_address());

            // If no more locks are held, reset the state.
            if addresses.is_empty() {
                self.held_policy = None;
                self.addresses = None;
            }
        }
    }
}

/// Error type for [`SharedLedger`] transitions.
#[derive(Debug, thiserror::Error, Diagnostic, PartialEq)]
pub enum SharedLedgerError {
    /// Attempted to acquire an Any lock while another is already held.
    #[error(
        "Recursive lock detected! PanicOnAnyLockNesting forbids ANY nesting. \
        Any lock already held: true"
    )]
    #[diagnostic(
        code(r3bl_tui::scoped_mutex::recursive_any_lock),
        help(
            "PanicOnAnyLockNesting is the strictest policy. You cannot nest \
            ANY locks with it."
        )
    )]
    RecursiveAnyLockAcquisition,

    /// Attempted to acquire an Any lock while one or more Specific locks are held.
    #[error(
        "Recursive lock detected! PanicOnAnyLockNesting forbids ANY nesting. \
        Specific lock(s) already held: true"
    )]
    #[diagnostic(
        code(r3bl_tui::scoped_mutex::any_lock_while_specific_held),
        help(
            "You cannot acquire an Any lock if a Specific lock is already held \
            on this thread."
        )
    )]
    AnyLockAcquisitionWhileSpecificLocksHeld,

    /// Attempted to acquire a Specific lock while an Any lock is held.
    #[error(
        "Recursive lock detected! Cannot acquire a Specific lock while an Any \
        lock is held."
    )]
    #[diagnostic(
        code(r3bl_tui::scoped_mutex::specific_lock_while_any_held),
        help(
            "You cannot acquire a Specific lock if an Any lock is already held \
            on this thread."
        )
    )]
    SpecificLockAcquisitionWhileAnyHeld,

    /// Attempted to acquire the same Specific lock address twice.
    #[error(
        "Recursive lock detected on ScopedMutex at address {address:x}! This would \
        have deadlocked."
    )]
    #[diagnostic(
        code(r3bl_tui::scoped_mutex::recursive_specific_lock),
        help(
            "You attempted to acquire the exact same ScopedMutex instance twice \
            on the same thread."
        )
    )]
    RecursiveSpecificLockAcquisition { address: usize },
}

#[cfg(test)]
mod tests_shared_ledger_state_machine {
    use super::*;
    use crate::scoped_mutex;
    use DeadlockPreventionPolicy::*;

    #[test]
    fn test_acquire_single_lock() {
        // Any lock.
        {
            let mutex_any = scoped_mutex!(ANY, 0);
            let mut ledger = SharedLedger::default();

            assert!(ledger.held_policy.is_none());
            assert!(ledger.addresses.is_none());

            let result = ledger.try_acquire(&mutex_any);
            assert!(result.is_ok());
            assert_eq!(ledger.held_policy, Some(PanicOnAnyLockNesting));
            assert_eq!(ledger.addresses.as_ref().unwrap().len(), 1);
            assert!(
                ledger
                    .addresses
                    .as_ref()
                    .unwrap()
                    .contains(&mutex_any.get_address())
            );
        }

        // Specific lock.
        {
            let mutex_specific = scoped_mutex!(SPECIFIC, 0);
            let mut ledger = SharedLedger::default();

            assert!(ledger.held_policy.is_none());
            assert!(ledger.addresses.is_none());

            let result = ledger.try_acquire(&mutex_specific);
            assert!(result.is_ok());
            assert_eq!(ledger.held_policy, Some(PanicOnSpecificLockNesting));
            assert_eq!(ledger.addresses.as_ref().unwrap().len(), 1);
            assert_eq!(
                *ledger.addresses.as_ref().unwrap().first().unwrap(),
                mutex_specific.get_address()
            );
        }
    }

    #[test]
    fn test_release_single_lock() {
        // Any lock.
        {
            let mutex_any = scoped_mutex!(ANY, 0);
            let mut ledger = SharedLedger::default();

            assert!(ledger.held_policy.is_none());
            assert!(ledger.addresses.is_none());

            ledger.try_acquire(&mutex_any).unwrap();
            ledger.release(&mutex_any);

            assert!(ledger.held_policy.is_none());
            assert!(ledger.addresses.is_none());
        }

        // Specific lock.
        {
            let mutex_specific = scoped_mutex!(SPECIFIC, 0);
            let mut ledger = SharedLedger::default();

            assert!(ledger.held_policy.is_none());
            assert!(ledger.addresses.is_none());

            ledger.try_acquire(&mutex_specific).unwrap();
            ledger.release(&mutex_specific);

            assert!(ledger.held_policy.is_none());
            assert!(ledger.addresses.is_none());
        }
    }

    #[test]
    fn test_opt_out_is_invisible() {
        // OptOut lock + Any lock.
        {
            let mutex_opt_out = scoped_mutex!(OPT_OUT, 0);
            let mutex_any = scoped_mutex!(ANY, 0);
            let mut ledger = SharedLedger::default();

            // OptOut lock. Can acquire. SEMANTIC NOTE: While the ledger allows this and
            // remains "invisible", a real ScopedMutex using OptOut would deadlock if
            // acquired recursively on the same thread because it bypasses all safety
            // checks.
            assert!(ledger.try_acquire(&mutex_opt_out).is_ok());
            assert_eq!(ledger.held_policy, None);

            // Any lock. Can acquire. SEMANTIC NOTE: While the ledger allows this and
            // remains "invisible", a real ScopedMutex using OptOut would deadlock if
            // acquired recursively on the same thread because it bypasses all safety
            // checks.
            assert!(ledger.try_acquire(&mutex_any).is_ok());
            assert_eq!(ledger.held_policy, Some(PanicOnAnyLockNesting));

            // OptOut lock. Can acquire. SEMANTIC NOTE: While the ledger allows this and
            // remains "invisible", a real ScopedMutex using OptOut would deadlock if
            // acquired recursively on the same thread because it bypasses all safety
            // checks.
            assert!(ledger.try_acquire(&mutex_opt_out).is_ok());
            assert_eq!(ledger.held_policy, Some(PanicOnAnyLockNesting));
        }

        // OptOut lock + Specific lock.
        {
            let mutex_opt_out = scoped_mutex!(OPT_OUT, 0);
            let mutex_specific = scoped_mutex!(SPECIFIC, 0);
            let mut ledger = SharedLedger::default();

            // OptOut lock. Can acquire. SEMANTIC NOTE: While the ledger allows this and
            // remains "invisible", a real ScopedMutex using OptOut would deadlock if
            // acquired recursively on the same thread because it bypasses all safety
            // checks.
            assert!(ledger.try_acquire(&mutex_opt_out).is_ok());
            assert_eq!(ledger.held_policy, None);

            // SEMANTIC NOTE: While the ledger allows this and remains "invisible", a real
            // ScopedMutex using OptOut would deadlock if acquired recursively on the same
            // thread because it bypasses all safety checks.
            assert!(ledger.try_acquire(&mutex_specific).is_ok());
            assert_eq!(ledger.held_policy, Some(PanicOnSpecificLockNesting));

            // OptOut lock. Can acquire. SEMANTIC NOTE: While the ledger allows this and
            // remains "invisible", a real ScopedMutex using OptOut would deadlock if
            // acquired recursively on the same thread because it bypasses all safety
            // checks.
            assert!(ledger.try_acquire(&mutex_opt_out).is_ok());
            assert_eq!(ledger.held_policy, Some(PanicOnSpecificLockNesting));
        }
    }

    #[test]
    fn test_recursive_any_lock_panics() {
        let mutex_any = scoped_mutex!(ANY, 0);
        let mut ledger = SharedLedger::default();

        // First lock acquisition.
        ledger.try_acquire(&mutex_any).unwrap();

        // Second lock acquisition - should error.
        let result = ledger.try_acquire(&mutex_any);
        assert_eq!(result, Err(SharedLedgerError::RecursiveAnyLockAcquisition));
    }

    #[test]
    fn test_multiple_specific_locks() {
        let mutex_specific_1 = scoped_mutex!(SPECIFIC, 0);
        let mutex_specific_2 = scoped_mutex!(SPECIFIC, 0);
        let mut ledger = SharedLedger::default();

        // Ok to acquire specific lock on mutex 1 for thread.
        assert!(ledger.try_acquire(&mutex_specific_1).is_ok());

        // Ok to acquire specific lock on mutex 2 for thread.
        assert!(ledger.try_acquire(&mutex_specific_2).is_ok());

        assert_eq!(ledger.addresses.as_ref().unwrap().len(), 2);
        assert_eq!(
            ledger.addresses.as_ref().unwrap()[0],
            mutex_specific_1.get_address()
        );
        assert_eq!(
            ledger.addresses.as_ref().unwrap()[1],
            mutex_specific_2.get_address()
        );

        // Try and acquire a lock on mutex_specific_1. This should error out.
        let result = ledger.try_acquire(&mutex_specific_1);
        assert_eq!(
            result,
            Err(SharedLedgerError::RecursiveSpecificLockAcquisition {
                address: mutex_specific_1.get_address()
            })
        );
    }

    #[test]
    fn test_try_acquire_any_while_specific_held_panics() {
        let mutex_specific = scoped_mutex!(SPECIFIC, 0);
        let mutex_any = scoped_mutex!(ANY, 0);
        let mut ledger = SharedLedger::default();

        // Thread acquires mutex_specific.
        ledger.try_acquire(&mutex_specific).unwrap();

        // Thread can't acquire mutex_any.
        let result = ledger.try_acquire(&mutex_any);
        assert!(matches!(
            result,
            Err(SharedLedgerError::AnyLockAcquisitionWhileSpecificLocksHeld)
        ));
    }

    #[test]
    fn test_try_acquire_specific_while_any_held_panics() {
        let mutex_any = scoped_mutex!(ANY, 0);
        let mutex_specific = scoped_mutex!(SPECIFIC, 0);
        let mut ledger = SharedLedger::default();

        // Thread acquires mutex_any.
        ledger.try_acquire(&mutex_any).unwrap();

        // Thread can't acquire mutex_specific.
        let result = ledger.try_acquire(&mutex_specific);
        assert!(matches!(
            result,
            Err(SharedLedgerError::SpecificLockAcquisitionWhileAnyHeld)
        ));
    }
}
