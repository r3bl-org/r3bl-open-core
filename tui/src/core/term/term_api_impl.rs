// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words isatty

//! Low-level, platform-specific [`TTY`] detection helpers.
//!
//! These functions encapsulate platform differences for [`TTY`] detection. On Unix,
//! [`DirectToAnsi`] uses [`rustix`] syscalls; on other platforms, both backends use
//! [`std::io::IsTerminal`].
//!
//! Higher-level consumers should prefer the functions in the [parent module] (e.g.,
//! [`is_input_interactive()`], [`is_output_interactive()`]) rather than calling these
//! directly.
//!
//! [`DirectToAnsi`]: mod@crate::direct_to_ansi
//! [`is_input_interactive()`]: crate::is_input_interactive
//! [`is_output_interactive()`]: crate::is_output_interactive
//! [`rustix`]: rustix
//! [`TTY`]: https://en.wikipedia.org/wiki/Tty_(Unix)
//! [parent module]: super

#[allow(unused_imports)]
#[cfg(unix)]
use crate::tui::terminal_lib_backends::{TERMINAL_LIB_BACKEND, TerminalLibBackend};
use std::io::IsTerminal;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TtyStatus {
    IsTty,
    IsNotTty,
}

#[must_use]
pub fn is_tty_stdin() -> TtyStatus {
    if is_cargo_run() {
        return TtyStatus::IsTty;
    }

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
    if is_cargo_run() {
        return TtyStatus::IsTty;
    }

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
    if is_cargo_run() {
        return TtyStatus::IsTty;
    }

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

/// Windows workaround: `cargo run` redirects streams, causing false non-TTY detection.
/// When running via `cargo run`, assume interactive if cargo env vars are present.
fn is_cargo_run() -> bool {
    #[cfg(target_os = "windows")]
    {
        std::env::var("CARGO").is_ok() || std::env::var("CARGO_PKG_NAME").is_ok()
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}
