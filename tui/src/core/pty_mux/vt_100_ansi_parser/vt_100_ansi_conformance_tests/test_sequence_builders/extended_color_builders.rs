// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Test convenience builders for extended color sequences (256-color & RGB).
//!
//! This module provides ergonomic functions for generating extended color ANSI sequences
//! in tests. These are thin wrappers around the [`Display`] implementation for
//! [`SgrColorSequence`] that provide shorter, more readable APIs for test code.
//!
//! # Purpose
//!
//! While [`SgrColorSequence`] provides type-safe bidirectional sequence generation
//! (parsing + display), these builders offer convenience functions specifically for
//! test scenarios where constructing the full enum variant would be verbose.
//!
//! # Color Formats
//!
//! These builders generate sequences using the **colon-separated format** (recommended
//! modern format):
//! - 256-color: `ESC[38:5:196m` (not `ESC[38;5;196m`)
//! - RGB: `ESC[38:2:255:128:0m` (not `ESC[38;2;255;128;0m`)
//!
//! # Example Usage
//!
//! ```rust,ignore
//! use crate::vt_100_ansi_conformance_tests::test_sequence_builders::extended_color_builders::*;
//!
//! // 256-color sequences
//! let fg = fg_ansi256(196);                  // → "\x1b[38:5:196m"
//! let bg = bg_ansi256(21);                   // → "\x1b[48:5:21m"
//!
//! // RGB sequences
//! let orange_fg = fg_rgb(255, 128, 0);       // → "\x1b[38:2:255:128:0m"
//! let blue_bg = bg_rgb(0, 128, 255);         // → "\x1b[48:2:0:128:255m"
//! ```
//!
//! [`SgrColorSequence`]: crate::protocols::csi_codes::SgrColorSequence
//! [`Display`]: std::fmt::Display

use crate::protocols::csi_codes::SgrColorSequence;

/// Generate 256-color foreground sequence: ESC[38:5:nm
///
/// Creates a sequence that sets the foreground color to a 256-color palette index.
///
/// # Parameters
/// - `index`: Color palette index (0-255)
///   - 0-15: Standard ANSI colors
///   - 16-231: 6×6×6 RGB cube
///   - 232-255: Grayscale ramp
///
/// # Returns
/// Formatted ANSI sequence: `\x1b[38:5:{index}m`
///
/// # Example
/// ```rust,ignore
/// let bright_red = fg_ansi256(196);
/// assert_eq!(bright_red, "\x1b[38:5:196m");
/// ```
#[must_use]
pub fn fg_ansi256(index: u8) -> String {
    SgrColorSequence::SetForegroundAnsi256(index).to_string()
}

/// Generate 256-color background sequence: ESC[48:5:nm
///
/// Creates a sequence that sets the background color to a 256-color palette index.
///
/// # Parameters
/// - `index`: Color palette index (0-255)
///
/// # Returns
/// Formatted ANSI sequence: `\x1b[48:5:{index}m`
///
/// # Example
/// ```rust,ignore
/// let blue_bg = bg_ansi256(21);
/// assert_eq!(blue_bg, "\x1b[48:5:21m");
/// ```
#[must_use]
pub fn bg_ansi256(index: u8) -> String {
    SgrColorSequence::SetBackgroundAnsi256(index).to_string()
}

/// Generate RGB foreground sequence: ESC[38:2:r:g:bm
///
/// Creates a sequence that sets the foreground color to a true RGB color.
///
/// # Parameters
/// - `r`: Red component (0-255)
/// - `g`: Green component (0-255)
/// - `b`: Blue component (0-255)
///
/// # Returns
/// Formatted ANSI sequence: `\x1b[38:2:{r}:{g}:{b}m`
///
/// # Example
/// ```rust,ignore
/// let orange = fg_rgb(255, 128, 0);
/// assert_eq!(orange, "\x1b[38:2:255:128:0m");
/// ```
#[must_use]
pub fn fg_rgb(r: u8, g: u8, b: u8) -> String {
    SgrColorSequence::SetForegroundRgb(r, g, b).to_string()
}

/// Generate RGB background sequence: ESC[48:2:r:g:bm
///
/// Creates a sequence that sets the background color to a true RGB color.
///
/// # Parameters
/// - `r`: Red component (0-255)
/// - `g`: Green component (0-255)
/// - `b`: Blue component (0-255)
///
/// # Returns
/// Formatted ANSI sequence: `\x1b[48:2:{r}:{g}:{b}m`
///
/// # Example
/// ```rust,ignore
/// let blue = bg_rgb(0, 128, 255);
/// assert_eq!(blue, "\x1b[48:2:0:128:255m");
/// ```
#[must_use]
pub fn bg_rgb(r: u8, g: u8, b: u8) -> String {
    SgrColorSequence::SetBackgroundRgb(r, g, b).to_string()
}
