// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words isatty winsize tcgetwinsize

#[allow(unused_imports)]
#[cfg(unix)]
use crate::tui::terminal_lib_backends::{TERMINAL_LIB_BACKEND, TerminalLibBackend};
use crate::{ColWidth, Size, height, width};
use miette::IntoDiagnostic;
use std::io::IsTerminal;

pub const DEFAULT_WIDTH: u16 = 80;

// ┌──────────────────────────────────────────────────────────────────────────────┐
// │ Platform-specific TTY helpers                                                │
// │                                                                              │
// │ These functions encapsulate platform differences for TTY detection.          │
// │ On Unix, DirectToAnsi uses rustix syscalls; on other platforms, both         │
// │ backends use std::io::IsTerminal.                                            │
// └──────────────────────────────────────────────────────────────────────────────┘

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TtyStatus {
    IsTty,
    IsNotTty,
}

#[must_use]
pub fn is_tty_stdin() -> TtyStatus {
    #[cfg(unix)]
    {
        let result = match TERMINAL_LIB_BACKEND {
            TerminalLibBackend::Crossterm => std::io::stdin().is_terminal(),
            TerminalLibBackend::DirectToAnsi => rustix::termios::isatty(std::io::stdin()),
        };
        if result {
            TtyStatus::IsTty
        } else {
            TtyStatus::IsNotTty
        }
    }
    #[cfg(not(unix))]
    {
        if std::io::stdin().is_terminal() {
            TtyStatus::IsTty
        } else {
            TtyStatus::IsNotTty
        }
    }
}

#[must_use]
pub fn is_tty_stdout() -> TtyStatus {
    #[cfg(unix)]
    {
        let result = match TERMINAL_LIB_BACKEND {
            TerminalLibBackend::Crossterm => std::io::stdout().is_terminal(),
            TerminalLibBackend::DirectToAnsi => {
                rustix::termios::isatty(std::io::stdout())
            }
        };
        if result {
            TtyStatus::IsTty
        } else {
            TtyStatus::IsNotTty
        }
    }
    #[cfg(not(unix))]
    {
        if std::io::stdout().is_terminal() {
            TtyStatus::IsTty
        } else {
            TtyStatus::IsNotTty
        }
    }
}

#[must_use]
pub fn is_tty_stderr() -> TtyStatus {
    #[cfg(unix)]
    {
        let result = match TERMINAL_LIB_BACKEND {
            TerminalLibBackend::Crossterm => std::io::stderr().is_terminal(),
            TerminalLibBackend::DirectToAnsi => {
                rustix::termios::isatty(std::io::stderr())
            }
        };
        if result {
            TtyStatus::IsTty
        } else {
            TtyStatus::IsNotTty
        }
    }
    #[cfg(not(unix))]
    {
        if std::io::stderr().is_terminal() {
            TtyStatus::IsTty
        } else {
            TtyStatus::IsNotTty
        }
    }
}

#[must_use]
pub fn get_terminal_width_no_default() -> Option<ColWidth> {
    match get_size() {
        Ok(size) => Some(size.col_width),
        Err(_) => None,
    }
}

/// Gets the terminal width. If there is a problem, return the default width.
#[must_use]
pub fn get_terminal_width() -> ColWidth {
    match get_size() {
        Ok(size) => size.col_width,
        Err(_) => width(DEFAULT_WIDTH),
    }
}

