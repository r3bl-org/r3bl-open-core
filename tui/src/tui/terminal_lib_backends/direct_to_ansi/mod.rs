// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # [`DirectAnsi`] Terminal Backend
//!
//! Pure-Rust ANSI sequence generation without crossterm dependencies.
//!
//! # You Are Here
//!
//! ```text
//! [S1: App/Component] → [S2: Pipeline] → [S3: Compositor] →
//! [S4: Backend Converter] → [S5: Backend Executor (DirectAnsi)] ← YOU ARE HERE
//! [S6: Terminal]
//! ```
//!
//! This module provides a complete **terminal rendering backend** that generates ANSI
//! escape sequences directly. It's designed to work seamlessly with the rendering
//! operation abstraction layer.
//!
//! > **For the complete rendering architecture**, see [`crate::render_op`] module
//! > documentation.
//!
//! ## What This Module Does
//!
//! [`DirectAnsi`] is the **Stage 5 Backend Executor** that translates render operations
//! into actual terminal control sequences. Unlike Crossterm (which uses FFI bindings to
//! `libc` on UNIX and `winapi` on Windows), [`DirectAnsi`] generates pure ANSI escape
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
//! 2. [`RenderOpPaintImplDirectAnsi`]: Implements [`RenderOpPaint`] trait for executing
//!    render operations: [`RenderOpOutput`] and [`RenderOpCommon`]
//! 3. [`PixelCharRenderer`]: Converts styled text to ANSI with smart attribute diffing
//! 4. [`RenderToAnsi`]: Trait for rendering offscreen buffers to ANSI
//!
//! [`DirectAnsi`]: self
//! [`RenderOpCommon`]: crate::RenderOpCommon
//! [`RenderOpIRVec`]: crate::RenderOpIRVec
//! [`RenderOpOutputVec`]: crate::RenderOpOutputVec
//! [`AnsiSequenceGenerator`]: crate::AnsiSequenceGenerator
//! [`RenderOpPaintImplDirectAnsi`]: crate::RenderOpPaintImplDirectAnsi
//! [`PixelCharRenderer`]: crate::PixelCharRenderer
//! [`RenderToAnsi`]: crate::RenderToAnsi
//! [`RenderOpPaint`]: crate::RenderOpPaint
//! [`RenderOpIR`]: crate::RenderOpIR
//! [`RenderOpOutput`]: crate::RenderOpOutput

// Attach.
mod ansi_sequence_generator;
mod debug;
mod paint_render_op_impl;
mod pixel_char_renderer;
mod render_to_ansi;

// Re-exports
pub use ansi_sequence_generator::*;
pub use debug::*;
pub use paint_render_op_impl::*;
pub use pixel_char_renderer::*;
pub use render_to_ansi::*;

// Tests
#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod tests;
