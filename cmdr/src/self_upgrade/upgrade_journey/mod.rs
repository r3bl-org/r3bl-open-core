// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

#![rustfmt::skip]

// Private submodules.
mod singleton;
mod start_outcome;
mod thread_runner;
mod typestates;
mod upgrade_journey_macro;

// Public barrel re-exports.
pub use singleton::*;
pub use start_outcome::*;
pub use thread_runner::*;
pub use typestates::*;

#[cfg(any(test, doc))]
pub mod integration_tests;
