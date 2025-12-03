// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words isatty winsize tcgetwinsize

use crate::{ColWidth, Size, height,
            tui::terminal_lib_backends::{TERMINAL_LIB_BACKEND, TerminalLibBackend},
            width};
use miette::IntoDiagnostic;
use std::io::IsTerminal;
pub const DEFAULT_WIDTH: u16 = 80;

#[must_use]
pub fn get_terminal_width_no_default() -> Option<ColWidth> {
    match get_size() {
        Ok(size) => Some(size.col_width),
        Err(_) => None,
    }
}

/// Get the terminal width. If there is a problem, return the default width.
#[must_use]
pub fn get_terminal_width() -> ColWidth {
    match get_size() {
        Ok(size) => size.col_width,
        Err(_) => width(DEFAULT_WIDTH),
    }
}

/// Get the terminal size.
///
/// Uses [`crossterm`] for the Crossterm backend, or [`rustix`] [`tcgetwinsize`] syscall
/// for the [`DirectToAnsi`] backend.
///
/// # Errors
///
/// Returns an error if:
/// - The terminal size cannot be determined
/// - The terminal is not available or not a TTY
///
/// [`DirectToAnsi`]: mod@crate::tui::terminal_lib_backends::direct_to_ansi
/// [`tcgetwinsize`]: fn@rustix::termios::tcgetwinsize
#[cfg(unix)]
pub fn get_size() -> miette::Result<Size> {
    match TERMINAL_LIB_BACKEND {
        TerminalLibBackend::Crossterm => {
            let (columns, rows) = crossterm::terminal::size().into_diagnostic()?;
            Ok(width(columns) + height(rows))
        }
        TerminalLibBackend::DirectToAnsi => {
            let winsize = rustix::termios::tcgetwinsize(std::io::stdout())
                .map_err(|e| miette::miette!("tcgetwinsize failed: {}", e))?;
            Ok(width(winsize.ws_col) + height(winsize.ws_row))
        }
    }
}

/// Get the terminal size.
///
/// # Errors
///
/// Returns an error if:
/// - The terminal size cannot be determined
/// - The terminal is not available or not a TTY
///
/// # TODO(windows)
///
/// This fallback only uses crossterm and ignores [`TERMINAL_LIB_BACKEND`]. The
/// [`DirectToAnsi`] backend uses Unix-specific APIs (`rustix::termios::tcgetwinsize`)
/// that aren't available on Windows.
///
/// [`DirectToAnsi`]: mod@crate::tui::terminal_lib_backends::direct_to_ansi
#[cfg(not(unix))]
pub fn get_size() -> miette::Result<Size> {
    let (columns, rows) = crossterm::terminal::size().into_diagnostic()?;
    Ok(width(columns) + height(rows))
}

#[derive(Debug)]
pub enum StdinIsPipedResult {
    StdinIsPiped,
    StdinIsNotPiped,
}

#[derive(Debug)]
pub enum StdoutIsPipedResult {
    StdoutIsPiped,
    StdoutIsNotPiped,
}

/// If you run `echo "test" | cargo run` the following will return true.
/// More info: <https://unix.stackexchange.com/questions/597083/how-does-piping-affect-stdin>
#[must_use]
pub fn is_stdin_piped() -> StdinIsPipedResult {
    let is_tty = match TERMINAL_LIB_BACKEND {
        TerminalLibBackend::Crossterm => std::io::stdin().is_terminal(),
        TerminalLibBackend::DirectToAnsi => rustix::termios::isatty(std::io::stdin()),
    };
    if is_tty {
        StdinIsPipedResult::StdinIsNotPiped
    } else {
        StdinIsPipedResult::StdinIsPiped
    }
}

