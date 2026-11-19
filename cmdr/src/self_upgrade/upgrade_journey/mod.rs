// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

#![rustfmt::skip]

// Attach.
mod exit_outcome;
pub(super) mod per_process_start_latch; // Rustdoc links from self_upgrade/mod.rs.
mod start_outcome;
mod thread_runner;
mod typestates;
mod upgrade_journey_macro;

// Re-export.
pub use exit_outcome::*;
pub use start_outcome::*;
pub use typestates::*;

#[cfg(any(test, doc))]
pub mod integration_tests;
