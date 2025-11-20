// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # Rendering Pipeline Architecture
//!
//! **Source of Truth:** This module documents the complete 6-stage rendering pipeline.
//! Each stage has its own module with "You Are Here" breadcrumbs. Use the stage reference
//! table below to understand data flow and find relevant implementations.
//!
//! <div class="warning">
//!
//! **Architecture Context:** This pipeline implements **Path 1 (Composed Component
//! Pipeline)** from the dual rendering paths architecture. For the high-level overview
//! of both rendering paths (Path 1: full TUI vs Path 2: simple CLI), see the
//! [dual rendering paths].
//!
//! </div>
//!
//! ## Quick Pipeline Overview
//!
//! ```text
//! Application Code
//!       ↓
//! Component (generates RenderOpIR)
//!       ↓
//! RenderPipeline (organizes by ZOrder)
//!       ↓
//! OffscreenBuffer (2D grid of styled pixels)
//!       ↓
//! RenderOpOutputVec (backend-independent ops)
//!       ↓
//! Terminal Backend (Crossterm or DirectToAnsi)
//!       ↓
//! Terminal Display
//! ```
//!
//! ## Visual Pipeline Flow
//!
//! ```text
//! ┌───────────────────────────────────────┐
//! │ Stage 1: Application/Component Layer  │
//! │ (Generates RenderOpIR with clipping)  │
//! └────────────────┬──────────────────────┘
//!                  │
//! ┌────────────────▼───────────────────────────┐
//! │ Stage 2: RenderPipeline                    │
//! │ (Collects & organizes RenderOpIR by ZOrder)│
//! └────────────────┬───────────────────────────┘
//!                  │
//! ┌────────────────▼─────────────────────────┐
//! │ Stage 3: Compositor                      │
//! │ (Renders RenderOpIR to OffscreenBuffer)  │
//! └────────────────┬─────────────────────────┘
//!                  │
//! ┌────────────────▼────────────────────────────────┐
//! │ Stage 4: Backend Converter Layer                │
//! │ (Render OffscreenBuffer to RenderOpOutput;      │
//! │  handle diff calculation for selective redraw)  │
//! │ - OffscreenBufferPaint trait implementation     │
//! │ - Converts PixelChars to styled text operations │
//! └────────────────┬────────────────────────────────┘
//!                  │
//! ┌────────────────▼──────────────────────────┐
//! │ Stage 5: Backend Executor                 │
//! │ (Execute RenderOpOutput via backend)      │
//! │ - RenderOpPaint trait implementations     │
//! │ - Crossterm or DirectToAnsi               │
//! │ - Cursor movement, colors, text painting  │
//! │ - Raw mode management & terminal flushing │
//! └────────────────┬──────────────────────────┘
//!                  │
//! ┌────────────────▼───────────────────┐
//! │ Stage 6: Terminal Output           │
//! │ (Rendered content visible to user) │
//! └────────────────────────────────────┘
//! ```
//!
//! ## Stage Reference
//!
//! Use this table to navigate to specific pipeline stages. Each stage has a module
//! with "You Are Here" breadcrumbs to help orient yourself.
//!
//! | Stage                          | What It Does                                             | Key Types                                 | Module                                                                        |
//! | ------------------------------ | -------------------------------------------------------- | ----------------------------------------- | ----------------------------------------------------------------------------- |
//! | **Stage 1: App/Component**     | Components generate IR operations with clipping metadata | [`RenderOpIR`], [`RenderOpIRVec`]         | [`render_op::render_op_ir`]                                                   |
//! | **Stage 2: Pipeline**          | Organizes operations by Z-order into a render queue      | [`RenderPipeline`], [`ZOrder`]            | [`render_pipeline`]                                                           |
//! | **Stage 3: Compositor**        | Executes IR operations, writes styled pixels to buffer   | [`OffscreenBuffer`], [`PixelChar`]        | [`compositor_render_ops_to_ofs_buf`]                                          |
//! | **Stage 4: Backend Converter** | Compares buffers, generates optimized output operations  | [`RenderOpOutput`], [`RenderOpOutputVec`] | [`offscreen_buffer::paint_impl`] (shared)                                     |
//! | **Stage 5: Backend Executor**  | Translates operations to terminal escape sequences       | [`RenderOpPaint`] trait                   | [`crossterm_paint_render_op_impl`] or [`direct_to_ansi_paint_render_op_impl`] |
//! | **Stage 6: Terminal**          | User-visible rendered content                            | Terminal emulator                         | (external)                                                                    |
//!
//! ## Architecture: Shared Stages (1-4) vs Backend-Specific Stage (5)
//!
//! **Key Principle**: Stages 1-4 are **shared** across all terminal backends (Crossterm
//! and `DirectToAnsi`). Only Stage 5 (Backend Executor) is backend-specific.
//!
//! ### Shared Stages (1-4)
//! - **Stage 1**: Components generate [`RenderOpIR`] operations
//! - **Stage 2**: [`RenderPipeline`] organizes operations by [`ZOrder`]
//! - **Stage 3**: Compositor renders to [`OffscreenBuffer`] (2D grid of styled pixels)
//! - **Stage 4**: Backend Converter transforms [`OffscreenBuffer`] →
//!   [`RenderOpOutputVec`]
//!   - Implementation: [`offscreen_buffer::paint_impl`]
//!   - Compares consecutive frames for diff-based selective redraw
//!   - Generates abstract rendering operations (backend-independent)
//!
//! ### Backend-Specific Stage (5)
//! **Why Stage 5 is different**: Each backend has its own [`RenderOpPaint`] trait
//! implementation that translates abstract operations to terminal-specific commands:
//!
//! - **Crossterm**: Implementation in [`crossterm_backend`]
//!   - Translates operations to Crossterm API calls
//! - **`DirectToAnsi`**: Implementation in [`direct_to_ansi`]
//!   - Generates raw ANSI escape sequences
//!
//! The backend selection is made at compile-time via the [`TERMINAL_LIB_BACKEND`]
//! constant, ensuring both backends use the same Stage 1-4 pipeline.
//!
//! ## Data Flow Across Stages
//!
//! - **Input → Stage 1**: User code, component state
//! - **Stage 1 → Stage 2**: [`RenderOpIRVec`] (IR ops with clipping)
//! - **Stage 2 → Stage 3**: Organized [`RenderOpIRVec`] by [`ZOrder`]
//! - **Stage 3 → Stage 4**: Complete [`OffscreenBuffer`] (2D grid)
//! - **Stage 4 → Stage 5**: [`RenderOpOutputVec`] (optimized ops, already clipped)
//! - **Stage 5 → Stage 6**: ANSI escape sequences written to terminal
//!
//! ## Type Safety & Semantic Boundaries
//!
//! The pipeline enforces strict separation through types:
//! - **IR Operations** ([`RenderOpIR`]): Used by components, require clipping
//! - **Output Operations** ([`RenderOpOutput`]): Used by backends, already clipped
//! - **Execution Barrier**: Only [`RenderOpOutputVec`] can be executed via
//!   [`RenderOpsExec`] trait, preventing IR operations from bypassing the compositor
//!
//! See [`render_op`] module for architectural details and type definitions.
//!
//! ## Module Map
//!
//! When navigating to individual modules below, you'll find a "You are here" comment at
//! the top showing which stage(s) the module implements. This helps you quickly
//! understand where the module fits in the 6-stage rendering pipeline.
//!
//! ### Core Data Types (Cross-Stage)
//! - [`render_op`] - `RenderOpIR`, `RenderOpOutput`, `RenderOpCommon`,
//!   `RenderOpsLocalData`, type safety details
//!
//! ### Pipeline Stages
//! - [`render_pipeline`] - **(Stage 2)** Collects & organizes `RenderOpIR` by Z-order
//! - [`compositor_render_ops_to_ofs_buf`] - **(Stage 3)** Renders `RenderOpsIR` to
//!   `OffscreenBuffer`
//! - [`offscreen_buffer`] - Virtual terminal buffer (2D grid of styled `PixelChars`)
//!   - [`offscreen_buffer::paint_impl`] - **(Stage 4: Shared)** Converts buffer →
//!     optimized operations (used by both Crossterm and `DirectToAnsi`)
//! - [`crossterm_backend::crossterm_paint_render_op_impl`] - **(Stage 5: Crossterm
//!   Executor)** Executes operations via Crossterm
//!
//! ### Supporting Modules
//! - [`offscreen_buffer_pool`] - Buffer pooling for efficiency
//! - [`z_order`] - Z-order layer management
//! - [`raw_mode`] - Terminal raw mode setup/teardown
//! - [`mod@paint`] - Text painting utilities
//! - [`direct_to_ansi`] - **(Stage 5 Alternative)** Direct ANSI escape sequence
//!   generation (Linux only)
//!
//! [`OffscreenBufferPaint`]: trait@offscreen_buffer::OffscreenBufferPaint
//! [`OffscreenBuffer`]: struct@offscreen_buffer::OffscreenBuffer
//! [`PixelChar`]: enum@offscreen_buffer::PixelChar
//! [`RenderOpCommon`]: enum@render_op::RenderOpCommon
//! [`RenderOpIRVec`]: struct@render_op::RenderOpIRVec
//! [`RenderOpIR`]: enum@render_op::RenderOpIR
//! [`RenderOpOutputVec`]: struct@render_op::RenderOpOutputVec
//! [`RenderOpOutput`]: enum@render_op::RenderOpOutput
//! [`RenderOpPaint`]: trait@render_op::RenderOpPaint
//! [`RenderOpsExec`]: trait@render_op::RenderOpsExec
//! [`RenderPipeline`]: struct@render_pipeline::RenderPipeline
//! [`ZOrder`]: enum@z_order::ZOrder
//! [`crossterm_paint_render_op_impl`]: mod@crossterm_backend::crossterm_paint_render_op_impl
//! [`direct_to_ansi_paint_render_op_impl`]: mod@direct_to_ansi::output::direct_to_ansi_paint_render_op_impl
//! [`paint_impl`]: mod@offscreen_buffer::paint_impl
//! [`paint_render_op_impl`]: mod@crossterm_backend::crossterm_paint_render_op_impl
//! [dual rendering paths]: mod@crate#dual-rendering-paths

