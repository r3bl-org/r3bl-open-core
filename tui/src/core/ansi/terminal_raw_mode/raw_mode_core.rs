// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Core raw mode functionality and RAII guard.
//!
//! This module provides:
//! - Platform-agnostic public API functions that dispatch to platform-specific
//!   implementations
//! - The `RawModeGuard` RAII wrapper for automatic resource cleanup
//!
//! # Architecture Context
//!
//! This module is part of a 3-layer raw mode architecture:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │  terminal_lib_backends/raw_mode.rs (High-level)             │
//! │  └─ RawMode struct for render pipeline integration          │
//! ├─────────────────────────────────────────────────────────────┤
//! │  terminal_raw_mode/ (This module - Mid-level)       ◄───────│
//! │  └─ try_enable_raw_mode(), try_disable_raw_mode(),          │
//! │     RawModeGuard                                            │
//! ├─────────────────────────────────────────────────────────────┤
//! │  constants/raw_mode.rs (Low-level)                          │
//! │  └─ VMIN_RAW_MODE, VTIME_RAW_MODE                           │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! **You are here**: The core implementation layer, providing the public API
//! for direct raw mode control and the [`RawModeGuard`] RAII wrapper.
//!
//! **See also**:
//! - [`VMIN_RAW_MODE`][vmin] / [`VTIME_RAW_MODE`][vtime] - POSIX [`termios`] constants
//! - [`TerminalModeController::enter_raw_mode`]: High-level terminal mode control
//!
//! [`TerminalModeController::enter_raw_mode`]:
//!     crate::TerminalModeController::enter_raw_mode
//! [`termios`]: https://en.wikipedia.org/wiki/Termios
//! [vmin]: crate::VMIN_RAW_MODE
//! [vtime]: crate::VTIME_RAW_MODE

// Import platform-specific implementations for DirectToAnsi backend.
#[cfg(unix)]
use super::raw_mode_unix;
#[cfg(windows)]
use super::raw_mode_windows;
use crate::{DEBUG_TERMINAL_BACKEND, TERMINAL_LIB_BACKEND, TerminalLibBackend};

