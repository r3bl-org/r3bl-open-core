//! ANSI Terminal Abstraction Layer
//!
//! This module provides bidirectional ANSI sequence handling for terminal emulation:
//!
//! ## Key Subsystems
//!
//! - **Parser** ([`parser`]): Convert incoming PTY output (ANSI sequences) → terminal
//!   state
//! - **Generator** ([`generator`]): Convert app styling → outgoing ANSI sequences
//! - **Color** ([`color`]): Color type definitions and conversions (RGB ↔ ANSI256)
//! - **Terminal Output** ([`terminal_output`]): I/O operations for writing to terminal
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
//! use r3bl_tui::core::ansi::parser;
//!
//! let sequence = parser::CsiSequence::cursor_position_report(10, 5);
//! ```
//!
//! ### Color Conversions
//! ```ignore
//! use r3bl_tui::core::ansi::color::{RgbValue, AnsiValue};
//!
//! let rgb = RgbValue { r: 255, g: 128, b: 64 };
//! let ansi = rgb.to_ansi();  // Convert to nearest ANSI color
//! ```
//!
//! ## Module Organization
//!
//! - **`color/`** - Type-safe color representations and conversions
//! - **`constants/`** - ANSI/VT100 escape sequence constants
//! - **`generator/`** - ANSI sequence generation (`SgrCode`, `CliTextInline`)
//! - **`parser/`** - ANSI sequence parsing (performer, protocols, operations)
//! - **`terminal_output.rs`** - I/O operations

pub mod color;
pub mod constants;
pub mod generator;
pub mod parser;
pub mod terminal_output;

// Color support detection module
mod detect_color_support;

// Re-export key types for ergonomics
pub use color::*;
pub use constants::*;
// Color support detection and constants from detect_color_support module
pub use detect_color_support::{ColorSupport, HyperlinkSupport, Stream,
                               examine_env_vars_to_determine_color_support,
                               examine_env_vars_to_determine_hyperlink_support,
                               global_color_support, global_hyperlink_support};
pub use generator::{// Constants
                    CRLF_BYTES,
                    // Main types
                    CliTextInline,
                    CliTextLine,
                    CliTextLines,
                    DsrRequestFromPtyEvent,
                    DsrRequestType,
                    DsrSequence,
                    // Builder enums
                    EscSequence,
                    SGR_RESET_BYTES,
                    SgrCode,
                    // Style helpers
                    bold,
                    // Main CLI text function
                    cli_text_inline,
                    // Submodules
                    cli_text_inline_impl,
                    dim,
                    dim_underline,
                    fg_black,
                    fg_blue,
                    fg_bright_cyan,
                    // Color functions - basic
                    fg_color,
                    fg_cyan,
                    // Color functions - dark shades
                    fg_dark_gray,
                    fg_dark_lizard_green,
                    fg_dark_pink,
                    fg_dark_purple,
                    fg_dark_teal,
                    fg_frozen_blue,
                    fg_green,
                    fg_guards_red,
                    fg_hot_pink,
                    fg_lavender,
                    fg_light_cyan,
                    fg_light_purple,
                    fg_light_yellow_green,
                    fg_lizard_green,
                    fg_magenta,
                    // Color functions - light/bright shades
                    fg_medium_gray,
                    // Color functions - custom/themed
                    fg_orange,
                    fg_pink,
                    fg_red,
                    fg_silver_metallic,
                    fg_sky_blue,
                    fg_slate_gray,
                    fg_soft_pink,
                    fg_white,
                    fg_yellow,
                    italic,
                    underline};
pub use parser::{AnsiToOfsBufPerformer, CsiSequence};
pub use terminal_output::*;
