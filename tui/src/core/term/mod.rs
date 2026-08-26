// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # Terminal Interactivity and Size Detection
//!
//! Centralized, backend-aware API for detecting terminal interactivity and size.
//!
//! See [`TerminalInteractiveStatus`] and [`check_is_terminal_interactive()`] for the
//! interactivity check matrix and shell pipeline redirection behavior.


// Attach.
mod constants;
pub mod term_api;
pub mod term_api_impl;

// Re-export.
pub use term_api::*;
pub use term_api_impl::*;

// Integration tests.
#[cfg(any(test, doc))]
pub mod term_integration_tests;
