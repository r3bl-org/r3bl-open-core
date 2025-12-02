// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Windows implementation of raw mode using Windows Console API.
//!
//! TODO(windows): Implement using Windows Console API for complete raw mode support.
//! Currently returns errors as Windows support is not yet implemented.
//! See: https://learn.microsoft.com/en-us/windows/console/setconsolemode

/// Enable raw mode on Windows.
///
/// TODO(windows): Implement using `SetConsoleMode()` to disable:
/// - `ENABLE_LINE_INPUT` - line buffering
/// - `ENABLE_ECHO_INPUT` - character echo
/// - `ENABLE_PROCESSED_INPUT` - Ctrl+C handling
///
/// # Panics
///
/// Panics with unimplemented message as Windows support is still being developed.
#[allow(dead_code)]
pub fn enable_raw_mode() -> miette::Result<()> {
    unimplemented!("Windows raw mode not yet implemented")
}

/// Disable raw mode on Windows.
///
/// TODO(windows): Implement using `SetConsoleMode()` to restore original console mode.
///
/// # Panics
///
/// Panics with unimplemented message as Windows support is still being developed.
#[allow(dead_code)]
pub fn disable_raw_mode() -> miette::Result<()> {
    unimplemented!("Windows raw mode not yet implemented")
}
