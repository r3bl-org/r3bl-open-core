// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Windows implementation of raw mode using Crossterm.
//!
//! On Windows, we delegate to Crossterm's raw mode implementation which handles
//! the Windows Console API (`SetConsoleMode()`) correctly.

use miette::IntoDiagnostic as _;

/// Enable raw mode on Windows using Crossterm.
///
/// Delegates to [`crossterm::terminal::enable_raw_mode()`] which handles the
/// Windows Console API (`SetConsoleMode()`) to disable:
/// - `ENABLE_LINE_INPUT` - line buffering
/// - `ENABLE_ECHO_INPUT` - character echo
/// - `ENABLE_PROCESSED_INPUT` - Ctrl+C handling
///
/// # Errors
///
/// Returns an error if the console mode cannot be changed.
pub fn enable_raw_mode() -> miette::Result<()> {
    crossterm::terminal::enable_raw_mode().into_diagnostic()
}

/// Disable raw mode on Windows using Crossterm.
///
/// Delegates to [`crossterm::terminal::disable_raw_mode()`] which restores
/// the original console mode.
///
/// # Errors
///
/// Returns an error if the console mode cannot be restored.
pub fn disable_raw_mode() -> miette::Result<()> {
    crossterm::terminal::disable_raw_mode().into_diagnostic()
}
