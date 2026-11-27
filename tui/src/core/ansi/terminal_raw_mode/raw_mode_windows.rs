// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Windows implementation of raw mode using Crossterm.
//!
//! On Windows, we delegate to Crossterm's raw mode implementation which handles
//! the Windows Console API (`SetConsoleMode()`) correctly.

use super::RawModeError;

/// Enables raw mode on Windows using Crossterm.
///
/// Delegates to [`crossterm::terminal::enable_raw_mode()`] which handles the
/// Windows Console API (`SetConsoleMode()`) to disable:
/// - `ENABLE_LINE_INPUT` - line buffering
/// - `ENABLE_ECHO_INPUT` - character echo
/// - `ENABLE_PROCESSED_INPUT` - Ctrl+C handling
///
/// # Errors
///
/// Returns [`RawModeError::BackendIo`] if the console mode cannot be changed.
pub fn try_enable_raw_mode() -> Result<(), RawModeError> {
    crossterm::terminal::enable_raw_mode().map_err(RawModeError::BackendIo)
}

/// Disable raw mode on Windows using Crossterm.
///
/// Delegates to [`crossterm::terminal::disable_raw_mode()`] which restores
/// the original console mode.
///
/// # Errors
///
/// Returns [`RawModeError::BackendIo`] if the console mode cannot be restored.
pub fn try_disable_raw_mode() -> Result<(), RawModeError> {
    crossterm::terminal::disable_raw_mode().map_err(RawModeError::BackendIo)
}
