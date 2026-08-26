// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words winsize tcgetwinsize devtty

//! Terminal interactivity, size detection, and [`stderr`] redirection disclaimer.
//! See [`TerminalInteractiveStatus`], [`TerminalNotInteractiveReason`], and
//! [`emit_stderr_redirection_disclaimer()`].
//!
//! [`stderr`]: std::io::stderr

use super::constants::{DEFAULT_WIDTH, ERR_MSG_BOTH_NOT_INTERACTIVE,
                       ERR_MSG_STDIN_NOT_INTERACTIVE, ERR_MSG_STDOUT_NOT_INTERACTIVE};
use crate::{AtomicU8Ext as _, IntoErr,
            TTYResult::{IsInteractive, IsNotInteractive},
            TtyStatus, VPSize, VPWidth, is_tty_stderr, is_tty_stdin, is_tty_stdout,
            vp_height, vp_width};
use miette::IntoDiagnostic;
use std::{io::Write, sync::atomic::AtomicU8};

/// Returned by [interactive terminal application entry points] (which are fallible).
///
/// Initialization ([`get_size()`], entering [raw mode], etc.) is fallible, since they
/// require the use of [`ioctl`] (which is wrapped by [`rustix`]), so this type has a
/// [`Broken`] variant to represent this state.
///
/// [`Broken`]: Self::Broken
/// [`ioctl`]: https://man7.org/linux/man-pages/man2/ioctl.2.html
/// [`rustix`]: rustix
/// [interactive terminal application entry points]: crate#interactive-terminal-application-entry-points
#[derive(Debug)]
pub enum TuiAvailability<T> {
    Available(T),
    NotAvailable(TerminalNotInteractiveReason),
    Broken(miette::Report),
}

impl<T> IntoErr for TuiAvailability<T> {
    fn into_err<U>(self) -> miette::Result<U> {
        match self {
            Self::Available(_) => {
                unreachable!("logic error: into_err() called on Available")
            }
            Self::NotAvailable(reason) => reason.into_err(),
            Self::Broken(report) => report.into_err(),
        }
    }
}

/// Represents the interactivity status of the terminal streams ([`stdin`] and
/// [`stdout`]).
///
/// This does not represent any fallible states. [`check_is_terminal_interactive()`]
/// returns this.
///
/// # Why Terminal Interactivity Checking Exists
///
/// Different parts of this codebase query terminal interactivity before operating:
/// - **[`Spinner`]** requires [`stdout`] to be an interactive [`TTY`] before animating;
///   writing spinner frames to a pipe or file produces garbage.
/// - **[`ReadlineAsyncContext`]** requires both [`stdin`] (to read keystrokes) and
///   [`stdout`] (to render the prompt) to be interactive [`TTY`]s.
/// - **[Color detection]** queries [`stdout`] / [`stderr`] to decide whether to emit
///   [`ANSI`] color codes.
/// - **[Raw mode]** needs a terminal file descriptor to call [`tcsetattr`]; if [`stdin`]
///   is redirected, it falls back to [`/dev/tty`].
/// - **Tests** distinguish a real terminal from CI pipe environments to adjust assertions
///   like expected color depth.
///
/// # Interactivity Levels
///
/// 1. **Input Interactivity** ([`is_input_interactive()`]): Can we read keystrokes and
///    mouse events from [`stdin`]?
/// 2. **Output Interactivity** ([`is_output_interactive()`]): Can we render the TUI to
///    [`stdout`]? (Redirection of [`stderr`] does not disable the TUI).
/// 3. **Full Interactivity** ([`is_fully_interactive()`]): Are all three streams
///    ([`stdin`], [`stdout`], [`stderr`]) connected to a [`TTY`]?
///
/// # Shell Redirection and Pipeline Behavior
///
/// When a process runs normally in a terminal, [`stdin`] ([`fd`] `0`), [`stdout`] ([`fd`]
/// `1`), and [`stderr`] ([`fd`] `2`) all point to the active terminal device
/// ([`/dev/pts/*`] or [`/dev/tty*`]), so [`isatty`] returns `true`.
///
/// Pipelines and file redirections swap the character device for an anonymous pipe or
/// file handle, changing the interactivity status:
///
/// | Scenario              | Description                         | [`stdin`] ([`fd`] `0`) | [`stdout`] ([`fd`] `1`) | [`stderr`] ([`fd`] `2`) | Interactivity Status                   |
/// | :-------------------- | :---------------------------------- | :--------------------- | :---------------------- | :---------------------- | :------------------------------------- |
/// | **Normal Terminal**   | Direct interactive terminal session | TTY ([`/dev/pts/*`])   | TTY ([`/dev/pts/*`])    | TTY ([`/dev/pts/*`])    | [`Available`]                          |
/// | **Piped Input**       | Input piped from another process    | Pipe (`false`)         | TTY ([`/dev/pts/*`])    | TTY ([`/dev/pts/*`])    | [`NotAvailable(StdinNotInteractive)`]  |
/// | **Piped Output**      | Output piped into another process   | TTY ([`/dev/pts/*`])   | Pipe (`false`)          | TTY ([`/dev/pts/*`])    | [`NotAvailable(StdoutNotInteractive)`] |
/// | **Redirected Stderr** | Error stream redirected to a file   | TTY ([`/dev/pts/*`])   | TTY ([`/dev/pts/*`])    | File (`false`)          | [`Available`] (logs decoupled)         |
///
/// Examples in bash:
/// 
/// ```bash
/// # Normal terminal:
/// my_app
///
/// # Piped input:
/// cat data.txt | my_app
///
/// # Piped output:
/// my_app | grep "pattern"
///
/// # Redirected stderr:
/// my_app 2> errors.log
/// ```
///
/// [`/dev/pts/*`]: crate::pty_engine::pty_pair::PtyPair#child-process-perspective
/// [`/dev/tty*`]: crate::pty_engine::pty_pair::PtyPair#what-is-a-tty
/// [`/dev/tty`]: crate::pty_engine::pty_pair::PtyPair#controlling-terminal-alias-devtty
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`Available`]: Self::Available
/// [`fd`]: https://man7.org/linux/man-pages/man2/open.2.html
/// [`isatty`]: https://man7.org/linux/man-pages/man3/isatty.3.html
/// [`NotAvailable(StdinNotInteractive)`]:
///     TerminalNotInteractiveReason::StdinNotInteractive
/// [`NotAvailable(StdoutNotInteractive)`]:
///     TerminalNotInteractiveReason::StdoutNotInteractive
/// [`ReadlineAsyncContext`]: crate::ReadlineAsyncContext
/// [`Spinner`]: crate::Spinner
/// [`stderr`]: std::io::stderr
/// [`stdin`]: std::io::stdin
/// [`stdout`]: std::io::stdout
/// [`tcsetattr`]: https://man7.org/linux/man-pages/man3/tcsetattr.3.html
/// [`TTY`]: https://en.wikipedia.org/wiki/Tty_(Unix)
/// [Color detection]: crate::examine_env_vars_to_determine_color_support
/// [Raw mode]: crate::terminal_raw_mode
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TerminalInteractiveStatus {
    Available,
    NotAvailable(TerminalNotInteractiveReason),
}

