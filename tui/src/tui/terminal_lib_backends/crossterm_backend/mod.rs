// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # Backend Implementation: Crossterm Terminal Library
//!
//! This module provides the **Crossterm-specific backend implementation** for the
//! rendering pipeline, containing Stage 5 executor code.
//!
//! ```text
//! [Stage 1: App/Component]
//!   ↓
//! [Stage 2: Pipeline]
//!   ↓
//! [Stage 3: Compositor]
//!   ↓
//! [Stage 4: Backend Converter] ← Shared (in offscreen_buffer/paint_impl)
//!   ↓
//! [Stage 5: Backend Executor] ← Crossterm implementation here
//!   ↓
//! [Stage 6: Terminal]
//! ```
//!
//! > **For the complete 6-stage rendering pipeline with visual diagrams and stage
//! > reference table**, see the [rendering pipeline overview].
//!
//! [rendering pipeline overview]: mod@crate::terminal_lib_backends#rendering-pipeline-architecture
//!
//! ## Module Organization
//!
//! This module contains the **Crossterm-specific Stage 5 backend executor**
//! implementation.
//!
//! ### Stage 4: Backend Converter (Shared)
//! - **Not in this module** - Stage 4 is shared across all backends
//! - See [`offscreen_buffer::paint_impl`] for the `OffscreenBufferPaintImplCrossterm`
//!   converter
//! - Converts [`OffscreenBuffer`] → [`RenderOpOutputVec`] (shared by both Crossterm and
//!   `DirectToAnsi`)
//!
//! ### Stage 5: Backend Executor (`crossterm_paint_render_op_impl`)
//! - **Implemented in this module** - Crossterm-specific execution
//! - Implements [`RenderOpPaint`] trait
//! - Executes [`RenderOpOutputVec`] operations via Crossterm API
//! - Manages terminal modes (raw mode, cursor visibility, mouse tracking)
//! - Uses [`RenderOpsLocalData`] for state tracking (avoid redundant commands)
//! - Handles colors, cursor movement, and text output
//!
//! [`offscreen_buffer::paint_impl`]: mod@crate::tui::terminal_lib_backends::offscreen_buffer::paint_impl
//!
//! [`OffscreenBuffer`]: crate::OffscreenBuffer
//! [`RenderOpOutputVec`]: crate::RenderOpOutputVec
//! [`OffscreenBufferPaint`]: crate::OffscreenBufferPaint
//! [`RenderOpPaint`]: crate::RenderOpPaint
//! [`RenderOpsLocalData`]: crate::RenderOpsLocalData

// Attach.
mod debug;
mod input_device_impl;

#[cfg(any(test, doc))]
pub mod crossterm_paint_render_op_impl;
#[cfg(not(any(test, doc)))]
mod crossterm_paint_render_op_impl;

// Re-export.
pub use crossterm_paint_render_op_impl::*;
pub use debug::*;
pub use input_device_impl::*;
