// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words kqueue filedescriptor terminfo undercurls

//! # [`DirectToAnsi`] Terminal Backend
//!
//! Pure-Rust [`ANSI`] sequence generation without crossterm dependencies.
//!
//! # You Are Here: **Stage 5 Alternative** (Backend Executor)
//!
//! ```text
//! [Stage 1: App/Component]
//!   ↓
//! [Stage 2: Pipeline]
//!   ↓
//! [Stage 3: Compositor]
//!   ↓
//! [Stage 4: Backend Converter]
//!   ↓
//! [Stage 5: Backend Executor (DirectToAnsi)] ← YOU ARE HERE
//!   ↓
//! [Stage 6: Terminal]
//! ```
//!
//! This module provides a complete **terminal rendering backend** that generates [`ANSI`]
//! escape sequences directly. It's designed to work seamlessly with the rendering
//! operation abstraction layer.
//!
//! ## Navigation
//! - **See complete architecture**: [`terminal_lib_backends` mod docs] (source of truth)
//! - **Previous stage**: [`ofs_buf::paint_impl` mod docs] (Stage 4: Backend
//!   Converter - shared by both Crossterm and `DirectToAnsi`)
//! - **Alternative Stage 5**: [`crossterm_backend::crossterm_paint_render_op_impl` mod
//!   docs] (Crossterm-based executor)
//! - **Next stage**: Terminal output (Stage 6)
//!
//! <div class="warning">
//!
//! **For the complete rendering architecture**, see [`terminal_lib_backends` mod docs]
//! module documentation (this is the authoritative source of truth).
//!
//! </div>
//!
//! ## What This Module Does
//!
//! [`DirectToAnsi`] is the **Stage 5 Backend Executor** that translates render operations
//! into actual terminal control sequences. Unlike Crossterm (which uses FFI bindings to
//! [`libc`] on UNIX and [`winapi`] on Windows), [`DirectToAnsi`] generates pure [`ANSI`]
//! escape sequences in Rust.
//!
//! - **Input**: [`RenderOpOutputVec`] from the Backend Converter
//! - **Output**: [`ANSI`] escape sequences written to terminal
//! - **Dependencies**: None (pure Rust)
//!
//! ## Architecture Note: Bypassing [`terminfo`]
//!
//! Unlike traditional terminal libraries (such as [`ncurses`]), [`DirectToAnsi`] **does
//! not** query the OS-level [`terminfo`] database to determine terminal capabilities or
//! escape sequences.
//!
//! Instead, it takes the modern approach: hardcoding standard [`VT-100`] and [`ANSI`]
//! escape sequences. Because almost all modern terminal emulators ([`WezTerm`],
//! [`Alacritty`], [`GNOME Terminal`], etc.) support standard [`ANSI`] natively, bypassing
//! [`terminfo`] provides several massive architectural advantages:
//!
//! 1. **Zero Deployment Dependencies**: The application remains a standalone binary.
//!    There is no need to install a custom `.terminfo` file on the target system (which
//!    requires root access).
//! 2. **Cross-OS Determinism**: [`terminfo`] databases vary wildly between OSes.
//!    Hardcoding ensures identical byte output across macOS, Linux, and FreeBSD.
//! 3. **SSH Robustness**: TUI applications will render perfectly over SSH even when the
//!    user's specific terminal [`terminfo`] file (e.g., [`wezterm.terminfo`]) is missing
//!    on the remote server.
//! 4. **Modern Capabilities**: Immediately leverages modern features (like 24-bit
//!    Truecolor or "undercurls") without waiting for OS databases to adopt them.
//!
//! > **Note on Child Processes**: While the renderer *bypasses* [`terminfo`] for output,
//! > the [`pty` mod docs: Masquerading] section explains how child processes use
//! > [`terminfo`] masquerading to know how to draw to the TUI.
//!
//! # Architecture
//!
//! The module consists of:
//! 1. [`ansi_output`]: Generates raw [`ANSI`] escape sequence bytes
//! 2. [`RenderOpPaintImplDirectToAnsi`]: Implements [`RenderOpPaint`] trait for executing
//!    render operations: [`RenderOpOutput`] and [`RenderOpCommon`]
//! 3. [`PixelCharRenderer`]: Converts styled text to [`ANSI`] with smart attribute
//!    diffing
//! 4. [`RenderToAnsi`]: Trait for rendering offscreen buffers to [`ANSI`]
//!
//! # Platform Support
//!
//! | Component                    | Linux   | macOS   | Windows   |
//! | ---------------------------- | ------- | ------- | --------- |
//! | Output ([`ANSI`] generation) | ✅      | ✅      | ✅        |
//! | Input (terminal reading)     | ✅      | ❌      | ❌        |
//!
//! The **output** side works on all platforms (pure [`ANSI`] sequence generation).
//!
//! The **input** side is Linux-only due to macOS [`kqueue`] limitations with
//! [`PTY`]/[`tty`] polling. See the [`input`] module documentation (Linux only) for
//! details and potential future macOS support via [`filedescriptor::poll()`].
//!
//! # Testing Strategy
//!
//! Integration tests are organized by component:
//!
//! - **Output**: [`output::direct_to_ansi_output_integration_tests`] —
//!   [`StdoutMock`]-based [`ANSI`] sequence verification (cross-platform)
//! - **Input**: [`input::integration_tests_stub`] — documentation module pointing to
//!   [`PTY`]-based parser tests in
//!   [`vt_100_terminal_input_parser::vt_100_parser_integration_tests`] (Linux-only).
//!
//! [`Alacritty`]: https://alacritty.org/
//! [`ansi_output`]: crate::ansi_output
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`compositor_render_ops_to_ofs_buf` mod docs]:
//!     mod@crate::compositor_render_ops_to_ofs_buf
//! [`crossterm_backend::crossterm_paint_render_op_impl` mod docs]:
//!     mod@crate::crossterm_backend::crossterm_paint_render_op_impl
//! [`DirectToAnsi`]: self
//! [`filedescriptor::poll()`]:
//!     https://docs.rs/filedescriptor/latest/filedescriptor/fn.poll.html
//! [`GNOME Terminal`]: https://help.gnome.org/users/gnome-terminal/stable/
//! [`input::integration_tests_stub`]:
//!     mod@crate::terminal_lib_backends::direct_to_ansi::input::integration_tests_stub
//! [`kqueue`]: https://man.freebsd.org/cgi/man.cgi?query=kqueue&sektion=2
//! [`libc`]: https://crates.io/crates/libc
//! [`ncurses`]: https://en.wikipedia.org/wiki/Ncurses
//! [`ofs_buf::paint_impl` mod docs]: mod@crate::ofs_buf::paint_impl
//! [`output::direct_to_ansi_output_integration_tests`]:
//!     mod@crate::terminal_lib_backends::direct_to_ansi::output::direct_to_ansi_output_integration_tests
//! [`PixelCharRenderer`]: crate::PixelCharRenderer
//! [`pty` mod docs: Masquerading]:
//!     mod@crate::core::pty#terminal-emulation--terminfo-masquerading
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`render_op_ir` mod docs]: mod@crate::render_op::render_op_ir
//! [`RenderOpCommon`]: crate::RenderOpCommon
//! [`RenderOpIR`]: crate::RenderOpIR
//! [`RenderOpIRVec`]: crate::RenderOpIRVec
//! [`RenderOpOutput`]: crate::RenderOpOutput
//! [`RenderOpOutputVec`]: crate::RenderOpOutputVec
//! [`RenderOpPaint`]: crate::RenderOpPaint
//! [`RenderOpPaintImplDirectToAnsi`]: crate::RenderOpPaintImplDirectToAnsi
//! [`RenderToAnsi`]: crate::RenderToAnsi
//! [`StdoutMock`]: crate::StdoutMock
//! [`terminal_lib_backends` mod docs]: mod@crate::tui::terminal_lib_backends
//! [`terminfo`]: https://en.wikipedia.org/wiki/Terminfo
//! [`tty`]: https://man7.org/linux/man-pages/man4/tty.4.html
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [`vt_100_terminal_input_parser::vt_100_parser_integration_tests`]:
//!     mod@crate::vt_100_terminal_input_parser::vt_100_parser_integration_tests
//! [`wezterm.terminfo`]: https://wezfurlong.org/wezterm/faq.html
//! [`WezTerm`]: https://wezfurlong.org/wezterm/
//! [`winapi`]: https://crates.io/crates/winapi
//! [rendering pipeline overview]:
//!     mod@crate::terminal_lib_backends#rendering-pipeline-architecture

#![rustfmt::skip]

// Private inner modules (hide implementation structure).
// Conditionally public for documentation links.
mod debug;

#[cfg(any(test, doc))]
pub mod output;
#[cfg(not(any(test, doc)))]
mod output;

// Input handling is Linux-only because macOS kqueue doesn't support PTY/tty polling.
// See `input/mod.rs` docs for technical details and potential future macOS support.
// On macOS/Windows, use Crossterm backend instead (set via TERMINAL_LIB_BACKEND).
// Doc builds are allowed on Unix platforms (macOS/Linux) where the dependencies exist.
// Windows doc builds exclude this module since signal_hook/mio::unix are unavailable.
#[cfg(any(all(unix, doc), all(target_os = "linux", test)))]
pub mod input;
#[cfg(all(target_os = "linux", not(any(test, doc))))]
mod input;

// Public re-exports (flat API surface).
pub use debug::*;
pub use output::*;
#[cfg(any(target_os = "linux", all(unix, doc)))]
pub use input::*;
