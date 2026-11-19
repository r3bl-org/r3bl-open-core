// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

// XMARK: Nested #[cfg] attributes combine with logical AND.

//! Shared HTTP response payloads, manifest helpers, and [`mockito`]-backed local HTTP
//! server fixtures for [`integration_tests`].
//!
//! # Hierarchical `cfg` Gating
//!
//! Nested `#[cfg]` attributes combine with logical AND: an item only compiles when both
//! its parent's gate and its own gate pass.
//!
//! The parent [`integration_tests`] module uses `#[cfg(any(test, doc))]` (`test || doc`),
//! keeping fixtures out of release builds while letting `cargo doc` resolve links to
//! [`constants`] and [`jsonc_golden`][1].
//!
//! However, `http_server` depends on [`mockito`], a `[dev-dependency]` that Cargo never
//! links during `cargo doc`. Gating `http_server` with `#[cfg(test)]` narrows its
//! condition to `(test || doc) && test == test`, so `cargo doc` skips it without
//! missing-crate errors.
//!
//! [1]: mod@crate::self_upgrade::integration_tests::test_fixtures::jsonc_golden
//! [`constants`]: mod@crate::self_upgrade::integration_tests::test_fixtures::constants
//! [`integration_tests`]: mod@crate::self_upgrade::integration_tests
//! [`mockito`]: https://docs.rs/mockito

#![rustfmt::skip]

pub mod constants;
pub mod jsonc_golden;
#[cfg(test)]
pub mod http_server;

pub use constants::*;
pub use jsonc_golden::*;
#[cfg(test)]
pub use http_server::*;
