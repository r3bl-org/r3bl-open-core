// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`SGR`] (Select Graphic Rendition) sequence constants.
//!
//! See [constants module design] for the three-tier architecture.
//!
//! [`SGR`]: crate::SgrCode
//! [constants module design]: mod@crate::constants#design

use crate::define_ansi_const;

/// Reset ([`SGR`] `0`): Complete sequence bytes to reset all text attributes to default.
/// Provides zero-overhead access for performance-critical paths.
///
/// Sequence: `ESC [ 0 m`
///
/// [`SGR`]: crate::SgrCode
pub const SGR_RESET_BYTES: &[u8] = b"\x1b[0m";

define_ansi_const!(@sgr_str : SGR_RESET_STR = ["0m"] =>
    "Reset (SGR 0)" : "Resets all text attributes to default. Avoids runtime format!() calls."
);

// Common text attributes.

define_ansi_const!(@sgr_str : SGR_BOLD_STR = ["1m"] => "Bold (SGR 1)" : "Bold sequence string.");
define_ansi_const!(@sgr_str : SGR_DIM_STR = ["2m"] => "Dim (SGR 2)" : "Dim sequence string.");
define_ansi_const!(@sgr_str : SGR_ITALIC_STR = ["3m"] => "Italic (SGR 3)" : "Italic sequence string.");
define_ansi_const!(@sgr_str : SGR_UNDERLINE_STR = ["4m"] => "Underline (SGR 4)" : "Underline sequence string.");
define_ansi_const!(@sgr_str : SGR_SLOW_BLINK_STR = ["5m"] => "Slow Blink (SGR 5)" : "Slow blink sequence string.");
define_ansi_const!(@sgr_str : SGR_RAPID_BLINK_STR = ["6m"] => "Rapid Blink (SGR 6)" : "Rapid blink sequence string.");
define_ansi_const!(@sgr_str : SGR_INVERT_STR = ["7m"] => "Invert (SGR 7)" : "Invert sequence string.");
define_ansi_const!(@sgr_str : SGR_HIDDEN_STR = ["8m"] => "Hidden (SGR 8)" : "Hidden sequence string.");
define_ansi_const!(@sgr_str : SGR_STRIKETHROUGH_STR = ["9m"] => "Strikethrough (SGR 9)" : "Strikethrough sequence string.");
define_ansi_const!(@sgr_str : SGR_OVERLINE_STR = ["53m"] => "Overline (SGR 53)" : "Overline sequence string.");

// Reset variants.

define_ansi_const!(@sgr_str : SGR_RESET_BOLD_DIM_STR = ["22m"] => "Reset Bold/Dim (SGR 22)" : "Reset bold/dim sequence string.");
define_ansi_const!(@sgr_str : SGR_RESET_ITALIC_STR = ["23m"] => "Reset Italic (SGR 23)" : "Reset italic sequence string.");
define_ansi_const!(@sgr_str : SGR_RESET_UNDERLINE_STR = ["24m"] => "Reset Underline (SGR 24)" : "Reset underline sequence string.");
define_ansi_const!(@sgr_str : SGR_RESET_BLINK_STR = ["25m"] => "Reset Blink (SGR 25)" : "Reset blink sequence string.");
define_ansi_const!(@sgr_str : SGR_RESET_INVERT_STR = ["27m"] => "Reset Invert (SGR 27)" : "Reset invert sequence string.");
define_ansi_const!(@sgr_str : SGR_RESET_HIDDEN_STR = ["28m"] => "Reset Hidden (SGR 28)" : "Reset hidden sequence string.");
define_ansi_const!(@sgr_str : SGR_RESET_STRIKETHROUGH_STR = ["29m"] => "Reset Strikethrough (SGR 29)" : "Reset strikethrough sequence string.");

// Common colors.

define_ansi_const!(@sgr_str : SGR_FG_RED_STR = ["31m"] => "Red Foreground (SGR 31)" : "Red foreground string.");

// Common bright colors (often used in examples).

define_ansi_const!(@sgr_str : SGR_FG_BRIGHT_RED_STR = ["91m"] => "Bright Red Foreground (SGR 91)" : "Bright red foreground string.");
define_ansi_const!(@sgr_str : SGR_FG_BRIGHT_GREEN_STR = ["92m"] => "Bright Green Foreground (SGR 92)" : "Bright green foreground string.");
define_ansi_const!(@sgr_str : SGR_FG_BRIGHT_YELLOW_STR = ["93m"] => "Bright Yellow Foreground (SGR 93)" : "Bright yellow foreground string.");
define_ansi_const!(@sgr_str : SGR_FG_BRIGHT_BLUE_STR = ["94m"] => "Bright Blue Foreground (SGR 94)" : "Bright blue foreground string.");
define_ansi_const!(@sgr_str : SGR_FG_BRIGHT_CYAN_STR = ["96m"] => "Bright Cyan Foreground (SGR 96)" : "Bright cyan foreground string.");

/// Carriage Return + Line Feed (CRLF): Terminal line ending sequence bytes.
///
/// Value: `\r\n`.
///
/// Used to move cursor to beginning of next line in terminal output.
pub const CRLF_BYTES: &[u8] = b"\r\n";