/// Gets the interactivity status of the terminal by querying [`isatty`] on [`stdin`]
/// and [`stdout`], which is infallible, so there is no [`Broken`] variant.
///
/// [`Broken`]: TuiAvailability::Broken
/// [`isatty`]: https://man7.org/linux/man-pages/man3/isatty.3.html
/// [`stdin`]: std::io::stdin
/// [`stdout`]: std::io::stdout
#[must_use]
pub fn check_is_terminal_interactive() -> TerminalInteractiveStatus {
    match (is_input_interactive(), is_output_interactive()) {
        (IsInteractive, IsInteractive) => TerminalInteractiveStatus::Available,
        (IsNotInteractive, IsInteractive) => TerminalInteractiveStatus::NotAvailable(
            TerminalNotInteractiveReason::StdinNotInteractive,
        ),
        (IsInteractive, IsNotInteractive) => TerminalInteractiveStatus::NotAvailable(
            TerminalNotInteractiveReason::StdoutNotInteractive,
        ),
        (IsNotInteractive, IsNotInteractive) => TerminalInteractiveStatus::NotAvailable(
            TerminalNotInteractiveReason::BothStdinAndStdoutNotInteractive,
        ),
    }
}

/// This is a convenience function for interactive terminal applications to ensure that
/// the terminal is indeed interactive. It checks both [`stdin`] and [`stdout`] and prints
/// the appropriate error message to [`stderr`] before [exiting with code `1`].
///
/// # Panics
///
/// Exits the process with an error message if the terminal is not interactive.
///
/// [`stderr`]: std::io::stderr
/// [`stdin`]: std::io::stdin
/// [`stdout`]: std::io::stdout
/// [exiting with code `1`]: std::process::exit(1)
pub fn assert_terminal_is_interactive() {
    match check_is_terminal_interactive() {
        TerminalInteractiveStatus::Available => {}
        TerminalInteractiveStatus::NotAvailable(reason) => {
            eprintln!("{}", reason.as_err_msg());
            std::process::exit(1);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalNotInteractiveReason {
    StdinNotInteractive,
    StdoutNotInteractive,
    BothStdinAndStdoutNotInteractive,
}

impl TerminalNotInteractiveReason {
    #[must_use]
    pub fn as_err_msg(&self) -> &'static str {
        match self {
            Self::StdinNotInteractive => ERR_MSG_STDIN_NOT_INTERACTIVE,
            Self::StdoutNotInteractive => ERR_MSG_STDOUT_NOT_INTERACTIVE,
            Self::BothStdinAndStdoutNotInteractive => ERR_MSG_BOTH_NOT_INTERACTIVE,
        }
    }
}

impl IntoErr for TerminalNotInteractiveReason {
    fn into_err<T>(self) -> miette::Result<T> {
        Err(miette::miette!("{}", self.as_err_msg()))
    }
}

/// Tracks whether [`emit_stderr_redirection_disclaimer()`] has already run.
/// `0` = not yet emitted, `1` = already emitted.
static DISCLAIMER_ALREADY_EMITTED: AtomicU8 = AtomicU8::new(0);

#[must_use]
pub fn get_terminal_width_no_default() -> Option<VPWidth> {
    match get_size() {
        Ok(size) => Some(size.col_width),
        Err(_) => None,
    }
}

/// Gets the terminal width. If there is a problem, return `DEFAULT_WIDTH`.
#[must_use]
pub fn get_terminal_width() -> VPWidth {
    match get_size() {
        Ok(size) => size.col_width,
        Err(_) => vp_width(DEFAULT_WIDTH),
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
/// [`Crossterm backend`]: crate::tui::TerminalLibBackend::Crossterm
/// [`crossterm`]: crossterm
/// [`DirectToAnsi`]: mod@crate::direct_to_ansi
/// [`rustix`]: rustix
/// [`tcgetwinsize`]: fn@rustix::termios::tcgetwinsize
/// [`TERMINAL_LIB_BACKEND`]: crate::tui::TERMINAL_LIB_BACKEND
/// [`TTY`]: https://en.wikipedia.org/wiki/Tty_(Unix)
pub fn get_size() -> miette::Result<VPSize> {
    #[cfg(unix)]
    {
        use crate::tui::terminal_lib_backends::{TERMINAL_LIB_BACKEND, TerminalLibBackend};

        match TERMINAL_LIB_BACKEND {
            TerminalLibBackend::Crossterm => {
                let (columns, rows) = crossterm::terminal::size().into_diagnostic()?;
                Ok(vp_width(columns) + vp_height(rows))
            }
            TerminalLibBackend::DirectToAnsi => {
                let winsize = rustix::termios::tcgetwinsize(std::io::stdout())
                    .map_err(|e| miette::miette!("tcgetwinsize failed: {}", e))?;
                Ok(vp_width(winsize.ws_col) + vp_height(winsize.ws_row))
            }
        }
    }
    #[cfg(not(unix))]
    {
        let (columns, rows) = crossterm::terminal::size().into_diagnostic()?;
        Ok(vp_width(columns) + vp_height(rows))
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
/// This is the primary check for UI components ([`Spinner`], [`ReadlineAsyncContext`],
/// [`TUI`], [`PTYMux`], [`choose()`]), reflecting that the TUI renders to [`stdout`] and
/// should not be disabled by [`stderr`] redirection. Diagnostic output (tracing, logs)
/// is routed to [`stderr`] via [`DisplayPreference::Stderr`], keeping it separate from
/// TUI rendering.
///
/// Example scenario where this returns [`TTYResult::IsInteractive`] despite redirection:
/// ```bash
/// command 2>log.txt
/// ```
/// In this case, [`emit_stderr_redirection_disclaimer()`] is called to notify the user
/// that [`stderr`] is redirected.
///
/// [`choose()`]: crate::choose
/// [`DisplayPreference::Stderr`]: crate::DisplayPreference::Stderr
/// [`emit_stderr_redirection_disclaimer()`]: crate::emit_stderr_redirection_disclaimer
/// [`PTYMux`]: crate::pty_mux::PTYMux
/// [`ReadlineAsyncContext`]: crate::readline_async::ReadlineAsyncContext
/// [`Spinner`]: crate::readline_async::Spinner
/// [`stderr`]: std::io::stderr
/// [`stdout`]: std::io::stdout
/// [`TTY`]: https://en.wikipedia.org/wiki/Tty_(Unix)
/// [`TUI`]: crate::tui::TerminalWindow::main_event_loop
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
/// This function is idempotent; calling it multiple times will only result in a single
/// message being printed per application lifetime.
///
/// This is useful for interactive applications where the user might wonder why their
/// redirected [`stderr`] is mostly empty.
///
/// [`stderr`]: std::io::stderr
pub fn emit_stderr_redirection_disclaimer() {
    if DISCLAIMER_ALREADY_EMITTED.get() != 0 {
        return;
    }

    if is_tty_stderr() == TtyStatus::IsNotTty {
        let _unused = writeln!(
            std::io::stderr(),
            "Note: stderr is redirected. Application logs are handled internally; \
             only catastrophic panics will appear here."
        );
    }

    DISCLAIMER_ALREADY_EMITTED.set(1);
}
