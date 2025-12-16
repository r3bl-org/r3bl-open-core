// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! ANSI sequence generation engine
//!
//! This module provides builders for converting styled text and terminal operations
//! into ANSI escape sequences for output.
//!
//! ## Key Types
//!
//! - [`SgrCode`] - SGR (Select Graphic Rendition) codes for text styling
//! - [`EscSequence`] - ESC (Escape) sequence builder for cursor and terminal control
//! - [`DsrSequence`] - DSR (Device Status Report) response builder
//! - [`CliTextInline`] - Styled text for CLI output

// Skip rustfmt for rest of file.
// https://stackoverflow.com/a/75910283/2085356
#![cfg_attr(rustfmt, rustfmt_skip)]

// Private modules (hide internal structure).
mod ansi_sequence_generator_output;
mod cli_text;
mod dsr_sequence;
mod esc_sequence;
mod sgr_code;

// Test/doc-only modules.
#[cfg(any(test, doc))]
mod ansi_sequence_generator_input;

// Public re-exports (flat API).
pub use ansi_sequence_generator_output::*;
pub use cli_text::cli_text_inline_impl;

#[cfg(any(test, doc))]
pub use ansi_sequence_generator_input::*;
pub use cli_text::*;
pub use dsr_sequence::*;
pub use esc_sequence::*;
pub use sgr_code::*;
