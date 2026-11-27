// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`ANSI`] sequence generation engine.
//!
//! This module provides builders for converting styled text and terminal operations into
//! [`ANSI`] escape sequences for output.
//!
//! ## Key Types
//!
//! - [`SgrCode`] - [`SGR`] (Select Graphic Rendition) codes for text styling
//! - [`EscSequence`] - [`ESC`] (Escape) sequence builder for cursor and terminal control
//! - [`DsrSequence`] - [`DSR`] (Device Status Report) response builder
//! - [`OscSequence`] - [`OSC`] (Operating System Command) sequence builder
//! - [`CliTextInline`] - Styled text for CLI output
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`DSR`]: crate::DsrSequence
//! [`ESC`]: crate::EscSequence
//! [`OSC`]: crate::OscSequence
//! [`OscSequence`]: crate::OscSequence
//! [`SGR`]: crate::SgrCode

#![rustfmt::skip]

// Attach. Public (Inter-crate).
pub mod mouse_sgr;
pub mod mouse_x10;
pub mod ansi_output;

// Attach. Folder-local (Intra-module).
mod cli_text;
mod dsr;
mod esc;
mod sgr_code;
mod da;

// Test fixtures.
#[cfg(any(test, doc))]
mod ansi_input_test_fixtures;

// Re-export.
pub(crate) use cli_text::impl_cli_text_inline;
pub use cli_text::*;
pub use dsr::*;
pub use esc::*;
pub use sgr_code::*;
pub use da::*;

#[cfg(any(test, doc))]
pub use ansi_input_test_fixtures::*;
