// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Windows implementation of raw mode using Windows Console API.
//!
//! TODO: Implement using Windows Console API for complete raw mode support.
//! Currently returns errors as Windows support is not yet implemented.

/// Enable raw mode on Windows (TODO: implement using Windows Console API).
///
/// # Errors
///
/// Panics with unimplemented message as Windows support is still being developed.
pub fn enable_raw_mode() -> miette::Result<()> {
    unimplemented!("Windows raw mode not yet implemented")
}

/// Disable raw mode on Windows (TODO: implement using Windows Console API).
///
/// # Errors
///
/// Panics with unimplemented message as Windows support is still being developed.
pub fn disable_raw_mode() -> miette::Result<()> {
    unimplemented!("Windows raw mode not yet implemented")
}
