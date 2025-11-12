// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Unix/Linux/macOS implementation of raw mode using rustix's safe termios API.

use miette::miette;
use rustix::{fd::{AsFd, BorrowedFd},
             termios::{self, OptionalActions, Termios}};
use std::{fs::File,
          io,
          sync::{LazyLock, Mutex}};

/// Stores the original terminal settings to restore later.
/// Using [`std::sync::LazyLock`] (stabilized in Rust 1.80) instead of `once_cell`.
static ORIGINAL_TERMIOS: LazyLock<Mutex<Option<Termios>>> =
    LazyLock::new(|| Mutex::new(None));

/// Represents either stdin or `/dev/tty` for terminal operations.
///
/// This enum allows us to handle both cases where stdin is a tty (normal terminal usage)
/// and where stdin is redirected (e.g., piped input), requiring us to use `/dev/tty`.
enum TerminalFd {
    /// Using standard input (when it's a terminal)
    Stdin(io::Stdin),
    /// Using `/dev/tty` (when stdin is redirected)
    DevTty(File),
}

impl AsFd for TerminalFd {
    fn as_fd(&self) -> BorrowedFd<'_> {
        match self {
            TerminalFd::Stdin(stdin) => stdin.as_fd(),
            TerminalFd::DevTty(file) => file.as_fd(),
        }
    }
}

/// Gets a file descriptor for the controlling terminal.
///
/// Follows crossterm's approach: checks if stdin is a tty and uses it if so;
/// otherwise opens `/dev/tty`. This handles cases where stdin is redirected.
///
/// # Errors
///
/// Returns an error if stdin is not a tty and `/dev/tty` cannot be opened.
fn get_terminal_fd() -> io::Result<TerminalFd> {
    let stdin = io::stdin();
    if termios::isatty(&stdin) {
        Ok(TerminalFd::Stdin(stdin))
    } else {
        let file = File::options().read(true).write(true).open("/dev/tty")?;
        Ok(TerminalFd::DevTty(file))
    }
}

/// Enable raw mode on the terminal (Unix/Linux/macOS implementation).
///
/// Uses rustix's type-safe termios API to:
/// 1. Get the controlling terminal (stdin if it's a tty, otherwise `/dev/tty`)
/// 2. Save the original terminal settings for restoration
/// 3. Disable canonical mode, echo, and signal generation
/// 4. Set VMIN=1, VTIME=0 for immediate byte-by-byte reading
///
/// Follows crossterm's approach: checks if stdin is a tty and uses it if so;
/// otherwise opens `/dev/tty`. This handles cases where stdin is redirected
/// (e.g., `echo "data" | your_app`).
///
/// See [module documentation] for conceptual overview and usage.
///
/// # Errors
///
/// Returns miette diagnostic errors if:
/// - Terminal file descriptor cannot be obtained
/// - Terminal attributes cannot be retrieved or set
/// - Mutex lock is poisoned
///
/// [module documentation]: mod@crate::core::ansi::terminal_raw_mode
pub fn enable_raw_mode() -> miette::Result<()> {
    let fd = get_terminal_fd()
        .map_err(|e| miette::miette!("failed to get terminal file descriptor: {e}"))?;

    let mut termios = termios::tcgetattr(&fd)
        .map_err(|e| miette::miette!("failed to retrieve terminal attributes: {e}"))?;

    // Save original settings
    {
        let mut original = ORIGINAL_TERMIOS
            .lock()
            .map_err(|e| miette!("terminal settings lock poisoned: {e}"))?;

        if original.is_none() {
            // rustix's Termios doesn't implement Copy, so we need to clone
            *original = Some(termios.clone());
        }
    }

    // Use rustix's built-in make_raw() method which correctly implements cfmakeraw
    // behavior. This is the same approach crossterm uses (see
    // crossterm-0.29.0/src/terminal/sys/unix.rs:135). make_raw() handles all the
    // necessary terminal attribute changes including:
    // - Disabling canonical mode (ICANON)
    // - Disabling signal generation (ISIG)
    // - Disabling echo (ECHO, ECHONL)
    // - Setting special character processing (VMIN=1, VTIME=0)
    // - Properly handling VEOF and other special characters
    termios.make_raw();

    // Apply the new settings
    termios::tcsetattr(&fd, OptionalActions::Now, &termios)
        .map_err(|e| miette::miette!("failed to set terminal attributes: {e}"))?;

    Ok(())
}

/// Disable raw mode and restore original terminal settings (Unix/Linux/macOS
/// implementation).
///
/// Restores the terminal settings saved by `enable_raw_mode()`. Uses the same
/// terminal file descriptor selection logic as `enable_raw_mode()` (stdin if it's
/// a tty, otherwise `/dev/tty`). No-op if raw mode was never enabled.
///
/// # Errors
///
/// Returns miette diagnostic errors if:
/// - Terminal file descriptor cannot be obtained
/// - Terminal attributes cannot be set
/// - Mutex lock is poisoned
pub fn disable_raw_mode() -> miette::Result<()> {
    let original = ORIGINAL_TERMIOS
        .lock()
        .map_err(|e| miette!("terminal settings lock poisoned: {e}"))?;

    if let Some(ref termios) = *original {
        let fd = get_terminal_fd().map_err(|e| {
            miette::miette!("failed to get terminal file descriptor: {e}")
        })?;

        termios::tcsetattr(&fd, OptionalActions::Now, termios)
            .map_err(|e| miette::miette!("failed to set terminal attributes: {e}"))?;
    }
    Ok(())
}
