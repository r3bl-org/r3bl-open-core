// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # Backend Implementation: Crossterm Terminal Library
//!
//! # You Are Here
//!
//! ```text
//! [STAGE 1: App/Component] → [STAGE 2: Pipeline] → [STAGE 3: Compositor] →
//! [STAGE 4: Backend Converter] ← YOU ARE HERE → [STAGE 5: Backend Executor] ← YOU ARE HERE
//! [STAGE 6: Terminal]
//! ```
//!
//! **Input (Stage 4)**: [`OffscreenBuffer`] (2D grid of styled characters)
//! **Output (Stage 4)**: [`RenderOpOutputVec`] (optimized terminal operations)
//!
//! **Input (Stage 5)**: [`RenderOpOutputVec`] operations to execute
//! **Output (Stage 5)**: Terminal state changes via Crossterm
//!
//! **Role**: Convert high-level rendering data to low-level terminal commands
//!
//! > **For the complete pipeline architecture**, see [`super`] (parent module).
//!
//! ## Module Organization
//!
//! This module contains the **Crossterm-specific backend implementation** with two key
//! stages:
//!
//! ### Stage 4: Backend Converter (`offscreen_buffer_paint_impl`)
//! - Implements [`OffscreenBufferPaint`] trait
//! - Scans the [`OffscreenBuffer`] and generates [`RenderOpOutputVec`]
//! - Computes diffs for selective redraw optimization
//! - Converts 2D pixel grid to optimized text painting operations
//!
//! ### Stage 5: Backend Executor (`paint_render_op_impl`)
//! - Implements [`RenderOpPaint`] trait
//! - Executes [`RenderOpOutputVec`] operations
//! - Translates operations to Crossterm API calls
//! - Manages terminal modes (raw mode, cursor visibility, mouse tracking)
//! - Uses [`RenderOpsLocalData`] for state tracking (avoid redundant commands)
//! - Handles colors, cursor movement, and text output
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
pub mod paint_render_op_impl;
#[cfg(not(any(test, doc)))]
mod paint_render_op_impl;

// Re-export.
pub use debug::*;
pub use input_device_impl::*;
pub use paint_render_op_impl::*;