// Skip rustfmt for rest of file.
// https://stackoverflow.com/a/75910283/2085356
#![cfg_attr(rustfmt, rustfmt_skip)]

/**************************/
/** Attach source files. **/
/**************************/
// Private mod.
mod backend_selection;

// Private in production, public for docs/tests (enables rustdoc links to submodules).

#[cfg(any(test, doc))]
pub mod compositor_render_ops_to_ofs_buf;
#[cfg(not(any(test, doc)))]
mod compositor_render_ops_to_ofs_buf;

#[cfg(any(test, doc))]
pub mod crossterm_backend;
#[cfg(not(any(test, doc)))]
mod crossterm_backend;

#[cfg(any(test, doc))]
pub mod direct_to_ansi;
#[cfg(not(any(test, doc)))]
mod direct_to_ansi;

#[cfg(any(test, doc))]
pub mod offscreen_buffer;
#[cfg(not(any(test, doc)))]
mod offscreen_buffer;

#[cfg(any(test, doc))]
pub mod offscreen_buffer_pool;
#[cfg(not(any(test, doc)))]
mod offscreen_buffer_pool;

#[cfg(any(test, doc))]
pub mod paint;
#[cfg(not(any(test, doc)))]
mod paint;

#[cfg(any(test, doc))]
pub mod raw_mode;
#[cfg(not(any(test, doc)))]
mod raw_mode;

