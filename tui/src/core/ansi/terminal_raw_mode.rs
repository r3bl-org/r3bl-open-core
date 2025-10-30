// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Terminal raw mode implementation for ANSI terminals.
//!
//! This module provides functionality to enable and disable raw mode on terminals,
//! which is essential for reading ANSI escape sequences character-by-character
//! without line buffering or terminal interpretation.
//!
//! ## Raw Mode vs Cooked Mode
//!
//! **Cooked Mode** (default):
//! - Input is line-buffered (waits for Enter key)
//! - Special characters are interpreted (Ctrl+C, Ctrl+D, etc.)
//! - ANSI escape sequences may be processed by the terminal
//! - Echoing is enabled (typed characters appear on screen)
//!
//! **Raw Mode**:
//! - No line buffering - bytes available immediately
//! - No special character processing - all bytes pass through
//! - No echo - typed characters don't automatically appear
//! - Perfect for reading ANSI escape sequences and building TUIs
//!
//! ## Platform Support
//!
//! - **Unix/Linux/macOS**: Uses rustix's safe termios API
//! - **Windows**: Not yet implemented (TODO)

use std::io;
use std::sync::{LazyLock, Mutex};

#[cfg(unix)]
mod unix_impl {
    use super::{io, LazyLock, Mutex};
    use rustix::termios::{
        self, ControlModes, InputModes, LocalModes, OptionalActions, OutputModes,
        SpecialCodeIndex, Termios,
    };

    /// Stores the original terminal settings to restore later.
    /// Using std::sync::LazyLock (stabilized in Rust 1.80) instead of once_cell.
    static ORIGINAL_TERMIOS: LazyLock<Mutex<Option<Termios>>> =
        LazyLock::new(|| Mutex::new(None));

    /// Enable raw mode on the terminal (Unix implementation).
    ///
    /// This function:
    /// 1. Saves the current terminal settings
    /// 2. Modifies settings to disable:
    ///    - ICANON (canonical mode) - no line buffering
    ///    - ECHO - no automatic echoing
    ///    - ISIG - no signal generation (Ctrl+C, etc.)
    ///    - IXON - no software flow control (Ctrl+S/Q)
    ///    - IEXTEN - no extended processing
    ///    - ICRNL - no CR to NL translation
    ///    - OPOST - no output processing
    /// 3. Sets VMIN=1, VTIME=0 for immediate byte-by-byte reading
    ///
    /// The original settings are saved so they can be restored with `disable_raw_mode()`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Terminal attributes cannot be retrieved
    /// - Terminal attributes cannot be set
    /// - Lock is poisoned
    pub fn enable_raw_mode() -> io::Result<()> {
        let stdin = io::stdin();
        let mut termios = termios::tcgetattr(&stdin)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("tcgetattr failed: {}", e)))?;

        // Save original settings
        {
            let mut original = ORIGINAL_TERMIOS
                .lock()
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Lock poisoned: {}", e)))?;

            if original.is_none() {
                // rustix's Termios doesn't implement Copy, so we need to clone
                *original = Some(termios.clone());
            }
        }

        // Modify settings for raw mode using rustix's type-safe API
        // Based on cfmakeraw() implementation
        termios.input_modes.remove(
            InputModes::IGNBRK
                | InputModes::BRKINT
                | InputModes::PARMRK
                | InputModes::ISTRIP
                | InputModes::INLCR
                | InputModes::IGNCR
                | InputModes::ICRNL
                | InputModes::IXON,
        );
        termios.output_modes.remove(OutputModes::OPOST);
        termios.local_modes.remove(
            LocalModes::ECHO
                | LocalModes::ECHONL
                | LocalModes::ICANON
                | LocalModes::ISIG
                | LocalModes::IEXTEN,
        );
        termios.control_modes.remove(ControlModes::CSIZE | ControlModes::PARENB);
        termios.control_modes.insert(ControlModes::CS8);

        // Set minimum bytes and timeout for read
        termios.special_codes[SpecialCodeIndex::VMIN] = 1;  // Read at least 1 byte
        termios.special_codes[SpecialCodeIndex::VTIME] = 0; // No timeout

        // Apply the new settings
        termios::tcsetattr(&stdin, OptionalActions::Now, &termios)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("tcsetattr failed: {}", e)))?;

        Ok(())
    }

    /// Disable raw mode and restore original terminal settings (Unix implementation).
    ///
    /// This restores the terminal settings that were saved when `enable_raw_mode()`
    /// was first called. If raw mode was never enabled, this is a no-op.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Terminal attributes cannot be set
    /// - Lock is poisoned
    pub fn disable_raw_mode() -> io::Result<()> {
        let original = ORIGINAL_TERMIOS
            .lock()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Lock poisoned: {}", e)))?;

        if let Some(ref termios) = *original {
            let stdin = io::stdin();
            termios::tcsetattr(&stdin, OptionalActions::Now, termios)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("tcsetattr failed: {}", e)))?;
        }
        Ok(())
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::io;

    /// Enable raw mode on Windows (TODO: implement using Windows Console API).
    ///
    /// # Errors
    ///
    /// Currently returns an error as Windows support is not yet implemented.
    pub fn enable_raw_mode() -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Windows raw mode not yet implemented",
        ))
    }

    /// Disable raw mode on Windows (TODO: implement using Windows Console API).
    ///
    /// # Errors
    ///
    /// Currently returns an error as Windows support is not yet implemented.
    pub fn disable_raw_mode() -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Windows raw mode not yet implemented",
        ))
    }
}

