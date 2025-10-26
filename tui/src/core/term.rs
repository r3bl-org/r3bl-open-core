// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{ColWidth, Size, height, width};
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
/// # Errors
///
/// Returns an error if:
/// - The terminal size cannot be determined
/// - The terminal is not available or not a TTY
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
    if std::io::stdin().is_terminal() {
        StdinIsPipedResult::StdinIsNotPiped
    } else {
        StdinIsPipedResult::StdinIsPiped
    }
}

/// If you run `cargo run | grep foo` the following will return true.
/// More info: <https://unix.stackexchange.com/questions/597083/how-does-piping-affect-stdin>
#[must_use]
pub fn is_stdout_piped() -> StdoutIsPipedResult {
    if std::io::stdout().is_terminal() {
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

/// Returns [`TTYResult::IsInteractive`] if stdin, stdout, and stderr are *all* fully
/// interactive.
///
/// There are situations where some can be interactive and others not, such as when piping
/// is active.
#[must_use]
pub fn is_fully_interactive_terminal() -> TTYResult {
    let is_tty: bool = std::io::stdin().is_terminal();
    if is_tty {
        TTYResult::IsInteractive
    } else {
        TTYResult::IsNotInteractive
    }
}

/// Returns [`TTYResult::IsNotInteractive`] if stdin, stdout, and stderr are *all* fully
/// uninteractive. This happens when `cargo test` runs.
///
/// There are situations where some can be interactive and others not, such as when piping
/// is active.
#[must_use]
pub fn is_fully_uninteractive_terminal() -> TTYResult {
    // Windows-specific workaround: When running through `cargo run` on Windows,
    // the terminal detection may incorrectly report all streams as non-terminal
    // even when running in an interactive terminal. This is because cargo may
    // redirect the streams. To work around this, we check if we're running
    // under cargo and if so, assume it's interactive.
    #[cfg(target_os = "windows")]
    if std::env::var("CARGO").is_ok() || std::env::var("CARGO_PKG_NAME").is_ok() {
        return TTYResult::IsInteractive;
    }

    let stdin_is_tty: bool = std::io::stdin().is_terminal();
    let stdout_is_tty: bool = std::io::stdout().is_terminal();
    let stderr_is_tty: bool = std::io::stderr().is_terminal();
    if !stdin_is_tty && !stdout_is_tty && !stderr_is_tty {
        TTYResult::IsNotInteractive
    } else {
        TTYResult::IsInteractive
    }
}

/// Returns [`TTYResult::IsNotInteractive`] if *any* of stdout or stderr are non-interactive.
///
/// This is useful for tests that need to skip when output is redirected or piped, even if
/// stdin is still interactive. For example, when running tests through a script that redirects
/// output to a file: `command >file 2>&1`
///
/// In this case:
/// - stdin: still a TTY (interactive)
/// - stdout: redirected to file (non-interactive)
/// - stderr: redirected to file (non-interactive)
///
/// This function would return `IsNotInteractive` because output streams are not fully
/// interactive, even though stdin is.
#[must_use]
pub fn is_partially_uninteractive_terminal() -> TTYResult {
    let stdout_is_tty: bool = std::io::stdout().is_terminal();
    let stderr_is_tty: bool = std::io::stderr().is_terminal();

    // If either stdout or stderr is not a TTY, consider it non-interactive
    if !stdout_is_tty || !stderr_is_tty {
        TTYResult::IsNotInteractive
    } else {
        TTYResult::IsInteractive
    }
}
