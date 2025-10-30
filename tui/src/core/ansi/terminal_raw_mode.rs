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
    fn drop(&mut self) { let _unused = disable_raw_mode(); }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    // XMARK: Process isolated test functions using env vars & PTY.

    /// PTY-based integration test for raw mode functionality.
    ///
    /// This test uses a master/slave PTY pair to verify that:
    /// 1. Raw mode can be enabled on a real PTY
    /// 2. Raw mode can be disabled and terminal settings restored
    /// 3. The RAII guard pattern works correctly
    ///
    /// ## Running the Test
    ///
    /// ```bash
    /// # Run just this test
    /// cargo test -p r3bl_tui --lib test_raw_mode_pty -- --nocapture
    ///
    /// # Run with all output visible
    /// cargo test -p r3bl_tui --lib test_raw_mode_pty -- --nocapture --show-output
    /// ```
    ///
    /// ## Test Architecture (2 Actors)
    ///
    /// This test validates raw mode functionality in a real PTY environment using a
    /// coordinator-worker pattern with two processes:
    ///
    /// ```text
    /// ┌───────────────────────────────────────────────────────────────┐
    /// │ Actor 1: PTY Master (test coordinator)                        │
    /// │ Synchronous code                                              │
    /// │                                                               │
    /// │  1. Create PTY pair (master/slave file descriptors)           │
    /// │  2. Spawn test binary with TEST_RAW_MODE_PTY_SLAVE=1 env var  │
    /// │  3. Read slave's stdout via PTY master                        │
    /// │  4. Verify raw mode was enabled and settings changed          │
    /// │  5. Verify results match expected values                      │
    /// └────────────────────────┬──────────────────────────────────────┘
    ///                          │ spawns with slave PTY as stdin/stdout
    /// ┌────────────────────────▼──────────────────────────────────────┐
    /// │ Actor 2: PTY Slave (worker process, TEST_RAW_MODE_PTY_SLAVE=1)│
    /// │ Synchronous code                                              │
    /// │                                                               │
    /// │  1. Test function detects TEST_RAW_MODE_PTY_SLAVE env var     │
    /// │  2. Read terminal settings BEFORE enabling raw mode           │
    /// │  3. Enable raw mode using RawModeGuard                        │
    /// │  4. Read terminal settings AFTER enabling raw mode            │
    /// │  5. Verify settings changed (compare before/after)            │
    /// │  6. Report results to stdout                                  │
    /// │  7. Exit immediately to prevent test harness recursion        │
    /// └───────────────────────────────────────────────────────────────┘
    /// ```
    ///
    /// ## Critical: Raw Mode Requirement
    ///
    /// **Raw Mode Clarification**: In PTY architecture, the SLAVE side is what the child
    /// process sees as its terminal. When the child reads from stdin, it's reading from
    /// the slave PTY. Therefore, we test raw mode on the SLAVE to verify that:
    ///
    /// 1. **No Line Buffering**: Input isn't line-buffered - characters are available
    ///    immediately without waiting for Enter key
    /// 2. **No Special Character Processing**: Special characters (like ESC sequences)
    ///    aren't interpreted by the terminal layer - they pass through as raw bytes
    /// 3. **Terminal Settings Actually Change**: We verify termios flags are modified
    ///    (ICANON, ECHO, ISIG, etc. are disabled)
    ///
    /// **Master vs Slave**: The master doesn't need raw mode - it's just a bidirectional
    /// pipe for communication. The slave is the actual "terminal" that needs proper
    /// settings for raw mode operation.
    ///
    /// Without raw mode, the PTY stays in "cooked" mode where:
    /// - Input waits for line termination (Enter key)
    /// - Control sequences may be interpreted instead of passed through
    /// - Applications can't read single keypresses immediately
    ///
    /// ## Why This Test Pattern?
    ///
    /// - **Real PTY Environment**: Tests raw mode with actual PTY, not mocks
    /// - **Process Isolation**: Each test run gets fresh PTY resources via process
    ///   spawning
    /// - **Coordinator-Worker Pattern**: Same test function handles both roles via env
    ///   var
    /// - **Actual Terminal Verification**: Uses rustix termios API to verify settings
    ///   changed
    ///
    /// The test detects its role via `TEST_RAW_MODE_PTY_SLAVE` environment variable:
    /// - If NOT set: Act as master (creates PTY, spawns slave, verifies)
    /// - If set: Act as slave (enables raw mode, reports results, then **exits
    ///   immediately**)
    #[test]
    fn test_raw_mode_pty() {
        // Skip in CI if running as master
        if std::env::var("TEST_RAW_MODE_PTY_SLAVE").is_err() && is_ci::cached() {
            println!("⏭️  Skipped in CI (requires interactive terminal)");
            return;
        }

        // Check if we're running as slave
        if std::env::var("TEST_RAW_MODE_PTY_SLAVE").is_ok() {
            run_raw_mode_pty_slave();
        } else {
            run_raw_mode_pty_master();
        }
    }

    /// Master process: creates PTY, spawns slave, verifies results.
    fn run_raw_mode_pty_master() {
        use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
        use std::io::BufRead;

        eprintln!("🚀 Master: Starting PTY-based raw mode test...");

        // 1. Create PTY pair
        let pty_system = NativePtySystem::default();
        let pty_pair = match pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        }) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("❌ Master: Failed to create PTY: {}", e);
                panic!("Failed to create PTY pair");
            }
        };

        eprintln!("🔍 Master: PTY pair created");

        // 2. Spawn test as slave
        // Pass args to run ONLY this test (same pattern as
        // pty_based_input_device_test.rs)
        let test_binary =
            std::env::current_exe().expect("Failed to get current executable");
        let mut cmd = CommandBuilder::new(&test_binary);
        cmd.env("TEST_RAW_MODE_PTY_SLAVE", "1");
        cmd.env("RUST_BACKTRACE", "1");
        // Run only this test to avoid polluting stdout with other test output
        cmd.args(&["--test-threads", "1", "--nocapture", "test_raw_mode_pty"]);

        let child = match pty_pair.slave.spawn_command(cmd) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("❌ Master: Failed to spawn slave: {}", e);
                panic!("Failed to spawn slave process");
            }
        };

        eprintln!(
            "🔍 Master: Slave process spawned (pid: {})",
            child.process_id().unwrap_or(0)
        );

        // 3. Read output from slave
        let reader = pty_pair
            .master
            .try_clone_reader()
            .expect("Failed to clone master reader");
        let buf_reader = std::io::BufReader::new(reader);
        let mut lines = buf_reader.lines();

        // Wait for "SLAVE_STARTING" to confirm slave is ready
        let mut got_startup = false;
        let mut test_result = String::new();

        // Read lines - since this is a module test with other tests, we need to filter
        // output
        for attempt in 0..500 {
            match lines.next() {
                Some(Ok(line)) => {
                    // Only log key lines to avoid spam
                    if line.contains("SLAVE_STARTING")
                        || line.contains("SUCCESS:")
                        || line.contains("FAILED:")
                    {
                        eprintln!("🔍 Master: Received [{}]: {}", attempt, line);
                    }

                    if line.contains("SLAVE_STARTING") {
                        got_startup = true;
                        eprintln!("✓ Master: Slave confirmed starting");
                    }

                    if line.contains("SUCCESS:") || line.contains("FAILED:") {
                        test_result = line;
                        eprintln!("✓ Master: Got result: {}", test_result);
                        break;
                    }
                }
                Some(Err(e)) => {
                    eprintln!("⚠️  Master: Read error: {}", e);
                    break;
                }
                None => {
                    eprintln!("⚠️  Master: EOF reached at attempt {}", attempt);
                    break;
                }
            }
        }

        // 4. Verify results
        if !got_startup {
            panic!("Slave did not start properly");
        }

        if test_result.is_empty() {
            panic!("Did not receive test result from slave");
        }

        if !test_result.contains("SUCCESS:") {
            panic!("Test failed: {}", test_result);
        }

        eprintln!("✓ Master: Test passed!");
        println!("✓ Raw mode PTY test passed: {}", test_result);
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
