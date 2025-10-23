// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`DirectAnsi`] Terminal Backend
//!
//! Pure-Rust ANSI sequence generation without crossterm dependencies.
//!
//! This module provides a complete terminal rendering backend that generates ANSI escape
//! sequences directly. It's designed to work seamlessly with the [`RenderOp`] abstraction
//! layer.
//!
//! # Architecture
//!
//! The module consists of:
//! 1. **[`AnsiSequenceGenerator`]**: Generates raw ANSI escape sequence bytes
//! 2. **[`RenderOpImplDirectAnsi`]**: Implements [`PaintRenderOp`] trait for executing
//!    [`RenderOps`]
//! 3. **[`PixelCharRenderer`]**: converts styled text to ANSI
//! 4. **[`RenderToAnsi`]**: trait for rendering to ANSI
//!
//! [`DirectAnsi`]: self
//! [`RenderOp`]: crate::RenderOp
//! [`RenderOps`]: crate::RenderOps
//! [`PaintRenderOp`]: crate::PaintRenderOp
//! [`AnsiSequenceGenerator`]: crate::AnsiSequenceGenerator
//! [`RenderOpImplDirectAnsi`]: crate::RenderOpImplDirectAnsi
//! [`PixelCharRenderer`]: crate::PixelCharRenderer
//! [`RenderToAnsi`]: crate::RenderToAnsi

// Attach.
mod ansi_sequence_generator;
mod pixel_char_renderer;
mod render_op_impl_direct_ansi;
mod render_to_ansi;

// Re-exports
pub use ansi_sequence_generator::AnsiSequenceGenerator;
pub use pixel_char_renderer::*;
pub use render_op_impl_direct_ansi::RenderOpImplDirectAnsi;
pub use render_to_ansi::RenderToAnsi;

// Tests
#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod tests;
