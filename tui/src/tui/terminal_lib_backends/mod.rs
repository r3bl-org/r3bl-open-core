// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # Rendering Pipeline Architecture
//!
//! **Source of Truth:** This module documents the complete 6-stage rendering pipeline.
//! Each stage has its own module with "You Are Here" breadcrumbs. Use the stage reference
//! table below to understand data flow and find relevant implementations.
//!
//! ## Quick Pipeline Overview
//!
//! ```text
//! App -> Component -> `RenderOpsIR` -> `RenderPipeline` (to `OffscreenBuffer`) -> `RenderOpsOutput` -> Terminal
//! ```
//!
//! ## Visual Pipeline Flow
//!
//! ```text
//! ┌───────────────────────────────────────┐
//! │ Stage 1: Application/Component Layer  │
//! │ (Generates RenderOpsIR with clipping) │
//! └────────────────┬──────────────────────┘
//!                  │
//! ┌────────────────▼───────────────────────────┐
//! │ Stage 2: RenderPipeline                    │
//! │ (Collects & organizes RenderOps by ZOrder) │
//! └────────────────┬───────────────────────────┘
//!                  │
//! ┌────────────────▼─────────────────────────┐
//! │ Stage 3: Compositor                      │
//! │ (Renders RenderOpsIR to OffscreenBuffer) │
//! └────────────────┬─────────────────────────┘
//!                  │
//! ┌────────────────▼────────────────────────────────┐
//! │ Stage 4: Backend Converter Layer                │
//! │ (Render OffscreenBuffer to RenderOpsOutput;     │
//! │  handle diff calculation for selective redraw)  │
//! │ - OffscreenBufferPaint trait implementation     │
//! │ - Converts PixelChars to styled text operations │
//! └────────────────┬────────────────────────────────┘
//!                  │
//! ┌────────────────▼──────────────────────────┐
//! │ Stage 5: Backend Executor                 │
//! │ (Execute RenderOps via Crossterm)         │
//! │ - RenderOpPaint trait (Crossterm impl)    │
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
//! ## Stage Reference: IDE Navigation Guide
//!
//! Use this table to navigate to specific pipeline stages. Each stage has a module
//! with "You Are Here" breadcrumbs to help orient yourself.
//!
//! | Stage                          | What It Does                                             | Key Types                                 | Module                                           |
//! | ------------------------------ | -------------------------------------------------------- | ----------------------------------------- | ------------------------------------------------ |
//! | **Stage 1: App/Component**     | Components generate IR operations with clipping metadata | [`RenderOpIR`], [`RenderOpIRVec`]         | [`render_op::render_op_ir`]                      |
//! | **Stage 2: Pipeline**          | Organizes operations by Z-order into a render queue      | [`RenderPipeline`], [`ZOrder`]            | [`render_pipeline`]                              |
//! | **Stage 3: Compositor**        | Executes IR operations, writes styled pixels to buffer   | `OffscreenBuffer`, `PixelChar`            | [`compositor_render_ops_to_ofs_buf`]             |
//! | **Stage 4: Backend Converter** | Compares buffers, generates optimized output operations  | [`RenderOpOutput`], [`RenderOpOutputVec`] | [`offscreen_buffer::paint_impl`] (shared)        |
//! | **Stage 5: Backend Executor**  | Translates operations to terminal escape sequences       | [`RenderOpPaint`] trait                   | `crossterm_backend::paint_render_op_impl`        |
//! | **Stage 6: Terminal**          | User-visible rendered content                            | Terminal emulator                         | (external)                                       |
//!
//! ## Architecture: Shared Stage 4 Across Backends
//!
//! **Key Principle**: Stage 4 (Backend Converter) is **shared** between all terminal
//! backends (Crossterm and DirectToAnsi). The code in [`offscreen_buffer::paint_impl`]
//! implements [`OffscreenBufferPaint`] which reads the [`OffscreenBuffer`] and generates
//! [`RenderOpOutputVec`] operations.
//!
//! **Why is this shared?** Stage 4 is a buffer operation, not a backend-specific
//! operation:
//! - **Input**: [`OffscreenBuffer`] (2D grid of styled characters from Stage 3)
//! - **Processing**: Compares consecutive frames for diff-based selective redraw
//! - **Output**: [`RenderOpOutputVec`] (abstract rendering operations)
//!
//! **Stage 5 is backend-specific**:
//! - **Crossterm**: [`RenderOpPaint`] trait implementation in [`crossterm_backend`]
//!   translates operations to Crossterm API calls
//! - **DirectToAnsi**: [`RenderOpPaint`] trait implementation in [`direct_to_ansi`]
//!   generates raw ANSI escape sequences
//!
//! Both backends use the same Stage 4 converter, with the selection made at compile-time
//! via the [`TERMINAL_LIB_BACKEND`] constant.
//!
//! ## Data Flow Across Stages
//!
//! - **Input → Stage 1**: User code, component state
//! - **Stage 1 → Stage 2**: `RenderOpIRVec` (IR ops with clipping)
//! - **Stage 2 → Stage 3**: Organized `RenderOpIRVec` by `ZOrder`
//! - **Stage 3 → Stage 4**: Complete `OffscreenBuffer` (2D grid)
//! - **Stage 4 → Stage 5**: `RenderOpOutputVec` (optimized ops, already clipped)
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
//! **Each module below has a "You are here" breadcrumb showing its place in this flow.**
//!
//! ### Core Data Types (Cross-Stage)
//! - [`render_op`] - `RenderOpIR`, `RenderOpOutput`, `RenderOpCommon`,
//!   `RenderOpsLocalData`, type safety details
//!
//! ### Pipeline Stages
//! - [`render_pipeline`] - **(Stage 2)** Collects & organizes `RenderOps` by Z-order
//! - [`compositor_render_ops_to_ofs_buf`] - **(Stage 3)** Renders `RenderOpsIR` to
//!   `OffscreenBuffer`
//! - [`offscreen_buffer`] - Virtual terminal buffer (2D grid of styled `PixelChars`)
//!   - [`offscreen_buffer::paint_impl`] - **(Stage 4: Shared)** Converts buffer →
//!     optimized operations (used by both Crossterm and DirectToAnsi)
//! - [`crossterm_backend::paint_render_op_impl`] - **(Stage 5: Crossterm Executor)**
//!   Executes operations via Crossterm
//!
//! ### Supporting Modules
//! - [`offscreen_buffer_pool`] - Buffer pooling for efficiency
//! - [`z_order`] - Z-order layer management
//! - [`raw_mode`] - Terminal raw mode setup/teardown
//! - [`mod@paint`] - Text painting utilities
//! - [`direct_to_ansi`] - **(Stage 5 Alternative)** Direct ANSI escape sequence
//!   generation (Linux only)
//!
//! [`paint_impl`]: mod@offscreen_buffer::paint_impl
//! [`paint_render_op_impl`]: mod@crossterm_backend::paint_render_op_impl

