// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words isatty ECHONL VMIN VTIME iflag cflag oflag icflag lflag

//! Unix/Linux/macOS implementation of raw mode using [`rustix`]'s safe [`termios`] API.
//!
//! For background on raw mode vs cooked mode, [`TTY`] history, line disciplines, and
//! `stty`, see the [parent module's raw vs cooked section].
//!
//! # The termios Interface
//!
//! [`termios`] is the POSIX standard API for controlling terminal I/O behavior. It
//! defines a [`Termios`] struct containing flags that control:
//!
//! - **Input modes** ([`c_iflag`]): How input bytes are processed
//! - **Output modes** ([`c_oflag`]): How output bytes are processed
//! - **Control modes** ([`c_cflag`]): Hardware control (baud rate, parity)
//! - **Local modes** ([`c_lflag`]): Canonical mode, echo, signals
//! - **Special characters** ([`c_cc`]): [`VMIN`], [`VTIME`], and control characters
//!
//! The key functions are:
//! - [`tcgetattr()`]: Get current terminal attributes
//! - [`tcsetattr()`]: Set terminal attributes
//! - [`cfmakeraw()`]: Configure for raw mode (what [`Termios::make_raw()`] does)
//!
//! # Why rustix?
//!
//! This module uses [`rustix`] instead of raw libc bindings because:
//!
//! 1. **Type safety**: Strong typing prevents mixing up file descriptors
//! 2. **Memory safety**: No raw pointers or manual memory management
//! 3. **Ergonomics**: Methods like [`Termios::make_raw()`] encapsulate complex flag
//!    manipulation
//! 4. **Correctness**: Handles platform differences (Linux vs macOS vs BSD)
//!
//! # See Also
//!
//! - [Parent module documentation] for conceptual overview
//! - [`enable_raw_mode()`] and [`disable_raw_mode()`] for the public API
//!
//! [`c_cc`]: https://man7.org/linux/man-pages/man3/termios.3.html
//! [`c_cflag`]: https://man7.org/linux/man-pages/man3/termios.3.html
//! [`c_iflag`]: https://man7.org/linux/man-pages/man3/termios.3.html
//! [`c_lflag`]: https://man7.org/linux/man-pages/man3/termios.3.html
//! [`c_oflag`]: https://man7.org/linux/man-pages/man3/termios.3.html
//! [`cfmakeraw()`]: https://man7.org/linux/man-pages/man3/cfmakeraw.3.html
//! [`disable_raw_mode()`]: crate::disable_raw_mode
//! [`enable_raw_mode()`]: crate::enable_raw_mode
//! [`rustix`]: https://docs.rs/rustix
//! [`tcgetattr()`]: rustix::termios::tcgetattr
//! [`tcsetattr()`]: rustix::termios::tcsetattr
//! [`Termios::make_raw()`]: rustix::termios::Termios::make_raw
//! [`termios`]: rustix::termios
//! [`Termios`]: rustix::termios::Termios
//! [`TTY`]: https://en.wikipedia.org/wiki/Tty_(Unix)
//! [`VMIN`]: https://man7.org/linux/man-pages/man3/termios.3.html
//! [`VTIME`]: https://man7.org/linux/man-pages/man3/termios.3.html
//! [Parent module documentation]: mod@crate::terminal_raw_mode
//! [parent module's raw vs cooked section]:
//!     mod@crate::terminal_raw_mode#raw-mode-vs-cooked-mode
//! [termios]: https://man7.org/linux/man-pages/man3/termios.3.html

use crate::{TtyStatus, is_tty_stdin};
use miette::miette;
use rustix::{fd::{AsFd, BorrowedFd},
             termios::{self, OptionalActions, Termios}};
use std::{fs::File,
          io,
          sync::{LazyLock, Mutex}};

/// Stores the terminal settings saved by [`enable_raw_mode()`] before entering raw mode,
/// so they can be restored by [`disable_raw_mode()`].
pub static SAVED_TERMIOS: LazyLock<Mutex<Option<Termios>>> =
    LazyLock::new(|| Mutex::new(None));

