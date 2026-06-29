// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Types used by [`ScopedMutex`] for deadlock prevention strategies. See
//! [`DeadlockPreventionPolicy`] and [`DeadlockPreventionGuard`]for details.
//!
//! [`ScopedMutex`]: crate::ScopedMutex

mod constants;
mod policy;
mod policy_impl;

pub use constants::*;
pub use policy::*;
pub use policy_impl::*;