/// Terminal library backend selection for the TUI system.
///
/// R3BL TUI supports multiple terminal manipulation libraries, allowing users to choose
/// the backend that best fits their needs. Currently supported backends include:
///
/// - **Crossterm**: Cross-platform terminal library (default and recommended)
/// - **`DirectToAnsi`**: Pure Rust ANSI sequence generation without external dependencies
///
/// # Example
///
/// ```rust
/// use r3bl_tui::TerminalLibBackend;
///
/// let backend = TerminalLibBackend::Crossterm;
/// match backend {
///     TerminalLibBackend::Crossterm => println!("Using Crossterm backend"),
///     TerminalLibBackend::DirectToAnsi => println!("Using DirectToAnsi backend"),
/// }
/// ```
#[derive(Debug)]
pub enum TerminalLibBackend {
    /// Cross-platform terminal library (default).
    Crossterm,
    /// Pure Rust ANSI sequence generation.
    DirectToAnsi,
}

/// The default terminal library backend for this platform.
///
/// On **Linux**, [`DirectToAnsi`] is selected for pure Rust ANSI sequence generation
/// without external dependencies.
///
/// # Platform Selection
///
/// R3BL TUI uses platform-specific backends:
/// - **Linux**: [`DirectToAnsi`] (pure Rust async I/O)
/// - **macOS/Windows**: Crossterm (cross-platform compatibility)
///
/// # Performance
///
/// [`DirectToAnsi`] achieves ~18% better performance than Crossterm on Linux through:
/// - Stack-allocated number formatting (eliminates heap allocations)
/// - `SmallVec[16]` for render operations (+0.47%)
/// - `StyleUSSpan[16]` for styled text spans (+~5.0%)
///
/// Benchmarked using 8-second continuous workload with 999Hz sampling and scripted
/// input (see `script_lib.fish::run_example_with_flamegraph_profiling_perf_fold`).
///
/// [`DirectToAnsi`]: variant@TerminalLibBackend::DirectToAnsi
#[cfg(target_os = "linux")]
pub const TERMINAL_LIB_BACKEND: TerminalLibBackend = TerminalLibBackend::DirectToAnsi;

/// The default terminal library backend for this platform.
///
/// On **macOS/Windows**, Crossterm is selected for its mature cross-platform
/// support and compatibility across different terminal emulators.
///
/// # Platform Selection
///
/// R3BL TUI uses platform-specific backends:
/// - **Linux**: [`DirectToAnsi`] (pure Rust async I/O)
/// - **macOS/Windows**: Crossterm (cross-platform compatibility)
///
/// [`DirectToAnsi`]: variant@TerminalLibBackend::DirectToAnsi
#[cfg(not(target_os = "linux"))]
pub const TERMINAL_LIB_BACKEND: TerminalLibBackend = TerminalLibBackend::Crossterm;

// Attach source files.
pub mod compositor_render_ops_to_ofs_buf;
pub mod crossterm_backend;
pub mod direct_to_ansi;
pub mod offscreen_buffer;
pub mod offscreen_buffer_pool;
pub mod paint;
pub mod raw_mode;
pub mod render_op;
pub mod render_pipeline;
pub mod render_tui_styled_texts;
pub mod z_order;

// Re-export.
pub use compositor_render_ops_to_ofs_buf::*;
pub use crossterm_backend::*;
pub use direct_to_ansi::*;
pub use offscreen_buffer::*;
pub use offscreen_buffer_pool::*;
pub use paint::*;
pub use raw_mode::*;
pub use render_op::*;
pub use render_pipeline::*;
pub use render_tui_styled_texts::*;
pub use z_order::*;

// Tests.
#[cfg(test)]
mod test_render_pipeline;

// Benchmarks.
#[cfg(test)]
mod pixel_char_bench;
#[cfg(test)]
mod render_op_bench;
