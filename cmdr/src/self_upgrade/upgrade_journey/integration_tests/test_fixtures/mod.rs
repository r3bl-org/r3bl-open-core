// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Shared HTTP response payloads, manifest helpers, and short-lived local HTTP server
//! fixtures for [`integration_tests`].
//!
//! [`integration_tests`]: mod@super

pub mod constants;
pub mod http_server;
pub mod jsonc_golden;

pub use constants::*;
pub use http_server::*;
pub use jsonc_golden::*;
