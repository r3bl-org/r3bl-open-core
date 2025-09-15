// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! ANSI/VT sequence parsing for PTY multiplexer.
//!
//! This module processes ANSI sequences from PTY output and updates an `OffscreenBuffer`
//! accordingly. It uses the `vte` crate (same as `Alacritty`) for robust ANSI parsing.

// Attach.
pub mod ansi_parser_public_api;
pub mod ansi_to_tui_color;
pub mod operations;
pub mod param_utils;
pub mod perform;
pub mod protocols;
pub mod term_units;

// Re-export.
pub use ansi_parser_public_api::*;
pub use operations::*;
pub use param_utils::*;
pub use protocols::*;
pub use term_units::*;

// Integration test modules.
#[cfg(test)]
mod ansi_integration_tests;
