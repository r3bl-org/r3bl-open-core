// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words iflag cflag lflag

//! Terminal raw mode implementation for ANSI terminals.
//!
//! This module provides functionality to enable and disable raw mode on terminals,
//! which is essential for reading ANSI escape sequences character-by-character
//! without line buffering or terminal interpretation.
//!
//! ## Raw Mode vs Cooked Mode
//!
//! **Cooked Mode** (the default when a terminal is opened):
//! - Input is line-buffered (waits for Enter key)
//! - Special characters are interpreted (`Ctrl+C`, `Ctrl+D`, etc.)
//! - ANSI escape sequences may be processed by the terminal
//! - Echoing is enabled (typed characters appear on screen)
//!
//! **Raw Mode**:
//! - No line buffering - bytes available immediately
//! - No special character processing - all bytes pass through
//! - No echo - typed characters don't automatically appear
//! - Perfect for reading ANSI escape sequences and building TUIs
//!
//! ## TTY, Line Discipline, and `stty`
//!
//! ### Historical Context
//!
//! The term `TTY` comes from "teletypewriter" — physical terminals from the 1960s-70s
//! that communicated with mainframes over serial lines. Modern terminal emulators (like
//! [Terminator], [GNOME Terminal], [WezTerm], [iTerm2], or [Alacritty]) still use this
//! abstraction: they create a **pseudo-terminal ([PTY])** that behaves like those old
//! hardware devices.
//!
//! ### The Line Discipline
//!
//! Between your terminal and the programs reading input sits the **line
//! discipline** — a kernel-level layer that:
//!
//! - **Buffers input** line-by-line (so you can edit before pressing Enter)
//! - **Interprets special characters** (Ctrl+C sends `SIGINT`, Ctrl+D sends `EOF`)
//! - **Echoes characters** back to the screen as you type
//! - **Processes editing keys** (backspace, arrow keys for line editing)
//!
//! This is "cooked" mode (canonical mode). In "raw" mode (non-canonical mode),
//! the line discipline stops buffering and processing — bytes flow directly
//! from the terminal to your program. This is what TUI applications need to
//! capture every keystroke, including escape sequences.
//!
//! ### Keybinding Handling Layers
//!
//! By default, terminals start in **canonical (cooked) mode**, where the
//! kernel's line discipline handles basic editing. But when applications like
//! [Bash] need richer editing features, they switch to **non-canonical (raw)
//! mode** and let a user-space library (like [GNU Readline]) handle input
//! instead:
//!
//! | Aspect       | Kernel Line Discipline (`N_TTY`)  | User-Space Library ([GNU Readline])      |
//! |:-------------|:----------------------------------|:-----------------------------------------|
//! | **Location** | Inside the Linux kernel           | Part of the shell ([Bash], Python, etc.) |
//! | **Active**   | Canonical ("Cooked") Mode         | Non-Canonical ("Raw") Mode               |
//! | **Purpose**  | Basic, ancient terminal functions | Advanced, feature-rich line editing      |
//!
//! **Kernel-handled keybindings** (when in canonical mode):
//! - `Ctrl+C` — Generates `SIGINT` (interrupt signal)
//! - `Ctrl+Z` — Generates `SIGTSTP` (suspend signal)
//! - `Ctrl+U` — `VKILL` character (kill entire line)
//! - `Ctrl+D` — `VEOF` character (end-of-file)
//!
//! **User-space keybindings** ([GNU Readline] in raw mode):
//! - `Ctrl+W` — Delete previous word (requires word boundary understanding)
//! - `Alt+B` / `Alt+F` — Move cursor by word
//! - `Tab` — Command/filename completion
//! - `Ctrl+R` — Reverse history search
//!
//! **How [GNU Readline] Bridges Both Worlds**
//!
//! When you run [Bash], it immediately switches the terminal to non-canonical
//! mode (raw mode). However, the [GNU Readline] library is clever:
//!
//! 1. It queries the kernel's settings for special characters (like `Ctrl+C` for `SIGINT`
//!    or `Ctrl+U` for `VKILL`)
//! 2. It sets up its own keybindings to mirror these kernel defaults
//!
//! This is why `Ctrl+U` still works in [Bash] even though the terminal is in raw mode:
//! [GNU Readline] intercepts it and executes its internal `backward-kill-line` function.
//! [GNU Readline]'s version is actually smarter — it correctly handles the case where
//! your cursor is in the middle of a line, which the kernel's primitive `VKILL` couldn't
//! handle well.
//!
//! **Shells Without [GNU Readline]: Fish and Nushell**
//!
//! Not all shells use [GNU Readline]. [Fish] and [Nushell] implement their own
//! line editors from scratch:
//!
//! - **[Fish]** has a built-in "reader" component (rewritten in Rust as of 2024) — a
//!   custom, tightly-integrated line editor that provides similar keybindings to [GNU
//!   Readline] but with additional features like syntax highlighting and autosuggestions
//!   as you type.
//!
//! - **[Nushell]** uses [Reedline], a standalone Rust crate they created. Unlike Fish's
//!   internal reader, Reedline is a reusable library you can use in your own projects.
//!
//! These shells still operate in raw mode — they just don't delegate to
//! [GNU Readline]. Instead, they read raw bytes from the terminal and implement all
//! line-editing logic themselves. This is exactly what TUI applications do.
//!
//! **Implications for TUI Developers**
//!
//! When your application enables raw mode, it becomes responsible for **all**
//! keybinding handling. The kernel no longer processes `Ctrl+U` or even
//! `Ctrl+C` (unless you explicitly leave signal handling enabled via the
//! `isig` [termios] flag). This is why TUI frameworks typically include their
//! own line-editing functionality — just like Fish and Nushell do.
//!
//! ### The `stty` Command
//!
//! The `stty` (set terminal type) command is a Unix utility for inspecting and
//! modifying terminal settings. It provides a command-line interface to the
//! same **termios** settings that this module manipulates programmatically.
//!
//! `stty` lets you control this line discipline.
//!
//! **View current settings:**
//! ```bash
//! stty -a          # All settings (input/output flags, control chars, etc.)
//! stty size        # Terminal dimensions (rows columns)
//! stty speed       # Baud rate
//! stty -g          # Machine-readable format (for save/restore)
//! ```
//!
//! **Raw mode** (disable all processing):
//! ```bash
//! stty raw         # Passes input directly to program, no buffering
//! stty -raw        # Or: stty cooked (restore normal mode)
//! ```
//!
//! **Echo control:**
//! ```bash
//! stty -echo       # Don't display typed characters (useful for passwords)
//! stty echo        # Restore echo
//! ```
//!
//! **Special characters:**
//! ```bash
//! stty intr ^X     # Change interrupt from Ctrl+C to Ctrl+X
//! stty erase ^H    # Set backspace character
//! ```
//!
//! **Common flags:**
//! - `echo`/`-echo`: Enable/disable character echo
//! - `icanon`/`-icanon`: Enable/disable canonical (line-buffered) mode
//! - `isig`/`-isig`: Enable/disable signal generation (Ctrl+C, etc.)
//! - `raw`/`cooked`: Shorthand for multiple flags at once
//!
//! **Example: capturing raw keypresses:**
//! ```bash
//! # Save current settings, go raw, capture one byte, restore
//! old_stty=$(stty -g)
//! stty raw -echo
//! key=$(dd bs=1 count=1 2>/dev/null)
//! stty "$old_stty"
//! printf '%s' "$key" | xxd
//! ```
//!
//! **Debug escape sequences:**
//! ```bash
//! # See what bytes a keypress generates:
//! stty raw -echo; cat -v; stty cooked echo
//! # Press keys, then Ctrl+C to exit
//! # Left arrow shows: `^[[D` (ESC [ D)
//! ```
//!
//! ### Connection to This Module
//!
//! The [`enable_raw_mode`] and [`disable_raw_mode`] functions use the same underlying
//! mechanism as `stty` — the POSIX [termios] API. On Unix systems, this module calls
//! `tcgetattr()` and `tcsetattr()` (via the [rustix] crate) to manipulate the same
//! terminal flags that `stty` controls. For details on the [termios] struct fields
//! (`c_iflag`, `c_lflag`, etc.) and why we use [rustix], see our [Unix implementation's
//! termios section].
//!
//! Understanding `stty` helps when debugging terminal behavior — if something isn't
//! working, you can use `stty -a` to inspect the current terminal state and verify that
//! raw mode is properly enabled or disabled.
//!
//! ### See Also
//!
//! - [`crate::pty`] — Uses the [`portable_pty` crate] to create pseudo-terminals (PTYs)
//!   for spawning child processes. While this module configures raw mode on your
//!   *current* terminal, the PTY module creates *new* pseudo-terminals for child
//!   processes. Both deal with the same underlying TTY abstraction: the PTY module
//!   creates the terminal pair, while raw mode configures how the line discipline
//!   processes input.
//!
//! ## Platform Support
//!
//! Backend dispatch is based on [`TERMINAL_LIB_BACKEND`]:
//!
//! - **Linux** ([`DirectToAnsi`]): Uses [rustix]'s safe termios API (see [`raw_mode_unix`])
//! - **macOS/Windows** ([`Crossterm`]): Uses [`crossterm::terminal`] functions
//!
//! [`TERMINAL_LIB_BACKEND`]: crate::TERMINAL_LIB_BACKEND
//! [`DirectToAnsi`]: crate::TerminalLibBackend::DirectToAnsi
//! [`Crossterm`]: crate::TerminalLibBackend::Crossterm
//!
//! ## Usage Example
//!
//! The recommended way to use raw mode is with the [`RawModeGuard`]:
//!
//! ```no_run
//! use r3bl_tui::RawModeGuard;
//!
//! {
//!     let _guard = RawModeGuard::new().expect("Failed to enable raw mode");
//!     // Terminal is now in raw mode
//!     // ... process ANSI escape sequences ...
//! } // Raw mode automatically disabled when guard is dropped
//! ```
//!
//! Alternatively, you can manually control raw mode:
//!
//! ```no_run
//! use r3bl_tui::{enable_raw_mode, disable_raw_mode};
//!
//! enable_raw_mode().expect("Failed to enable raw mode");
//! // ... process input ...
//! disable_raw_mode().expect("Failed to disable raw mode");
//! ```
//!
//! [Alacritty]: https://alacritty.org/
//! [Bash]: https://www.gnu.org/software/bash/
//! [Fish]: https://fishshell.com/docs/current/interactive.html
//! [GNOME Terminal]: https://help.gnome.org/users/gnome-terminal/stable/
//! [GNU Readline]: https://tiswww.case.edu/php/chet/readline/rltop.html
//! [Nushell]: https://www.nushell.sh/
//! [PTY]: crate::pty
//! [Reedline]: https://github.com/nushell/reedline
//! [Terminator]: https://gnome-terminator.org/
//! [Unix implementation's termios section]: mod@crate::core::ansi::terminal_raw_mode::raw_mode_unix#the-termios-interface
//! [WezTerm]: https://wezfurlong.org/wezterm/
//! [`portable_pty` crate]: https://docs.rs/portable-pty
//! [iTerm2]: https://iterm2.com/
//! [rustix]: https://docs.rs/rustix
//! [termios]: https://man7.org/linux/man-pages/man3/termios.3.html

// Private modules (hide internal structure).
mod raw_mode_core;

// Public for docs and tests, private otherwise.
#[cfg(all(unix, any(test, doc)))]
pub mod raw_mode_unix;
#[cfg(all(unix, not(any(test, doc))))]
mod raw_mode_unix;

#[cfg(windows)]
mod raw_mode_windows;

// Re-export the public API (flat, ergonomic surface).
pub use raw_mode_core::*;

// Conditional re-export for automated integration tests (Unix only).
#[cfg(all(unix, any(test, doc)))]
pub mod integration_tests;

// Conditional re-export for manual validation tests (Unix only).
#[cfg(all(unix, any(test, doc)))]
pub mod validation_tests;