/// If you run `cargo run | grep foo` the following will return true.
/// More info: <https://unix.stackexchange.com/questions/597083/how-does-piping-affect-stdin>
#[must_use]
pub fn is_stdout_piped() -> StdoutIsPipedResult {
    let is_tty = match TERMINAL_LIB_BACKEND {
        TerminalLibBackend::Crossterm => std::io::stdout().is_terminal(),
        TerminalLibBackend::DirectToAnsi => rustix::termios::isatty(std::io::stdout()),
    };
    if is_tty {
        StdoutIsPipedResult::StdoutIsNotPiped
    } else {
        StdoutIsPipedResult::StdoutIsPiped
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TTYResult {
    IsInteractive,
    IsNotInteractive,
}

/// Returns [`TTYResult::IsInteractive`] if stdin is an interactive terminal (TTY).
///
/// This is useful for checking if the program can receive interactive input from the
/// user. For example, a terminal multiplexer needs stdin to be a TTY to read keystrokes.
///
/// Note: This only checks stdin. Use [`is_headless`] to check if *all* streams (stdin,
/// stdout, stderr) are non-interactive, or [`is_output_interactive`] to check if output
/// streams are interactive.
#[must_use]
pub fn is_stdin_interactive() -> TTYResult {
    let is_tty = match TERMINAL_LIB_BACKEND {
        TerminalLibBackend::Crossterm => std::io::stdin().is_terminal(),
        TerminalLibBackend::DirectToAnsi => rustix::termios::isatty(std::io::stdin()),
    };
    if is_tty {
        TTYResult::IsInteractive
    } else {
        TTYResult::IsNotInteractive
    }
}

/// Returns [`TTYResult::IsNotInteractive`] if stdin, stdout, and stderr are *all*
/// non-interactive (not TTYs). This typically happens when running under `cargo test`
/// or in other headless/batch environments.
///
/// Use this to detect fully non-interactive environments where no terminal I/O is
/// possible.
#[must_use]
pub fn is_headless() -> TTYResult {
    // TODO(windows): Workaround for cargo redirecting streams on Windows.
    // When running through `cargo run` on Windows, the terminal detection may incorrectly
    // report all streams as non-terminal even when running in an interactive terminal.
    // This is because cargo may redirect the streams. To work around this, we check if
    // we're running under cargo and if so, assume it's interactive.
    #[cfg(target_os = "windows")]
    if std::env::var("CARGO").is_ok() || std::env::var("CARGO_PKG_NAME").is_ok() {
        return TTYResult::IsInteractive;
    }

    let (stdin_is_tty, stdout_is_tty, stderr_is_tty) = match TERMINAL_LIB_BACKEND {
        TerminalLibBackend::Crossterm => (
            std::io::stdin().is_terminal(),
            std::io::stdout().is_terminal(),
            std::io::stderr().is_terminal(),
        ),
        TerminalLibBackend::DirectToAnsi => (
            rustix::termios::isatty(std::io::stdin()),
            rustix::termios::isatty(std::io::stdout()),
            rustix::termios::isatty(std::io::stderr()),
        ),
    };
    if !stdin_is_tty && !stdout_is_tty && !stderr_is_tty {
        TTYResult::IsNotInteractive
    } else {
        TTYResult::IsInteractive
    }
}

/// Returns [`TTYResult::IsInteractive`] if both stdout and stderr are interactive TTYs.
///
/// This is useful for checking if the program can display output to an interactive
/// terminal. Returns [`TTYResult::IsNotInteractive`] if *either* stdout or stderr is
/// redirected or piped.
///
/// Example scenario where this returns `IsNotInteractive`:
/// ```bash
/// command >file 2>&1
/// ```
/// Here stdin may still be a TTY, but output streams are redirected to a file.
#[must_use]
pub fn is_output_interactive() -> TTYResult {
    let (stdout_is_tty, stderr_is_tty) = match TERMINAL_LIB_BACKEND {
        TerminalLibBackend::Crossterm => (
            std::io::stdout().is_terminal(),
            std::io::stderr().is_terminal(),
        ),
        TerminalLibBackend::DirectToAnsi => (
            rustix::termios::isatty(std::io::stdout()),
            rustix::termios::isatty(std::io::stderr()),
        ),
    };

    // If either stdout or stderr is not a TTY, consider output non-interactive
    if !stdout_is_tty || !stderr_is_tty {
        TTYResult::IsNotInteractive
    } else {
        TTYResult::IsInteractive
    }
}