/// Enable raw mode on the terminal.
///
/// Raw mode is essential for:
/// - Reading ANSI escape sequences character-by-character
/// - Building terminal user interfaces (TUIs)
/// - Processing keyboard input without buffering
///
/// Remember to call [`disable_raw_mode()`] before your program exits to restore
/// the terminal to its original state.
///
/// # Example
///
/// ```no_run
/// use r3bl_tui::core::ansi::terminal_raw_mode;
///
/// // Enable raw mode
/// terminal_raw_mode::enable_raw_mode().expect("Failed to enable raw mode");
///
/// // ... read and process ANSI sequences ...
///
/// // Always restore before exit
/// terminal_raw_mode::disable_raw_mode().expect("Failed to disable raw mode");
/// ```
///
/// # Platform Notes
///
/// - **Unix/Linux/macOS**: Uses rustix's safe termios API
/// - **Windows**: TODO - will use Windows Console API
///
/// # Errors
///
/// Returns an error if:
/// - Terminal attributes cannot be retrieved or set
/// - Platform is not supported (Windows currently)
/// - Lock is poisoned (internal state corruption)
pub fn enable_raw_mode() -> io::Result<()> {
    #[cfg(unix)]
    return unix_impl::enable_raw_mode();

    #[cfg(windows)]
    return windows_impl::enable_raw_mode();

    #[cfg(not(any(unix, windows)))]
    return Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Platform not supported",
    ));
}

/// Disable raw mode and restore original terminal settings.
///
/// This should always be called before your program exits if you enabled raw mode.
/// It's safe to call even if raw mode was never enabled (it will be a no-op).
///
/// # Example
///
/// ```no_run
/// use r3bl_tui::core::ansi::terminal_raw_mode;
///
/// // Use a drop guard to ensure cleanup
/// struct RawModeGuard;
/// impl Drop for RawModeGuard {
///     fn drop(&mut self) {
///         let _ = terminal_raw_mode::disable_raw_mode();
///     }
/// }
///
/// let _guard = RawModeGuard;
/// terminal_raw_mode::enable_raw_mode().expect("Failed to enable raw mode");
/// // ... raw mode operations ...
/// // Guard automatically disables raw mode when dropped
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - Terminal attributes cannot be set
/// - Platform is not supported (Windows currently)
/// - Lock is poisoned (internal state corruption)
pub fn disable_raw_mode() -> io::Result<()> {
    #[cfg(unix)]
    return unix_impl::disable_raw_mode();

    #[cfg(windows)]
    return windows_impl::disable_raw_mode();

    #[cfg(not(any(unix, windows)))]
    return Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Platform not supported",
    ));
}

/// RAII guard that automatically disables raw mode when dropped.
///
/// This is the recommended way to use raw mode as it ensures the terminal
/// is restored even if your program panics.
///
/// # Example
///
/// ```no_run
/// use r3bl_tui::core::ansi::terminal_raw_mode::RawModeGuard;
///
/// {
///     let _guard = RawModeGuard::new().expect("Failed to enable raw mode");
///     // Terminal is now in raw mode
///     // ... do work ...
/// } // Raw mode automatically disabled when guard is dropped
/// ```
#[derive(Debug)]
pub struct RawModeGuard;

impl RawModeGuard {
    /// Create a new guard and enable raw mode.
    ///
    /// # Errors
    ///
    /// Returns an error if raw mode cannot be enabled.
    /// See [`enable_raw_mode()`] for specific error conditions.
    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        Ok(RawModeGuard)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_mode_guard() {
        // We can't really test raw mode in unit tests since they don't have a real terminal
        // But we can at least test that the guard compiles and drops correctly
        {
            // This would fail in CI/non-terminal environment, so we don't actually run it
            // Just verify it compiles
            if false {
                let _guard = RawModeGuard::new().expect("Failed to create guard");
            }
        }
        // If we had enabled raw mode, it would be disabled here
    }
}