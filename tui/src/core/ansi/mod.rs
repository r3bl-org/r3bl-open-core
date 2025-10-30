/*
 * // Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.
 */

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
//!   PTY Input                             App Output
//!      ↓                                     ↓
//! ┌─────────────┐                     ┌──────────────┐
//! │   Parser    │ ◀─── Constants ───▶ │  Generator   │
//! └─────────────┘    (ANSI specs)     └──────────────┘
//!    ↓                     │                 ↑
//! Terminal State           │            Styled Text
//!                  ┌───────▼────────┐        │
//!                  │ Color types &  │────────┘
//!                  │ Conversion     │
//!                  └────────────────┘
//! ```
//!
//! ## Terminal Input Modes: Raw vs Cooked
//!
//! To understand why this module exists, you need to know how terminals handle input.
//!
//! ### Cooked Mode (Default)
//!
//! This is the **default terminal mode** when you open a shell:
//!
//! ```text
//! You type:        "hello^H^H"  (^H = backspace key)
//!                      ↓
//! OS processes:    character buffering, line editing, special key handling
//!                      ↓
//! Program gets:    "hel" (only after Enter, with backspace processed)
//! ```
//!
//! The OS handles input processing: backspace deletes, Ctrl+C terminates the program,
//! Enter sends the line. The program only receives complete lines.
//!
//! ### Raw Mode (Interactive TUI)
//!
//! Interactive applications (vim, less, this TUI) need **character-by-character input**:
//!
//! ```text
//! You press:       [individual keystroke]
//!                      ↓
//! OS processing:   [NONE - raw bytes sent immediately]
//!                      ↓
//! Program gets:    raw keystroke immediately
//!                  (including escape sequences for arrow keys, Ctrl+C, etc.)
//! ```
//!
//! **Why raw mode?** The program needs to:
//! - Capture every keystroke immediately (no line buffering)
//! - Distinguish between Ctrl+C (user interrupt) vs. Ctrl+C keypress the user wants
//! - Detect special keys (arrows, function keys) sent as **escape sequences**
//! - Control the cursor, colors, and screen layout
//!
//! ### Escape Sequences in Raw Mode
//!
//! When a user presses a special key in raw mode, the terminal sends an **escape sequence**.
//! For example:
//!
//! ```text
//! User presses:    Up arrow
//! Terminal sends:  ESC [ A    (3 bytes: 0x1B 0x5B 0x41)
//! Displayed as:    ^[[A       (when using cat -v to visualize)
//! ```
//!
//! Use `cat -v` to see raw escape sequences:
//!
//! ```text
//! $ cat -v          # cat with visualization of control characters
//! # [user types: "hello" then Up arrow then Left arrow]
//! hello^[[A^[[D
//! # ^[ is the Escape character (ESC, 0x1B)
//! # [A is "cursor up"
//! # [D is "cursor left"
//! ```
//!
//! **Common escape sequences:**
//! - `^[[A` = Up arrow
//! - `^[[B` = Down arrow
//! - `^[[C` = Right arrow
//! - `^[[D` = Left arrow
//! - `^[[3~` = Delete key
//! - `^[OP` = F1 key
//!
//! This module's parser ([`vt_100_pty_output_parser`])
//! converts these escape sequence bytes into structured events the application can handle.
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
//!
//! [`vt_100_pty_output_parser`]: mod@crate::core::ansi::vt_100_pty_output_parser

// XMARK: Snippet to stop rustfmt from reformatting entire file.

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
pub mod terminal_raw_mode;
// This module is private in non-test, non-doc builds.
#[cfg(not(any(test, doc)))]
mod terminal_raw_mode;

// XMARK: Example for how to conditionally expose private modules for testing and documentation.

// Module is public only when building documentation or tests.
// This allows rustdoc links to work while keeping it private in release builds.
#[cfg(any(test, doc))]
pub mod vt_100_pty_output_parser;
// This module is private in non-test, non-doc builds.
#[cfg(not(any(test, doc)))]
mod vt_100_pty_output_parser;

// Input parsing module - public for protocol access
pub mod vt_100_terminal_input_parser;

// Re-export flat public API.
pub use color::*;
pub use constants::*;
pub use detect_color_support::*;
pub use generator::*;
pub use terminal_output::*;
pub use vt_100_pty_output_parser::*;
pub use terminal_raw_mode::*;

// Re-export test fixtures for testing purposes only.
#[cfg(test)]
pub use vt_100_pty_output_parser::vt_100_pty_output_conformance_tests;
