// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # [`DirectToAnsi`] Terminal Backend
//!
//! Pure-Rust ANSI sequence generation without crossterm dependencies.
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
//! This module provides a complete **terminal rendering backend** that generates ANSI
//! escape sequences directly. It's designed to work seamlessly with the rendering
//! operation abstraction layer.
//!
//! ## Navigation
//! - **See complete architecture**: [`terminal_lib_backends` mod docs] (source of truth)
//! - **Previous stage**: [`offscreen_buffer::paint_impl` mod docs] (Stage 4: Backend Converter - shared
//!   by both Crossterm and `DirectToAnsi`)
//! - **Alternative Stage 5**: [`crossterm_backend::crossterm_paint_render_op_impl` mod docs] (Crossterm-based executor)
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
//! `libc` on UNIX and `winapi` on Windows), [`DirectToAnsi`] generates pure ANSI escape
//! sequences in Rust.
//!
//! **Input**: [`RenderOpOutputVec`] from the Backend Converter
//! **Output**: ANSI escape sequences written to terminal
//! **Dependencies**: None (pure Rust)
//!
//! # Architecture
//!
//! The module consists of:
//! 1. [`AnsiSequenceGenerator`]: Generates raw ANSI escape sequence bytes
//! 2. [`RenderOpPaintImplDirectToAnsi`]: Implements [`RenderOpPaint`] trait for executing
//!    render operations: [`RenderOpOutput`] and [`RenderOpCommon`]
//! 3. [`PixelCharRenderer`]: Converts styled text to ANSI with smart attribute diffing
//! 4. [`RenderToAnsi`]: Trait for rendering offscreen buffers to ANSI
//!
//! # Platform Support
//!
//! | Component | Linux | macOS | Windows |
//! |-----------|-------|-------|---------|
//! | Output (ANSI generation) | ✅ | ✅ | ✅ |
//! | Input (terminal reading) | ✅ | ❌ | ❌ |
//!
//! The **output** side works on all platforms (pure ANSI sequence generation).
//!
//! The **input** side is Linux-only due to macOS `kqueue` limitations with PTY/tty
//! polling. See the `input` module documentation (Linux only) for details and potential
//! future macOS support via [`filedescriptor::poll()`].
//!
//! [`filedescriptor::poll()`]: https://docs.rs/filedescriptor/latest/filedescriptor/fn.poll.html
//!
//! [`AnsiSequenceGenerator`]: crate::AnsiSequenceGenerator
//! [`DirectToAnsi`]: self
//! [`PixelCharRenderer`]: crate::PixelCharRenderer
//! [`RenderOpCommon`]: crate::RenderOpCommon
//! [`RenderOpIRVec`]: crate::RenderOpIRVec
//! [`RenderOpIR`]: crate::RenderOpIR
//! [`RenderOpOutputVec`]: crate::RenderOpOutputVec
//! [`RenderOpOutput`]: crate::RenderOpOutput
//! [`RenderOpPaintImplDirectToAnsi`]: crate::RenderOpPaintImplDirectToAnsi
//! [`RenderOpPaint`]: crate::RenderOpPaint
//! [`RenderToAnsi`]: crate::RenderToAnsi
//! [`compositor_render_ops_to_ofs_buf` mod docs]: mod@crate::tui::terminal_lib_backends::compositor_render_ops_to_ofs_buf
//! [`crossterm_backend::crossterm_paint_render_op_impl` mod docs]: mod@crate::tui::terminal_lib_backends::crossterm_backend::crossterm_paint_render_op_impl
//! [`offscreen_buffer::paint_impl` mod docs]: mod@crate::tui::terminal_lib_backends::offscreen_buffer::paint_impl
//! [`render_op_ir` mod docs]: mod@crate::tui::terminal_lib_backends::render_op::render_op_ir
//! [`terminal_lib_backends` mod docs]: mod@crate::tui::terminal_lib_backends
//! [rendering pipeline overview]: mod@crate::terminal_lib_backends#rendering-pipeline-architecture

// Skip rustfmt for rest of file to preserve manual alignment.
// https://stackoverflow.com/a/75910283/2085356
#![cfg_attr(rustfmt, rustfmt_skip)]

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
#[cfg(all(target_os = "linux", any(test, doc)))]
pub mod input;
#[cfg(all(target_os = "linux", not(any(test, doc))))]
mod input;

// Public re-exports (flat API surface).
pub use debug::*;
pub use output::*;
#[cfg(target_os = "linux")]
pub use input::*;

// Tests.
#[cfg(test)]
mod integration_tests;