/// Enables raw mode on the terminal (Unix/Linux/macOS implementation).
///
/// Uses [`rustix`]'s type-safe [`termios`] API to:
/// 1. Get the controlling terminal ([`stdin`] if it's a [`TTY`], otherwise [`/dev/tty`])
/// 2. Save the original terminal settings for restoration
/// 3. Disable canonical mode, echo, and signal generation
/// 4. Set [`VMIN`]=1, [`VTIME`]=0 for immediate byte-by-byte reading
///
/// # Crossterm approach
///
/// This implementation follows [`crossterm`]'s approach: checks if [`stdin`] is a [`TTY`]
/// and uses it if so; otherwise opens [`/dev/tty`]. This handles cases where [`stdin`] is
/// redirected, e.g.:
/// ```bash
/// echo "data" | your_app
/// ```
///
/// Additionally, it uses [`rustix`]'s built-in [`make_raw()`] method which correctly
/// implements [`cfmakeraw`] behavior. This is the same approach [`crossterm`] uses (see
/// [`crossterm-0.28/src/terminal/sys/unix.rs:124`]). [`make_raw()`] handles all the
/// necessary terminal attribute changes including:
/// - Disabling canonical mode ([`ICANON`])
/// - Disabling signal generation ([`ISIG`])
/// - Disabling echo ([`ECHO`], [`ECHONL`])
/// - Setting special character processing ([`VMIN`]=1, [`VTIME`]=0)
/// - Properly handling [`VEOF`] and other special characters
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
/// [`/dev/tty`]: https://man7.org/linux/man-pages/man4/tty.4.html
/// [`cfmakeraw`]: https://man7.org/linux/man-pages/man3/cfmakeraw.3.html
/// [`crossterm-0.28/src/terminal/sys/unix.rs:124`]:
///     https://github.com/crossterm-rs/crossterm/blob/0.28/src/terminal/sys/unix.rs#L124
/// [`crossterm`]: https://docs.rs/crossterm
/// [`ECHO`]: https://man7.org/linux/man-pages/man3/termios.3.html
/// [`ECHONL`]: https://man7.org/linux/man-pages/man3/termios.3.html
/// [`ICANON`]: https://man7.org/linux/man-pages/man3/termios.3.html
/// [`ISIG`]: https://man7.org/linux/man-pages/man3/termios.3.html
/// [`make_raw()`]: rustix::termios::Termios::make_raw
/// [`rustix`]: https://docs.rs/rustix
/// [`stdin`]: std::io::stdin
/// [`termios`]: rustix::termios
/// [`TTY`]: https://en.wikipedia.org/wiki/Tty_(Unix)
/// [`VEOF`]: https://man7.org/linux/man-pages/man3/termios.3.html
/// [`VMIN`]: https://man7.org/linux/man-pages/man3/termios.3.html
/// [`VTIME`]: https://man7.org/linux/man-pages/man3/termios.3.html
/// [module documentation]: mod@crate::terminal_raw_mode
pub fn enable_raw_mode() -> miette::Result<()> {
    let fd = terminal_fd::get()
        .map_err(|e| miette::miette!("failed to get terminal file descriptor: {e}"))?;

    let mut termios = termios::tcgetattr(&fd)
        .map_err(|e| miette::miette!("failed to retrieve terminal attributes: {e}"))?;

    // Save original settings.
    {
        let mut guard_saved_termios = SAVED_TERMIOS
            .lock()
            .map_err(|e| miette!("terminal settings lock poisoned: {e}"))?;

        if guard_saved_termios.is_none() {
            // rustix's Termios doesn't implement Copy, so we need to clone.
            *guard_saved_termios = Some(termios.clone());
        }
    }

    // See "Crossterm approach" section in the rustdoc above for details.
    termios.make_raw();

    // Apply the new settings
    termios::tcsetattr(&fd, OptionalActions::Now, &termios)
        .map_err(|e| miette::miette!("failed to set terminal attributes: {e}"))?;

    Ok(())
}