/// Gets the terminal size.
///
/// Uses [`crossterm`] for the [`Crossterm backend`], or [`rustix`] [`tcgetwinsize`]
/// syscall for the [`DirectToAnsi`] backend. On non-Unix platforms (Windows), this always
/// uses [`Crossterm backend`] regardless of [`TERMINAL_LIB_BACKEND`] since
/// [`DirectToAnsi`] is Linux-only.
///
/// # Errors
///
/// Returns an error if:
/// - The terminal size cannot be determined.
/// - The terminal is not available or not a [`TTY`].
///
/// [`Crossterm backend`]: crate::TerminalLibBackend::Crossterm
/// [`crossterm`]: crossterm
/// [`DirectToAnsi`]: mod@crate::direct_to_ansi
/// [`rustix`]: rustix
/// [`tcgetwinsize`]: fn@rustix::termios::tcgetwinsize
/// [`TERMINAL_LIB_BACKEND`]: crate::TERMINAL_LIB_BACKEND
/// [`TTY`]: https://en.wikipedia.org/wiki/Tty_(Unix)
pub fn get_size() -> miette::Result<Size> {
    #[cfg(unix)]
    {
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
    #[cfg(not(unix))]
    {
        let (columns, rows) = crossterm::terminal::size().into_diagnostic()?;
        Ok(width(columns) + height(rows))
    }
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

/// Returns [`StdinIsPiped`] when [`stdin`] is redirected (e.g., `echo "test" | cargo
/// run`).
///
/// A pipe replaces the terminal [`fd`] with a pipe [`fd`], so "not a [`TTY`]" is
/// equivalent to "piped". This function wraps [`is_tty_stdin()`] and inverts the
/// interpretation.
///
/// See [this explanation] for how piping affects [`stdin`].
///
/// [`fd`]: https://man7.org/linux/man-pages/man2/open.2.html
/// [`is_tty_stdin()`]: crate::is_tty_stdin
/// [`stdin`]: std::io::stdin
/// [`StdinIsPiped`]: StdinIsPipedResult::StdinIsPiped
/// [`TTY`]: https://en.wikipedia.org/wiki/Tty_(Unix)
/// [this explanation]:
///     https://unix.stackexchange.com/questions/597083/how-does-piping-affect-stdin
#[must_use]
pub fn is_stdin_piped() -> StdinIsPipedResult {
    if is_tty_stdin() == TtyStatus::IsTty {
        StdinIsPipedResult::StdinIsNotPiped
    } else {
        StdinIsPipedResult::StdinIsPiped
    }
}

/// Returns [`StdoutIsPiped`] when [`stdout`] is redirected (e.g., `cargo run | grep
/// foo`).
///
/// A pipe replaces the terminal [`fd`] with a pipe [`fd`], so "not a [`TTY`]" is
/// equivalent to "piped". This function wraps [`is_tty_stdout()`] and inverts the
/// interpretation.
///
/// See [this explanation] for how piping affects [`stdout`].
///
/// [`fd`]: https://man7.org/linux/man-pages/man2/open.2.html
/// [`is_tty_stdout()`]: crate::is_tty_stdout
/// [`stdout`]: std::io::stdout
/// [`StdoutIsPiped`]: StdoutIsPipedResult::StdoutIsPiped
/// [`TTY`]: https://en.wikipedia.org/wiki/Tty_(Unix)
/// [this explanation]:
///     https://unix.stackexchange.com/questions/597083/how-does-piping-affect-stdin
#[must_use]
pub fn is_stdout_piped() -> StdoutIsPipedResult {
    if is_tty_stdout() == TtyStatus::IsTty {
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

/// Returns [`TTYResult::IsInteractive`] if stdin is an interactive terminal ([`TTY`]).
///
/// This is useful for checking if the program can receive interactive input from the
/// user. For example, a terminal multiplexer needs stdin to be a [`TTY`] to read
/// keystrokes.
///
/// Note: This only checks stdin. Use [`is_headless`] to check if *all* streams
/// ([`stdin`], [`stdout`], [`stderr`]) are non-interactive, or [`is_output_interactive`]
/// to check if output streams are interactive.
///
/// [`stderr`]: std::io::stderr
/// [`stdin`]: std::io::stdin
/// [`stdout`]: std::io::stdout
/// [`TTY`]: https://en.wikipedia.org/wiki/Tty_(Unix)
#[must_use]
pub fn is_stdin_interactive() -> TTYResult {
    if is_tty_stdin() == TtyStatus::IsTty {
        TTYResult::IsInteractive
    } else {
        TTYResult::IsNotInteractive
    }
}

/// Returns [`TTYResult::IsNotInteractive`] if [`stdin`], [`stdout`], and [`stderr`] are
/// *all* non-interactive (not [`TTY`]s). This typically happens when running under
/// [`cargo test`] or in other headless/batch environments.
///
/// Use this to detect fully non-interactive environments where no terminal I/O is
/// possible.
///
/// [`cargo test`]: https://doc.rust-lang.org/cargo/commands/cargo-test.html
/// [`stderr`]: std::io::stderr
/// [`stdin`]: std::io::stdin
/// [`stdout`]: std::io::stdout
/// [`TTY`]: https://en.wikipedia.org/wiki/Tty_(Unix)
#[must_use]
pub fn is_headless() -> TTYResult {
    // Windows workaround: cargo redirects streams, causing false non-TTY detection.
    // When running via `cargo run`, assume interactive if cargo env vars are present.
    #[cfg(target_os = "windows")]
    if std::env::var("CARGO").is_ok() || std::env::var("CARGO_PKG_NAME").is_ok() {
        return TTYResult::IsInteractive;
    }

    if is_tty_stdin() == TtyStatus::IsNotTty
        && is_tty_stdout() == TtyStatus::IsNotTty
        && is_tty_stderr() == TtyStatus::IsNotTty
    {
        TTYResult::IsNotInteractive
    } else {
        TTYResult::IsInteractive
    }
}

/// Returns [`TTYResult::IsInteractive`] if both [`stdout`] and [`stderr`] are interactive
/// [`TTY`]s.
///
/// This is useful for checking if the program can display output to an interactive
/// terminal. Returns [`TTYResult::IsNotInteractive`] if *either* [`stdout`] or [`stderr`]
/// is redirected or piped.
///
/// Example scenario where this returns [`TTYResult::IsNotInteractive`]:
/// ```bash
/// command >file 2>&1
/// ```
/// Here [`stdin`] may still be a [`TTY`], but output streams are redirected to a file.
///
/// [`stderr`]: std::io::stderr
/// [`stdin`]: std::io::stdin
/// [`stdout`]: std::io::stdout
/// [`TTY`]: https://en.wikipedia.org/wiki/Tty_(Unix)
#[must_use]
pub fn is_output_interactive() -> TTYResult {
    // If either stdout or stderr is not a TTY, consider output non-interactive.
    if is_tty_stdout() == TtyStatus::IsNotTty || is_tty_stderr() == TtyStatus::IsNotTty {
        TTYResult::IsNotInteractive
    } else {
        TTYResult::IsInteractive
    }
}
