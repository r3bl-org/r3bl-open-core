// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Core raw mode functionality and RAII guard.
//!
//! This module provides:
//! - Platform-agnostic public API functions that dispatch to platform-specific
//!   implementations
//! - The `RawModeGuard` RAII wrapper for automatic resource cleanup

// Import platform-specific implementations
#[cfg(unix)]
use super::raw_mode_unix;
#[cfg(windows)]
use super::raw_mode_windows;

/// Enable raw mode on the terminal.
///
/// See [module documentation] module documentation for:
/// - Why raw mode is needed and how it differs from cooked mode
/// - Platform-specific implementation details
/// - Complete usage examples
///
/// # Errors
///
/// Returns miette diagnostic errors if:
/// - Terminal attributes cannot be retrieved or set
/// - Platform is not supported (Windows currently)
/// - Lock is poisoned (internal state corruption)
///
/// [module documentation]: mod@crate::core::ansi::terminal_raw_mode
pub fn enable_raw_mode() -> miette::Result<()> {
    #[cfg(unix)]
    {
        raw_mode_unix::enable_raw_mode()
    }

    #[cfg(windows)]
    {
        raw_mode_windows::enable_raw_mode()
    }

    #[cfg(not(any(unix, windows)))]
    {
        Err(miette!("Platform not supported"))
    }
}

/// Disable raw mode and restore original terminal settings.
///
/// Safe to call even if raw mode was never enabled (it will be a no-op).
/// Prefer using [`RawModeGuard`] for automatic cleanup.
///
/// See [module documentation] for complete documentation and examples.
///
/// # Errors
///
/// Returns miette diagnostic errors if:
/// - Terminal attributes cannot be set
/// - Platform is not supported (Windows currently)
/// - Lock is poisoned (internal state corruption)
///
/// [module documentation]: mod@crate::core::ansi::terminal_raw_mode
pub fn disable_raw_mode() -> miette::Result<()> {
    #[cfg(unix)]
    {
        raw_mode_unix::disable_raw_mode()
    }

    #[cfg(windows)]
    {
        raw_mode_windows::disable_raw_mode()
    }

    #[cfg(not(any(unix, windows)))]
    {
        Err(miette!("Platform not supported"))
    }
}

/// RAII guard that automatically disables raw mode when dropped.
///
/// Recommended way to use raw mode as it ensures terminal restoration even on panic.
/// See [module documentation] for usage examples and complete
/// documentation.
///
/// [module documentation]: mod@crate::core::ansi::terminal_raw_mode
#[derive(Debug)]
pub struct RawModeGuard;

impl RawModeGuard {
    /// Create a new guard and enable raw mode.
    ///
    /// # Errors
    ///
    /// Returns miette diagnostic errors if raw mode cannot be enabled.
    /// See [`enable_raw_mode()`] for error conditions.
    pub fn new() -> miette::Result<Self> {
        enable_raw_mode()?;
        Ok(RawModeGuard)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) { drop(disable_raw_mode()); }
}
