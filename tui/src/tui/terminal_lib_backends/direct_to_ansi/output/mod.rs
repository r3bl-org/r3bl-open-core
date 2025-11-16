// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Output rendering module for `DirectToAnsi` backend.
//!
//! This module contains the output-specific functionality for the `DirectToAnsi` backend,
//! including ANSI sequence generation, render operation painting, and text rendering.
//!
//! ## Architecture
//!
//! The module consists of:
//! 1. [`AnsiSequenceGenerator`]: Generates raw ANSI escape sequence bytes
//! 2. [`RenderOpPaintImplDirectToAnsi`]: Implements [`RenderOpPaint`] trait for executing
//!    render operations: [`RenderOpOutput`] and [`RenderOpCommon`]
//! 3. [`PixelCharRenderer`]: Converts styled text to ANSI with smart attribute diffing
//! 4. [`RenderToAnsi`]: Trait for rendering offscreen buffers to ANSI
//!
//! [`RenderOpCommon`]: crate::RenderOpCommon
//! [`AnsiSequenceGenerator`]: crate::AnsiSequenceGenerator
//! [`RenderOpPaintImplDirectToAnsi`]: crate::RenderOpPaintImplDirectToAnsi
//! [`PixelCharRenderer`]: crate::PixelCharRenderer
//! [`RenderToAnsi`]: crate::RenderToAnsi
//! [`RenderOpPaint`]: crate::RenderOpPaint
//! [`RenderOpOutput`]: crate::RenderOpOutput

// Attach.
// Conditionally public for documentation links.
#[cfg(any(test, doc))]
pub mod direct_to_ansi_paint_render_op_impl;
#[cfg(not(any(test, doc)))]
mod direct_to_ansi_paint_render_op_impl;

mod pixel_char_renderer;
mod render_to_ansi;

// Re-exports - flatten the public API
pub use direct_to_ansi_paint_render_op_impl::*;
pub use pixel_char_renderer::*;
pub use render_to_ansi::*;

// Tests
#[cfg(test)]
mod tests;
