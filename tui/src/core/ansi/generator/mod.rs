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
//!
//! ## Usage
//!
//! ```ignore
//! use crate::core::ansi::generator::{SgrCode, EscSequence, DsrSequence, CliTextInline};
//!
//! let styled = CliTextInline::new("Hello", vec![SgrCode::Bold]);
//! let esc = EscSequence::SaveCursor;
//! println!("{}", styled);
//! ```

// Private modules (hide internal structure)
mod cli_text;
mod dsr_sequence;
mod esc_sequence;
mod sgr_code;

// Re-export cli_text_inline_impl from cli_text
// Re-export byte constants from constants module
pub use crate::core::ansi::constants::{CRLF_BYTES, SGR_RESET_BYTES};
pub use cli_text::cli_text_inline_impl;
// Public re-exports (flat API)
pub use cli_text::*;
pub use dsr_sequence::*;
pub use esc_sequence::*;
pub use sgr_code::*;
