// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Deadlock safe [`Mutex`]. See [`ScopedMutex`] and [`scoped_mutex!`] for details.
//!
//! [`Mutex`]: std::sync::Mutex
//! [`scoped_mutex!`]: macro@crate::scoped_mutex
//! [`ScopedMutex`]: super::ScopedMutex

use crate::{DeadlockPreventionPolicy, ScopedMutex};
use std::sync::Mutex;

/// This macro creates a [`ScopedMutex`] ergonomically and is the primary public
/// construction API. This macro is powered by the [`MutexExt`] extension trait.
///
/// # Parameters
///
/// 1. It supports three variants: `ANY`, `SPECIFIC`, and `OPT_OUT`, corresponding to the
///    variants of [`DeadlockPreventionPolicy`] enum.
/// 2. It accepts an expression which is passed to [`Mutex::new`].
///
/// # Examples
///
/// ```
/// use r3bl_tui::{scoped_mutex, ScopedMutex, DeadlockPreventionPolicy::{
///     PanicOnAnyLockNesting, PanicOnSpecificLockNesting, OptOut
/// }};
///
/// let mutex_any: ScopedMutex<i32, { PanicOnAnyLockNesting }> =
///     scoped_mutex!(ANY, 0);
/// let mutex_specific: ScopedMutex<i32, { PanicOnSpecificLockNesting }> =
///     scoped_mutex!(SPECIFIC, 0);
/// let mutex_opt_out: ScopedMutex<i32, { OptOut }> =
///     scoped_mutex!(OPT_OUT, 0);
/// ```
///
/// [`Mutex::new`]: std::sync::Mutex::new
/// [`MutexExt`]: crate::MutexExt
/// [`scoped_mutex!`]: macro@crate::scoped_mutex
#[macro_export]
macro_rules! scoped_mutex {
    (ANY, $val:expr) => {
        $crate::ScopedMutex::<
            _,
            { $crate::DeadlockPreventionPolicy::PanicOnAnyLockNesting },
        >::new($val)
    };
    (SPECIFIC, $val:expr) => {
        $crate::ScopedMutex::<
            _,
            { $crate::DeadlockPreventionPolicy::PanicOnSpecificLockNesting },
        >::new($val)
    };
    (OPT_OUT, $val:expr) => {
        $crate::ScopedMutex::<_, { $crate::DeadlockPreventionPolicy::OptOut }>::new($val)
    };
}

// XMARK: Clever Rust, use a struct `S` in a trait bound with `Into<S>` supertrait bound
// to allow the extension trait to "know" about the specific struct it is designed to
// extend (and get around the orphan rule).

/// Extension trait that converts a [`Mutex`] into a [`ScopedMutex`] wrapper, allowing you
/// to choose a [`DeadlockPreventionPolicy`] (using the `POLICY` const generic).
///
/// This trait also powers the [`scoped_mutex!`] macro, which provides a more concise way
/// to create a [`ScopedMutex`] from a new value.
///
/// # Ergonomic Extension Pattern (Orphan Rule Bypass)
///
/// This trait is designed to extend the standard library's [`Mutex`] with closure-based
/// access. This pattern bypasses the **Orphan Rule** (which prevents adding methods to
/// types from other crates) by:
/// 1. Defining an **Extension Trait** ([`MutexExt`]).
/// 2. Using a **Supertrait Bound** (`Into<Mutex<S>>`) to "bind" the trait specifically to
///    the [`Mutex`] struct.
/// 3. Leveraging **Reflexive Implementations** to allow [`Mutex`] to implement the trait
///    with zero boilerplate.
///
/// This allows you to turn any [`Mutex`] into a [`ScopedMutex`] with a single method
/// call, explicitly selecting your desired `POLICY`, which is a `const` generic parameter
/// holding a value (that is a variant of the [`DeadlockPreventionPolicy`] enum). See the
/// [Parameters] section in [`ScopedMutex`] for more info.
///
/// # Supertrait Bound: `Into<Mutex<S>>`
///
/// This trait uses `Into<Mutex<S>>` as a supertrait bound to allow it to "know" about the
/// specific **struct** (not another trait) it wraps, which is [`Mutex<S>`]. This enables
/// the default implementations below to use `self.into()` to turn `self` (an instance of
/// [`Mutex<S>`]) into a [`Mutex<S>`].
///
/// Since [`Mutex<S>`] automatically implements `Into<Mutex<S>>` (via the standard
/// library's reflexive `impl<T> From<T> for T`), the [`Mutex<S>`] struct itself can
/// implement this trait with zero boilerplate (using an empty `impl` block).
///
/// [`scoped_mutex!`]: macro@crate::scoped_mutex
/// [Parameters]: crate::ScopedMutex#parameters
pub trait MutexExt<S>: Sized + Into<Mutex<S>> {
    /// Creates a [`ScopedMutex`] with the given [`DeadlockPreventionPolicy`].
    fn into_scoped_mutex<const POLICY: DeadlockPreventionPolicy>(
        /* std::sync::Mutex that is consumed */ self,
    ) -> ScopedMutex<S, POLICY> {
        ScopedMutex { state: self.into() }
    }
}

/// Blanket implementation of [`MutexExt`] for [`Mutex`].
impl<S> MutexExt<S> for Mutex<S> {}
