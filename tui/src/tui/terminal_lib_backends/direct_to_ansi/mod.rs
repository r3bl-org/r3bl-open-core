// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

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
//! # Terminal Protocols & Capabilities Hub
//!
//! [`DirectToAnsi`] acts as the **Stage 5 I/O Execution Layer**, driving the pure
//! Rust [`Sans-IO`] protocol parsers and generators located in `core/ansi/` and
//! `core/osc/`.
//!
//! Rather than duplicating low-level protocol specifications across backends, this module
//! serves as the central navigation hub linking to the authoritative protocol
//! documentation across the codebase.
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────────────────────────────┐
//! │                         CORE SANS-IO PROTOCOL LAYER                               │
//! │                                                                                   │
//! │  • Progressive Enhancement (Kitty CSI u)      ──► vt_100_terminal_input_parser    │
//! │  • Inbound OSC Framing & Alt+]                ──► terminal_events::osc / scanner  │
//! │  • Outbound OSC Sequence Generation           ──► core::osc::osc_codes            │
//! │  • Hybrid Clipboard (OSC 52 + Paste)          ──► tui::editor::clipboard          │
//! │  • Child Process PTY Controlled Interception  ──► core::pty / OscBuffer           │
//! └──────────────────────────────────────┬────────────────────────────────────────────┘
//!                                        │ Driven by
//! ┌──────────────────────────────────────▼────────────────────────────────────────────┐
//! │                 STAGE 5 BACKEND EXECUTOR (DirectToAnsi)                           │
//! │                                                                                   │
//! │  • direct_to_ansi::input   ──► Linux non-blocking stdin & mio epoll poller        │
//! │  • direct_to_ansi::output  ──► High-performance ANSI stream writer to stdout      │
//! └───────────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Protocol Navigation Matrix
//!
//! 1. **[`Kitty`] Keyboard Protocol (`CSI u`)**:
//!    - **Direction**: 🛬 Inbound (`stdin`) & Outbound (`stdout`)
//!    - **Source of Truth**: [`vt_100_terminal_input_parser`]
//!    - **Highlights**:
//!      - Zero-latency progressive enhancement (`CSI > 1 u`).
//!      - Disambiguates `Shift+Enter`, `Ctrl+Tab`, and distinct `Ctrl+I` vs `Tab`.
//!      - See [Legacy vs Kitty Capability Matrix].
//!
//! 2. **[`OSC`] 52 In-Band Clipboard**:
//!    - **Direction**: 🛫 Outbound (`stdout`)
//!    - **Source of Truth**: [`ClipboardService`] & [`Osc52Clipboard`]
//!    - **Highlights**:
//!      - Automatic fallback when desktop display server (`$DISPLAY` / `$WAYLAND_DISPLAY`)
//!        is unset.
//!      - Encodes clipboard text into Base64 payload over SSH.
//!      - Works seamlessly in headless and Docker environments.
//!
//! 3. **[`DEC`] 2004 Bracketed Paste**:
//!    - **Direction**: 🛬 Inbound (`stdin`)
//!    - **Source of Truth**: [`paste_state_machine`] & [`terminal_events`]
//!    - **Highlights**:
//!      - Asynchronous paste handling (`CSI 200 ~` / `CSI 201 ~`).
//!      - Eliminates need for insecure synchronous [`OSC`] 52 queries.
//!
//! 4. **Inbound [`OSC`] Framing & `Alt+]` Disambiguation**:
//!    - **Direction**: 🛬 Inbound (`stdin`)
//!    - **Source of Truth**: [`terminal_events::osc`] & [`osc_scanner`]
//!    - **Highlights**:
//!      - 2-phase lexical scanner distinguishing human `Alt+]` from terminal [`OSC`]
//!        responses.
//!      - Uses [`MaybeMore`] stream availability heuristic for 0ms latency.
//!      - 1 MiB runaway sequence safety drain via [`OscCircuitBreaker`].
//!
//! 5. **Outbound [`OSC`] Codes & Formatting**:
//!    - **Direction**: 🛫 Outbound (`stdout`)
//!    - **Source of Truth**: [`core::osc`] & [`OscController`]
//!    - **Highlights**:
//!      - Type-safe enum builder ([`OscSequence`]).
//!      - Sets window titles (`OSC 0`/`2`), creates clickable hyperlinks (`OSC 8`), and
//!        sends progress notifications (`OSC 9;4`).
//!
//! 6. **[`PTY`] Controlled Child Interception**:
//!    - **Direction**: 🔄 Intermediate ([`PTY`] controlled output)
//!    - **Source of Truth**: [`core::pty`] & [`OscBuffer`]
//!    - **Highlights**:
//!      - Uses `TERM=xterm-256color` masquerading for child processes.
//!      - Background reader scans and extracts cargo/rustup `OSC 9;4` progress updates.
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
//! [`ClipboardService`]: crate::ClipboardService
//! [`core::osc`]: mod@crate::core::osc
//! [`core::pty`]: mod@crate::core::pty
//! [`crossterm_backend::crossterm_paint_render_op_impl` mod docs]: mod@crate::crossterm_backend::crossterm_paint_render_op_impl
//! [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
//! [`DirectToAnsi`]: self
//! [`filedescriptor::poll()`]: https://docs.rs/filedescriptor/latest/filedescriptor/fn.poll.html
//! [`GNOME Terminal`]: https://help.gnome.org/users/gnome-terminal/stable/
//! [`input::integration_tests_stub`]: mod@crate::terminal_lib_backends::direct_to_ansi::input::integration_tests_stub
//! [`Kitty`]: https://sw.kovidgoyal.net/kitty/
//! [`kqueue`]: https://man.freebsd.org/cgi/man.cgi?query=kqueue&sektion=2
//! [`Legacy vs Kitty Capability Matrix`]: mod@crate::core::ansi::vt_100_terminal_input_parser#terminal-input-capability-matrix-legacy-vt-100-vs-kitty-keyboard-protocol
//! [`libc`]: https://crates.io/crates/libc
//! [`MaybeMore`]: crate::core::ansi::vt_100_terminal_input_parser::MaybeMore
//! [`ncurses`]: https://en.wikipedia.org/wiki/Ncurses
//! [`ofs_buf::paint_impl` mod docs]: mod@crate::ofs_buf::paint_impl
//! [`Osc52Clipboard`]: crate::Osc52Clipboard
//! [`osc_scanner`]: mod@crate::core::ansi::vt_100_terminal_input_parser::osc_scanner
//! [`OSC`]: crate::osc_codes::OscSequence
//! [`OscBuffer`]: crate::OscBuffer
//! [`OscCircuitBreaker`]: crate::core::ansi::vt_100_terminal_input_parser::input_byte_stream_to_ir::OscCircuitBreaker
//! [`OscController`]: crate::OscController
//! [`OscSequence`]: crate::osc_codes::OscSequence
//! [`output::direct_to_ansi_output_integration_tests`]: mod@crate::terminal_lib_backends::direct_to_ansi::output::direct_to_ansi_output_integration_tests
//! [`paste_state_machine`]: mod@crate::terminal_lib_backends::direct_to_ansi::input::paste_state_machine
//! [`PixelCharRenderer`]: crate::PixelCharRenderer
//! [`pty` mod docs: Masquerading]: mod@crate::core::pty#terminal-emulation--terminfo-masquerading
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`RenderOpCommon`]: crate::tui::RenderOpCommon
//! [`RenderOpOutput`]: crate::RenderOpOutput
//! [`RenderOpOutputVec`]: crate::tui::RenderOpOutputVec
//! [`RenderOpPaint`]: crate::RenderOpPaint
//! [`RenderOpPaintImplDirectToAnsi`]: crate::RenderOpPaintImplDirectToAnsi
//! [`RenderToAnsi`]: crate::RenderToAnsi
//! [`Sans-IO`]: https://sans-io.readthedocs.io/
//! [`StdoutMock`]: crate::StdoutMock
//! [`terminal_events::osc`]: mod@crate::core::ansi::vt_100_terminal_input_parser::terminal_events::osc
//! [`terminal_events`]: mod@crate::core::ansi::vt_100_terminal_input_parser::terminal_events
//! [`terminal_lib_backends` mod docs]: mod@crate::tui::terminal_lib_backends
//! [`terminfo`]: https://en.wikipedia.org/wiki/Terminfo
//! [`tty`]: https://man7.org/linux/man-pages/man4/tty.4.html
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [`vt_100_terminal_input_parser::vt_100_parser_integration_tests`]: mod@crate::vt_100_terminal_input_parser::vt_100_parser_integration_tests
//! [`vt_100_terminal_input_parser`]: mod@crate::vt_100_terminal_input_parser
//! [`wezterm.terminfo`]: https://wezterm.org/faq.html
//! [`WezTerm`]: https://wezterm.org/
//! [`winapi`]: https://crates.io/crates/winapi

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
// Non-Linux platforms (macOS/Windows) exclude this module across all builds.
#[cfg(all(target_os = "linux", any(test, doc)))]
pub mod input;
#[cfg(all(target_os = "linux", not(any(test, doc))))]
mod input;

// Public re-exports (flat API surface).
pub use debug::*;
pub use output::*;
#[cfg(target_os = "linux")]
pub use input::*;
