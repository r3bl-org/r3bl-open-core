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
//! ## TTY, Line Discipline, and `stty`
//!
//! ### Historical Context
//!
//! The term "TTY" comes from "teletypewriter" — physical terminals from the
//! 1960s-70s that communicated with mainframes over serial lines. Modern
//! terminal emulators (like GNOME Terminal, iTerm2, or Alacritty) still use
//! this abstraction: they create a **pseudo-terminal (PTY)** that behaves like
//! those old hardware devices.
//!
//! ### The Line Discipline
//!
//! Between your terminal and the programs reading input sits the **line
//! discipline** — a kernel-level layer that:
//!
//! - **Buffers input** line-by-line (so you can edit before pressing Enter)
//! - **Interprets special characters** (Ctrl+C sends SIGINT, Ctrl+D sends EOF)
//! - **Echoes characters** back to the screen as you type
//! - **Processes editing keys** (backspace, arrow keys for line editing)
//!
//! This is "cooked" mode. In "raw" mode, the line discipline is bypassed —
//! bytes flow directly from the terminal to your program without any kernel
//! processing. This is what TUI applications need to capture every keystroke,
//! including escape sequences.
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
//! # Left arrow shows: ^[[D (ESC [ D)
//! ```
//!
//! ### Connection to This Module
//!
//! The [`enable_raw_mode`] and [`disable_raw_mode`] functions use the same
//! underlying mechanism as `stty` — the POSIX **termios** API. On Unix systems,
//! this module calls `tcgetattr()` and `tcsetattr()` (via the rustix crate) to
//! manipulate the same terminal flags that `stty` controls.
//!
//! Understanding `stty` helps when debugging terminal behavior — if something
//! isn't working, you can use `stty -a` to inspect the current terminal state
//! and verify that raw mode is properly enabled or disabled.
//!
//! ### See Also
//!
//! - [`crate::pty`] — Uses the `portable_pty` crate to create pseudo-terminals
//!   (PTYs) for spawning child processes. While this module configures raw mode
//!   on your *current* terminal, the PTY module creates *new* pseudo-terminals
//!   for child processes. Both deal with the same underlying TTY abstraction:
//!   the PTY module creates the terminal pair, while raw mode configures how
//!   the line discipline processes input.
//!
//! ## Platform Support
//!
//! - **Unix/Linux/macOS**: Uses rustix's safe termios API
//! - **Windows**: Not yet implemented (TODO)
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

// Private modules (hide internal structure).
mod raw_mode_core;

#[cfg(unix)]
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
