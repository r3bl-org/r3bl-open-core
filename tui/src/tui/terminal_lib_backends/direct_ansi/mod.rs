// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`DirectAnsi`] Terminal Backend
//!
//! Pure-Rust ANSI sequence generation without crossterm dependencies.
//!
//! This module provides a complete terminal rendering backend that generates ANSI escape
//! sequences directly. It's designed to work seamlessly with the rendering operation
//! abstraction layer.
//!
//! # Architecture
//!
//! The module consists of:
//! 1. [`AnsiSequenceGenerator`]: Generates raw ANSI escape sequence bytes
//! 2. [`RenderOpImplDirectAnsi`]: Implements [`PaintRenderOp`] trait for executing render
//!    operations: [`RenderOpIR`], [`RenderOpOutput`], and [`RenderOpCommon`]
//! 3. [`PixelCharRenderer`]: converts styled text to ANSI
//! 4. [`RenderToAnsi`]: trait for rendering to ANSI
//!
//! [`DirectAnsi`]: self
//! [`RenderOpCommon`]: crate::RenderOpCommon
//! [`RenderOpIRVec`]: crate::RenderOpIRVec
//! [`RenderOpOutputVec`]: crate::RenderOpOutputVec
//! [`AnsiSequenceGenerator`]: crate::AnsiSequenceGenerator
//! [`RenderOpImplDirectAnsi`]: crate::RenderOpImplDirectAnsi
//! [`PixelCharRenderer`]: crate::PixelCharRenderer
//! [`RenderToAnsi`]: crate::RenderToAnsi
//! [`PaintRenderOp`]: crate::PaintRenderOp
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
