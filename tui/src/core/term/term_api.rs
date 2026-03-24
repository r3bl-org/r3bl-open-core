// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words winsize tcgetwinsize

//! High-level terminal interactivity, size detection, and [`stderr`] redirection
//! disclaimer.
//!
//! These functions build on the low-level [`TTY`] helpers in [`term_api_impl`] to provide
//! the primary API consumed by [`Spinner`], [`ReadlineAsyncContext`], [`TUI`], and tests.
//!
//! [`ReadlineAsyncContext`]: crate::ReadlineAsyncContext
//! [`Spinner`]: crate::Spinner
//! [`stderr`]: std::io::stderr
//! [`term_api_impl`]: super::term_api_impl
//! [`TTY`]: https://en.wikipedia.org/wiki/Tty_(Unix)
//! [`TUI`]: crate::tui::TerminalWindow::main_event_loop

#[allow(unused_imports)]
#[cfg(unix)]
use crate::tui::terminal_lib_backends::{TERMINAL_LIB_BACKEND, TerminalLibBackend};
use crate::{ColWidth, Size, TtyStatus, height, is_tty_stderr, is_tty_stdin,
            is_tty_stdout, width};
use miette::IntoDiagnostic;
use std::io::Write;

pub const DEFAULT_WIDTH: u16 = 80;

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

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TTYResult {
    IsInteractive,
    IsNotInteractive,
}

/// Returns [`TTYResult::IsInteractive`] if [`stdin`] is an interactive terminal
/// ([`TTY`]).
///
/// This is useful for checking if the program can receive interactive input from the
/// user. For example, a terminal multiplexer needs [`stdin`] to be a [`TTY`] to read
/// keystrokes.
///
/// [`stdin`]: std::io::stdin
/// [`TTY`]: https://en.wikipedia.org/wiki/Tty_(Unix)
#[must_use]
pub fn is_input_interactive() -> TTYResult {
    if is_tty_stdin() == TtyStatus::IsTty {
        TTYResult::IsInteractive
    } else {
        TTYResult::IsNotInteractive
    }
}

/// Returns [`TTYResult::IsInteractive`] if [`stdout`] is an interactive [`TTY`].
///
/// This is the primary check for UI components (Spinner, Readline), reflecting that the
/// TUI renders to [`stdout`] and should not be disabled by [`stderr`] redirection.
///
/// Example scenario where this returns [`TTYResult::IsInteractive`] despite redirection:
/// ```bash
/// command 2>log.txt
/// ```
///
/// [`stderr`]: std::io::stderr
/// [`stdout`]: std::io::stdout
/// [`TTY`]: https://en.wikipedia.org/wiki/Tty_(Unix)
#[must_use]
pub fn is_output_interactive() -> TTYResult {
    if is_tty_stdout() == TtyStatus::IsTty {
        TTYResult::IsInteractive
    } else {
        TTYResult::IsNotInteractive
    }
}

/// Returns [`TTYResult::IsInteractive`] only if [`stdin`], [`stdout`], AND [`stderr`] are
/// all connected to a [`TTY`].
///
/// This is the strictest check, used primarily by tests to detect a standard,
/// non-redirected terminal environment for assertions like color depth.
///
/// [`stderr`]: std::io::stderr
/// [`stdin`]: std::io::stdin
/// [`stdout`]: std::io::stdout
/// [`TTY`]: https://en.wikipedia.org/wiki/Tty_(Unix)
#[must_use]
pub fn is_fully_interactive() -> TTYResult {
    if is_tty_stdin() == TtyStatus::IsTty
        && is_tty_stdout() == TtyStatus::IsTty
        && is_tty_stderr() == TtyStatus::IsTty
    {
        TTYResult::IsInteractive
    } else {
        TTYResult::IsNotInteractive
    }
}

/// If [`stderr`] is redirected, emits a one-line disclaimer explaining that logs are
/// handled internally and only catastrophic panics will appear in the redirected stream.
///
/// This is useful for interactive applications where the user might wonder why their
/// redirected `stderr` is mostly empty.
///
/// [`stderr`]: std::io::stderr
pub fn emit_stderr_redirection_disclaimer() {
    if is_tty_stderr() == TtyStatus::IsNotTty {
        let _unused = writeln!(
            std::io::stderr(),
            "Note: stderr is redirected. Application logs are handled internally; \
             only catastrophic panics will appear here."
        );
    }
}