/// Disable raw mode and restore original terminal settings (Unix/Linux/macOS
/// implementation).
///
/// Restores the terminal settings saved by [`enable_raw_mode()`]. Uses the same terminal
/// file descriptor selection logic as [`enable_raw_mode()`] ([`stdin`] if it's a [`TTY`],
/// otherwise [`/dev/tty`]). No-op if raw mode was never enabled.
///
/// # Errors
///
/// Returns [`miette`] diagnostic errors if:
/// - Terminal file descriptor cannot be obtained.
/// - Terminal attributes cannot be set.
/// - [`Mutex`] lock is poisoned.
///
/// [`/dev/tty`]: https://man7.org/linux/man-pages/man4/tty.4.html
/// [`enable_raw_mode()`]: crate::enable_raw_mode
/// [`miette`]: mod@miette
/// [`stdin`]: std::io::stdin
/// [`TTY`]: https://en.wikipedia.org/wiki/Tty_(Unix)
pub fn disable_raw_mode() -> miette::Result<()> {
    let mut guard_saved_termios = SAVED_TERMIOS
        .lock()
        .map_err(|e| miette!("terminal settings lock poisoned: {e}"))?;

    if let Some(ref saved_termios) = *guard_saved_termios {
        let fd = terminal_fd::get().map_err(|e| {
            miette::miette!("failed to get terminal file descriptor: {e}")
        })?;
        termios::tcsetattr(&fd, OptionalActions::Now, saved_termios)
            .map_err(|e| miette::miette!("failed to set terminal attributes: {e}"))?;
    }

    // Clear so a subsequent enable_raw_mode() saves a fresh snapshot.
    *guard_saved_termios = None;

    ok!()
}

mod terminal_fd {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Represents either stdin or [`/dev/tty`] for terminal operations.
    ///
    /// This enum allows us to handle both cases where stdin is a [`TTY`] (normal terminal
    /// usage) and where stdin is redirected (e.g., piped input), requiring us to use
    /// [`/dev/tty`].
    ///
    /// [`/dev/tty`]: https://man7.org/linux/man-pages/man4/tty.4.html
    /// [`TTY`]: https://en.wikipedia.org/wiki/Tty_(Unix)
    pub enum TerminalFd {
        /// Using standard input (when it's a terminal)
        Stdin(io::Stdin),
        /// Using [`/dev/tty`] (when stdin is redirected)
        ///
        /// [`/dev/tty`]: https://man7.org/linux/man-pages/man4/tty.4.html
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
    /// This function implements a robust strategy for acquiring a terminal handle,
    /// essential for enabling raw mode regardless of whether input is redirected:
    ///
    /// 1. **Check Stdin**: It first checks if [`stdin`] is a [`TTY`]. If so, it uses
    ///    [`stdin`] as the primary handle for terminal configuration.
    /// 2. **Fallback to [`/dev/tty`]**: If [`stdin`] is not a [`TTY`] (e.g., when input
    ///    is redirected from a pipe or file like `cat data.txt | my_app`), it attempts to
    ///    open [`/dev/tty`] directly.
    ///
    /// ### What is [`/dev/tty`]?
    ///
    /// [`/dev/tty`] is a special file in Unix-like systems that acts as a "backdoor" to
    /// the **controlling terminal** of the current process. Even when [`stdin`] is
    /// redirected to a pipe, [`/dev/tty`] still provides access to the physical
    /// terminal device. This allows the application to call [`tcsetattr`] and enable
    /// raw mode on the actual terminal window where the user is interacting.
    ///
    /// ### Design Rationale: Backend-Aware [`TTY`] Detection
    ///
    /// This function uses the crate's centralized [`TTY`] detection logic (via
    /// [`term.rs`]) to ensure consistency. This "backend-aware" routing ensures that
    /// the entire crate respects the same [`TERMINAL_LIB_BACKEND`] configuration when
    /// determining terminal interactivity.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`stdin`] is not a [`TTY`] AND [`/dev/tty`] cannot be opened.
    /// - The process lacks permissions to open the controlling terminal.
    ///
    /// [`/dev/tty`]: https://man7.org/linux/man-pages/man4/tty.4.html
    /// [`stdin`]: std::io::stdin
    /// [`tcsetattr`]: termios::tcsetattr
    /// [`term.rs`]: mod@crate::term
    /// [`TERMINAL_LIB_BACKEND`]: crate::TERMINAL_LIB_BACKEND
    /// [`TTY`]: https://en.wikipedia.org/wiki/Tty_(Unix)
    pub fn get() -> io::Result<TerminalFd> {
        if is_tty_stdin() == TtyStatus::IsTty {
            let stdin = io::stdin();
            Ok(TerminalFd::Stdin(stdin))
        } else {
            let file = File::options().read(true).write(true).open("/dev/tty")?;
            Ok(TerminalFd::DevTty(file))
        }
    }
}