#[cfg(any(test, doc))]
pub mod render_op;
#[cfg(not(any(test, doc)))]
mod render_op;

#[cfg(any(test, doc))]
pub mod render_pipeline;
#[cfg(not(any(test, doc)))]
mod render_pipeline;

#[cfg(any(test, doc))]
pub mod render_tui_styled_texts;
#[cfg(not(any(test, doc)))]
mod render_tui_styled_texts;

#[cfg(any(test, doc))]
pub mod z_order;
#[cfg(not(any(test, doc)))]
mod z_order;

/***********************************************/
/** Re-export shared components (Stages 1-5). **/
/***********************************************/
pub use compositor_render_ops_to_ofs_buf::*;
pub use offscreen_buffer::*;
pub use offscreen_buffer_pool::*;
pub use paint::*;
pub use raw_mode::*;
pub use render_op::*;
pub use render_pipeline::*;
pub use render_tui_styled_texts::*;
pub use z_order::*;
pub use backend_selection::*;
// Both backends are compiled; selection happens at compile time via TERMINAL_LIB_BACKEND.
pub use crossterm_backend::*;
pub use direct_to_ansi::*;

/**********/
/* Tests. */
/**********/
#[cfg(test)]
mod test_render_pipeline;

/***************/
/* Benchmarks. */
/***************/
#[cfg(test)]
mod pixel_char_bench;
#[cfg(test)]
mod render_op_bench;