/// Custom error type for terminal raw mode operations across Unix, Windows, and
/// [`Crossterm`] backends.
///
/// [`Crossterm`]: crate::tui::TerminalLibBackend::Crossterm
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[non_exhaustive]
pub enum RawModeError {
    /// Failed to obtain a terminal file descriptor ([`stdin`] or `/dev/tty`).
    ///
    /// [`stdin`]: std::io::stdin
    #[error("Failed to get terminal file descriptor: {0}")]
    #[diagnostic(
        code(r3bl_tui::terminal_raw_mode::get_fd_failed),
        help(
            "Ensure the application is running in an interactive terminal environment."
        )
    )]
    GetFileDescriptorFailed(#[source] std::io::Error),

    /// Failed to query or apply POSIX [`termios`] attributes.
    ///
    /// [`termios`]: https://en.wikipedia.org/wiki/Termios
    #[cfg(unix)]
    #[error("Failed to get/set terminal attributes: {0}")]
    #[diagnostic(
        code(r3bl_tui::terminal_raw_mode::attributes_failed),
        help(
            "The terminal device might not support standard termios attribute retrieval, or it might have been closed."
        )
    )]
    AttributesFailed(#[source] rustix::io::Errno),

    /// Crossterm or Windows console mode operation failed.
    #[error("Terminal raw mode operation failed: {0}")]
    #[diagnostic(code(r3bl_tui::terminal_raw_mode::io_failed))]
    BackendIo(#[source] std::io::Error),

    /// Current target platform is not supported for `DirectToAnsi` raw mode.
    #[error("Platform is not supported for DirectToAnsi raw mode")]
    UnsupportedPlatform,
}

/// Enables raw mode on the terminal.
///
/// Dispatches to the correct raw mode implementation based on [`TERMINAL_LIB_BACKEND`]:
/// - **Linux** ([`DirectToAnsi`]): Uses rustix-based [`termios`] API
/// - **macOS/Windows** ([`Crossterm`]): Uses [`crossterm::terminal::enable_raw_mode()`]
///
/// See [module documentation] for:
/// - Why raw mode is needed and how it differs from cooked mode
/// - Platform-specific implementation details
/// - Complete usage examples
///
/// # Errors
///
/// Returns [`RawModeError`] if:
/// - Terminal attributes or console mode cannot be retrieved or set
/// - Platform is not supported (e.g., WASM, embedded targets without OS)
///
/// [`Crossterm`]: crate::tui::TerminalLibBackend::Crossterm
/// [`DirectToAnsi`]: crate::tui::TerminalLibBackend::DirectToAnsi
/// [`TERMINAL_LIB_BACKEND`]: crate::tui::TERMINAL_LIB_BACKEND
/// [`termios`]: https://en.wikipedia.org/wiki/Termios
/// [module documentation]: mod@crate::core::ansi::terminal_raw_mode
pub fn try_enable_raw_mode() -> Result<(), RawModeError> {
    DEBUG_TERMINAL_BACKEND.then(|| {
        tracing::debug!(
            message = "try_enable_raw_mode: 🟢 enabling raw mode",
            backend = ?TERMINAL_LIB_BACKEND
        );
    });
    let result = match TERMINAL_LIB_BACKEND {
        TerminalLibBackend::DirectToAnsi => {
            #[cfg(unix)]
            {
                raw_mode_unix::try_enable_raw_mode()
            }

            #[cfg(windows)]
            {
                raw_mode_windows::try_enable_raw_mode()
            }

            #[cfg(not(any(unix, windows)))]
            {
                Err(RawModeError::UnsupportedPlatform)
            }
        }
        TerminalLibBackend::Crossterm => {
            crossterm::terminal::enable_raw_mode().map_err(RawModeError::BackendIo)
        }
    };

    match &result {
        Ok(()) => DEBUG_TERMINAL_BACKEND.then(|| {
            tracing::debug!(message = "try_enable_raw_mode: ✅ success");
        }),
        Err(e) => DEBUG_TERMINAL_BACKEND.then(|| {
            tracing::error!(message = "try_enable_raw_mode: ❌ failed", error = %e);
        }),
    };

    result
}

/// Disable raw mode and restore original terminal settings.
///
/// Dispatches to the correct raw mode implementation based on [`TERMINAL_LIB_BACKEND`]:
/// - **Linux** ([`DirectToAnsi`]): Uses rustix-based [`termios`] API
/// - **macOS/Windows** ([`Crossterm`]): Uses [`crossterm::terminal::disable_raw_mode()`]
///
/// Safe to call even if raw mode was never enabled (it will be a no-op).
/// Prefer using [`RawModeGuard`] for automatic cleanup.
///
/// See [module documentation] for complete documentation and examples.
///
/// # Errors
///
/// Returns [`RawModeError`] if:
/// - Terminal attributes or console mode cannot be restored
/// - Platform is not supported (e.g., WASM, embedded targets without OS)
///
/// [`Crossterm`]: crate::tui::TerminalLibBackend::Crossterm
/// [`DirectToAnsi`]: crate::tui::TerminalLibBackend::DirectToAnsi
/// [`TERMINAL_LIB_BACKEND`]: crate::tui::TERMINAL_LIB_BACKEND
/// [`termios`]: https://en.wikipedia.org/wiki/Termios
/// [module documentation]: mod@crate::core::ansi::terminal_raw_mode
pub fn try_disable_raw_mode() -> Result<(), RawModeError> {
    DEBUG_TERMINAL_BACKEND.then(|| {
        tracing::debug!(
            message = "try_disable_raw_mode: 🔴 disabling raw mode",
            backend = ?TERMINAL_LIB_BACKEND
        );
    });

    let result = match TERMINAL_LIB_BACKEND {
        TerminalLibBackend::DirectToAnsi => {
            #[cfg(unix)]
            {
                raw_mode_unix::try_disable_raw_mode()
            }

            #[cfg(windows)]
            {
                raw_mode_windows::try_disable_raw_mode()
            }

            #[cfg(not(any(unix, windows)))]
            {
                Err(RawModeError::UnsupportedPlatform)
            }
        }
        TerminalLibBackend::Crossterm => {
            crossterm::terminal::disable_raw_mode().map_err(RawModeError::BackendIo)
        }
    };

    match &result {
        Ok(()) => DEBUG_TERMINAL_BACKEND.then(|| {
            tracing::debug!(message = "try_disable_raw_mode: ✅ success");
        }),
        Err(e) => DEBUG_TERMINAL_BACKEND.then(|| {
            tracing::error!(message = "try_disable_raw_mode: ❌ failed", error = %e);
        }),
    };

    result
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
    /// Creates a new guard and enables raw mode.
    ///
    /// # Errors
    ///
    /// Returns [`RawModeError`] if raw mode cannot be enabled.
    /// See [`try_enable_raw_mode()`] for error conditions.
    pub fn try_new() -> Result<Self, RawModeError> {
        try_enable_raw_mode()?;
        Ok(RawModeGuard)
    }
}

impl Drop for RawModeGuard {
    /// # Poison Safety
    ///
    /// This implementation is **poison-safe**. It delegates to [`try_disable_raw_mode()`],
    /// which is poison-safe and ensures that the terminal is restored even if the
    /// internal state is corrupted. We prioritize **Resilience over Integrity** here
    /// to prevent a **Double Panic Abort** that would **brick the user's terminal**.
    ///
    /// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section
    /// in the crate root documentation for details.
    ///
    /// [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety]:
    ///     crate#terminal-restoration-panic-drop-and-mutex-poison-safety
    fn drop(&mut self) { drop(try_disable_raw_mode()); }
}
