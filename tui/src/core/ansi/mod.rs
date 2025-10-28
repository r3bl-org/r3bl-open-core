//! ANSI Terminal Abstraction Layer
//!
//! This module provides bidirectional ANSI sequence handling for terminal emulation:
//!
//! ## Key Subsystems
//!
//! - **Parser**: Convert incoming PTY output (ANSI sequences) → terminal state (via
//!   [`AnsiToOfsBufPerformer`])
//! - **Generator**: Convert app styling → outgoing ANSI sequences (via [`SgrCode`],
//!   [`CliTextInline`])
//! - **Color**: Color type definitions and conversions (RGB ↔ ANSI256)
//! - **Terminal Output**: I/O operations for writing to terminal
//!
//! ## Architecture Overview
//!
//! ```text
//! PTY Input                                  App Output
//!    ↓                                          ↓
//! ┌─────────────┐                       ┌──────────────┐
//! │   Parser    │◀─── Constants ───→   │  Generator   │
//! └─────────────┘        (ANSI         └──────────────┘
//! ↓                      specs)               ↑
//! Terminal State          │           Styled Text
//!                        ↓                    │
//!                    ┌──────────┐            │
//!                    │  Color   │────────────┘
//!                    │Types &   │
//!                    │Conversion│
//!                    └──────────┘
//! ```
//!
//! ## Usage Examples
//!
//! ### Styling Text for Output
//! ```ignore
//! use r3bl_tui::{SgrCode, CliTextInline};
//!
//! let styled = CliTextInline::new("Hello", vec![SgrCode::Bold]);
//! println!("{}", styled);
//! ```
//!
//! ### Parsing ANSI Sequences
//! ```ignore
//! use r3bl_tui::CsiSequence;
//!
//! let sequence = CsiSequence::cursor_position_report(10, 5);
//! ```
//!
//! ### Color Conversions
//! ```ignore
//! use r3bl_tui::{RgbValue, AnsiValue};
//!
//! let rgb = RgbValue { r: 255, g: 128, b: 64 };
//! let ansi = rgb.to_ansi();  // Convert to nearest ANSI color
//! ```
//!
//! ## Key Types and Public API
//!
//! **Color System:**
//! - `TuiColor` - Terminal color with RGB and ANSI256 support
//! - [`RgbValue`], [`AnsiValue`] - Color value types
//!
//! **Text Styling:**
//! - [`SgrCode`] - SGR (Select Graphic Rendition) styling codes
//! - [`CliTextInline`] - Styled inline text for output
//!
//! **ANSI Parsing:**
//! - [`AnsiToOfsBufPerformer`] - Main ANSI parser implementation
//! - [`CsiSequence`] - CSI escape sequence types
//!
//! **Terminal I/O:**
//! - Color detection and support queries

// Skip rustfmt for rest of file.
// https://stackoverflow.com/a/75910283/2085356
#![cfg_attr(rustfmt, rustfmt_skip)]

// Private modules.
mod color;
mod constants;
mod detect_color_support;
mod generator;
mod terminal_output;

// Module is public only when building documentation or tests.
// This allows rustdoc links to work while keeping it private in release builds.
#[cfg(any(test, doc))]
pub mod vt_100_ansi_parser;
// This module is private in non-test, non-doc builds.
#[cfg(not(any(test, doc)))]
mod vt_100_ansi_parser;

// Input parsing module - public for protocol access
pub mod vt_100_terminal_input_parser;

// Re-export flat public API.
pub use color::*;
pub use constants::*;
pub use detect_color_support::*;
pub use generator::*;
pub use terminal_output::*;

// Re-export test fixtures for testing purposes only.
#[cfg(test)]
pub use vt_100_ansi_parser::vt_100_ansi_conformance_tests;
pub use vt_100_ansi_parser::*;
