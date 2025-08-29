// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! ANSI/VT sequence parsing for PTY multiplexer.
//!
//! This module processes ANSI sequences from PTY output and updates an `OffscreenBuffer`
//! accordingly. It uses the `vte` crate (same as `Alacritty`) for robust ANSI parsing.

// Attach.
pub mod ansi_parser_perform_impl;
pub mod ansi_parser_public_api;
pub mod ansi_to_tui_color;
pub mod csi_codes;
pub mod esc_codes;
pub mod term_units;

// Re-export.
pub use ansi_parser_public_api::*;

// Test modules (no `ansi_parser_perform_impl_tests/mod.rs`).
#[cfg(test)]
#[rustfmt::skip] // Keep the ordering of the following lines as is.
mod ansi_parser_perform_impl_tests {
    pub(super) mod tests_fixtures; // Used by all the test modules below.
    mod tests_processor_lifecycle;
    mod tests_character_encoding;
    mod tests_cursor_operations;
    mod tests_control_sequences;
    mod tests_display_operations;
    mod tests_line_wrap_and_scroll_control;
    mod tests_osc_sequences; // <- TODO review
    mod tests_integration; // <- TODO review
}
