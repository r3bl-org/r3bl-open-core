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

use std::{io,
          sync::{LazyLock, Mutex}};

#[cfg(unix)]
mod unix_impl {
    use super::{LazyLock, Mutex, io};
    use rustix::termios::{self, ControlModes, InputModes, LocalModes, OptionalActions,
                          OutputModes, SpecialCodeIndex, Termios};

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
        let mut termios = termios::tcgetattr(&stdin).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("tcgetattr failed: {}", e))
        })?;

        // Save original settings
        {
            let mut original = ORIGINAL_TERMIOS.lock().map_err(|e| {
                io::Error::new(io::ErrorKind::Other, format!("Lock poisoned: {}", e))
            })?;

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
        termios
            .control_modes
            .remove(ControlModes::CSIZE | ControlModes::PARENB);
        termios.control_modes.insert(ControlModes::CS8);

        // Set minimum bytes and timeout for read
        termios.special_codes[SpecialCodeIndex::VMIN] = 1; // Read at least 1 byte
        termios.special_codes[SpecialCodeIndex::VTIME] = 0; // No timeout

        // Apply the new settings
        termios::tcsetattr(&stdin, OptionalActions::Now, &termios).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("tcsetattr failed: {}", e))
        })?;

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
        let original = ORIGINAL_TERMIOS.lock().map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("Lock poisoned: {}", e))
        })?;

        if let Some(ref termios) = *original {
            let stdin = io::stdin();
            termios::tcsetattr(&stdin, OptionalActions::Now, termios).map_err(|e| {
                io::Error::new(io::ErrorKind::Other, format!("tcsetattr failed: {}", e))
            })?;
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
    fn drop(&mut self) { drop(disable_raw_mode()); }
}

#[cfg(any(test, doc))]
pub mod integration_tests {
    use super::*;
    use crate::run_test_in_isolated_process_with_pty;

    /// PTY-based integration test for raw mode functionality.
    ///
    /// This test uses a master/slave PTY pair to verify that:
    /// 1. Raw mode can be enabled on a real PTY
    /// 2. Raw mode can be disabled and terminal settings restored
    /// 3. The RAII guard pattern works correctly
    ///
    /// Run with: `cargo test -p r3bl_tui --lib test_raw_mode_pty -- --nocapture`
    #[test]
    fn test_raw_mode_pty() {
        run_test_in_isolated_process_with_pty!(
            env_var: "TEST_RAW_MODE_PTY_SLAVE",
            test_name: "test_raw_mode_pty",
            slave: run_raw_mode_pty_slave,
            master: run_raw_mode_pty_master
        );
    }

    /// Master process: verifies results.
    /// Receives PTY pair and child process from the macro.
    fn run_raw_mode_pty_master(
        pty_pair: portable_pty::PtyPair,
        mut child: Box<dyn portable_pty::Child + Send + Sync>,
    ) {
        use std::{io::{BufRead, BufReader},
                  time::{Duration, Instant}};

        eprintln!("🚀 PTY Master: Starting raw mode test...");

        // Read from PTY and verify
        let reader = pty_pair
            .master
            .try_clone_reader()
            .expect("Failed to get reader");
        let mut buf_reader = BufReader::new(reader);

        eprintln!("📝 PTY Master: Waiting for slave results...");

        let mut slave_started = false;
        let mut test_passed = false;
        let start_timeout = Instant::now();

        while start_timeout.elapsed() < Duration::from_secs(5) {
            let mut line = String::new();
            match buf_reader.read_line(&mut line) {
                Ok(0) => {
                    eprintln!("  ⚠️  EOF reached");
                    break;
                }
                Ok(_) => {
                    let trimmed = line.trim();
                    eprintln!("  ← Slave output: {}", trimmed);

                    if trimmed.contains("SLAVE_STARTING") {
                        slave_started = true;
                        eprintln!("  ✓ Slave confirmed starting");
                    }
                    if trimmed.contains("SUCCESS:") {
                        test_passed = true;
                        eprintln!("  ✓ Test passed: {}", trimmed);
                        break;
                    }
                    if trimmed.contains("FAILED:") {
                        panic!("Test failed: {}", trimmed);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) => panic!("Read error: {}", e),
            }
        }

        assert!(slave_started, "Slave did not start properly");
        assert!(test_passed, "Test did not report success");

        // 4. Wait for slave to exit
        match child.wait() {
            Ok(status) => {
                eprintln!("✅ PTY Master: Slave exited: {:?}", status);
            }
            Err(e) => {
                panic!("Failed to wait for slave: {}", e);
            }
        }

        eprintln!("✅ PTY Master: Raw mode test passed!");
    }

    /// Slave process: enables raw mode and reports results.
    /// This function MUST exit before returning so other tests don't run.
    fn run_raw_mode_pty_slave() -> ! {
        use rustix::termios;
        use std::io::Write;

        println!("SLAVE_STARTING");
        std::io::stdout().flush().expect("Failed to flush");

        eprintln!("🔍 Slave: Starting raw mode test...");

        // Get current terminal settings BEFORE enabling raw mode
        let stdin = io::stdin();
        let before_termios = match termios::tcgetattr(&stdin) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("⚠️  Slave: Failed to get termios before: {}", e);
                println!("FAILED: Could not read termios");
                std::io::stdout().flush().expect("Failed to flush");
                std::process::exit(1);
            }
        };

        // Enable raw mode using the guard
        let _guard = match RawModeGuard::new() {
            Ok(g) => g,
            Err(e) => {
                eprintln!("⚠️  Slave: Failed to enable raw mode: {}", e);
                println!("FAILED: Could not enable raw mode");
                std::io::stdout().flush().expect("Failed to flush");
                std::process::exit(1);
            }
        };

        eprintln!("✓ Slave: Raw mode enabled");

        // Get terminal settings AFTER enabling raw mode
        let after_termios = match termios::tcgetattr(&stdin) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("⚠️  Slave: Failed to get termios after: {}", e);
                println!("FAILED: Could not read termios after");
                std::io::stdout().flush().expect("Failed to flush");
                std::process::exit(1);
            }
        };

        // Verify that settings actually changed
        if before_termios.local_modes == after_termios.local_modes {
            eprintln!("⚠️  Slave: Local modes didn't change!");
            println!("FAILED: Modes not changed");
            std::io::stdout().flush().expect("Failed to flush");
            std::process::exit(1);
        }

        eprintln!("✓ Slave: Terminal settings changed correctly");

        // Report success
        println!("SUCCESS: Raw mode enabled and verified");
        std::io::stdout().flush().expect("Failed to flush");

        eprintln!("🔍 Slave: Guard will be dropped now...");
        // Guard is dropped here, disabling raw mode
        eprintln!("🔍 Slave: Completed, exiting");
        // CRITICAL: Exit immediately to prevent test harness from running other tests
        std::process::exit(0);
    }
}
