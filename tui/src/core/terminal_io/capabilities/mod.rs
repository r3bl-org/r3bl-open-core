// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # Terminal Interactivity and Size Detection
//!
//! Centralized, backend-aware API for detecting terminal interactivity and size.
//!
//! See [`TerminalInteractiveStatus`] and [`check_is_terminal_interactive()`] for the
//! interactivity check matrix and shell pipeline redirection behavior.

// Attach.
pub mod capabilities_impl;
pub mod capabilities_public_api;
mod constants;

// Re-export.
pub use capabilities_impl::*;
pub use capabilities_public_api::*;
pub use constants::*;

// Integration tests.
#[cfg(any(test, doc))]
pub mod capabilities_integration_tests;
