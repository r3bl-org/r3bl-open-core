// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Terminal raw mode implementation for ANSI terminals.
//!
//! This module provides functionality to enable and disable raw mode on terminals,
//! which is essential for reading ANSI escape sequences character-by-character
//! without line buffering or terminal interpretation.
//!
//! ## Raw Mode vs Cooked Mode
//!
//! **Cooked Mode** (default):
//! - Input is line-buffered (waits for Enter key)
//! - Special characters are interpreted (Ctrl+C, Ctrl+D, etc.)
//! - ANSI escape sequences may be processed by the terminal
//! - Echoing is enabled (typed characters appear on screen)
//!
//! **Raw Mode**:
//! - No line buffering - bytes available immediately
//! - No special character processing - all bytes pass through
//! - No echo - typed characters don't automatically appear
//! - Perfect for reading ANSI escape sequences and building TUIs
//!
//! ## Platform Support
//!
//! - **Unix/Linux/macOS**: Uses rustix's safe termios API
//! - **Windows**: Not yet implemented (TODO)
//!
//! ## Usage Example
//!
//! The recommended way to use raw mode is with the [`RawModeGuard`]:
//!
//! ```no_run
//! use r3bl_tui::RawModeGuard;
//!
//! {
//!     let _guard = RawModeGuard::new().expect("Failed to enable raw mode");
//!     // Terminal is now in raw mode
//!     // ... process ANSI escape sequences ...
//! } // Raw mode automatically disabled when guard is dropped
//! ```
//!
//! Alternatively, you can manually control raw mode:
//!
//! ```no_run
//! use r3bl_tui::{enable_raw_mode, disable_raw_mode};
//!
//! enable_raw_mode().expect("Failed to enable raw mode");
//! // ... process input ...
//! disable_raw_mode().expect("Failed to disable raw mode");
//! ```

// Private modules (hide internal structure).
mod raw_mode_core;
mod raw_mode_unix;
mod raw_mode_windows;

// Re-export the public API (flat, ergonomic surface).
pub use raw_mode_core::*;

// Conditional re-export for automated integration tests.
#[cfg(any(test, doc))]
pub mod integration_tests;

// Conditional re-export for manual validation tests.
#[cfg(any(test, doc))]
pub mod validation_tests;
