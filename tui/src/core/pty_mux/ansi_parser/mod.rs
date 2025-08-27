// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! ANSI/VT sequence parsing for PTY multiplexer.
//!
//! This module processes ANSI sequences from PTY output and updates an `OffscreenBuffer`
//! accordingly. It uses the `vte` crate (same as `Alacritty`) for robust ANSI parsing.

#[rustfmt::skip] // Reorder the following for better readability

// Attach.
pub mod ansi_parser_perform_impl;
pub mod ansi_parser_public_api;
pub mod ansi_to_tui_color;
pub mod csi_codes;
pub mod esc_codes;
pub mod term_units;

// Re-export.
pub use ansi_parser_public_api::*;

// Test modules.
#[cfg(test)]
mod ansi_parser_perform_impl_tests;
