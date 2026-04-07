// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Constants for [`DeadlockPreventionPolicy`]. See [`ADDRESS_SIZE`], [`ANY_SCORE`],
//! [`SPECIFIC_SCORE`], [`OPT_OUT_SCORE`] for details.
//!
//! [`DeadlockPreventionPolicy`]: crate::DeadlockPreventionPolicy

/// The maximum number of simultaneous locks that can be tracked in the [`SharedLedger`].
/// 
/// [`SharedLedger`]: crate::SharedLedger
pub const ADDRESS_SIZE: usize = 4;

/// Specificity value of [`PanicOnAnyLockNesting`].
///
/// [`PanicOnAnyLockNesting`]: crate::DeadlockPreventionPolicy::PanicOnAnyLockNesting
pub const ANY_SCORE: u8 = 100;

/// Specificity value of [`PanicOnSpecificLockNesting`].
///
/// [`PanicOnSpecificLockNesting`]:
///     crate::DeadlockPreventionPolicy::PanicOnSpecificLockNesting
pub const SPECIFIC_SCORE: u8 = 10;

/// Specificity value of [`OptOut`].
///
/// [`OptOut`]: crate::DeadlockPreventionPolicy::OptOut
pub const OPT_OUT_SCORE: u8 = 0;
