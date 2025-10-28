// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # Rendering Pipeline Architecture
//!
//! Here's the flow:
//!
//! ```text
//! App -> Component -> `RenderOpsIR` -> `RenderPipeline` (to `OffscreenBuffer`) -> `RenderOpsOutput` -> Terminal
//! ```
//!
//! ```text
//! ┌───────────────────────────────────────┐
//! │ Application/Component Layer           │
//! │ (Generates RenderOpsIR with clipping) │
//! └────────────────┬──────────────────────┘
//!                  │
//! ┌────────────────▼───────────────────────────┐
//! │ RenderPipeline                             │
//! │ (Collects & organizes RenderOps by ZOrder) │
//! └────────────────┬───────────────────────────┘
//!                  │
//! ┌────────────────▼─────────────────────────┐
//! │ Compositor                               │
//! │ (Renders RenderOpsIR to OffscreenBuffer) │
//! └────────────────┬─────────────────────────┘
//!                  │
//! ┌────────────────▼────────────────────────────────┐
//! │ Backend Converter Layer                         │
//! │ (Render OffscreenBuffer to RenderOpsOutput;     │
//! │  handle diff calculation for selective redraw)  │
//! │ - OffscreenBufferPaint trait implementation     │
//! │ - Converts PixelChars to styled text operations │
//! └────────────────┬────────────────────────────────┘
//!                  │
//! ┌────────────────▼──────────────────────────┐
//! │ Backend Executor                          │
//! │ (Execute RenderOps via Crossterm)         │
//! │ - RenderOpPaint trait (Crossterm impl)    │
//! │ - Cursor movement, colors, text painting  │
//! │ - Raw mode management & terminal flushing │
//! └────────────────┬──────────────────────────┘
//!                  │
//! ┌────────────────▼───────────────────┐
//! │ Terminal Output                    │
//! │ (Rendered content visible to user) │
//! └────────────────────────────────────┘
//! ```
//!
//! ## Module Map
//!
//! **Each module below has a "You are here" breadcrumb showing its place in this flow.**
//!
//! ### Core Data Types (Cross-Stage)
//! - [`render_op`] - `RenderOpIR`, `RenderOpOutput`, `RenderOpCommon`,
//!   `RenderOpsLocalData`
//!
//! ### Pipeline Stages
//! - [`render_pipeline`] - Collects & organizes `RenderOps` by Z-order
//! - [`compositor_render_ops_to_ofs_buf`] - Renders `RenderOpsIR` to `OffscreenBuffer`
//! - [`offscreen_buffer`] - Virtual terminal buffer (2D grid of styled `PixelChars`)
//! - [`offscreen_buffer_paint_impl`] - Converts buffer → optimized operations
//! - [`paint_render_op_impl`] - Executes operations via Crossterm
//!
//! ### Supporting Modules
//! - [`offscreen_buffer_pool`] - Buffer pooling for efficiency
//! - [`z_order`] - Z-order layer management
//! - [`raw_mode`] - Terminal raw mode setup/teardown
//! - [`mod@paint`] - Text painting utilities
//! - [`direct_to_ansi`] - Direct ANSI escape sequence generation
//!
//! # Background information on terminals PTY, TTY, VT100, ANSI, ASCII
//!
//! crossterm:
//! - docs: <https://docs.rs/crossterm/latest/crossterm/index.html>
//! - Raw mode: <https://docs.rs/crossterm/0.23.2/crossterm/terminal/index.html#raw-mode>
//! - Event Poll vs block: <https://github.com/crossterm-rs/crossterm/wiki/Upgrade-from-0.13-to-0.14#115-event-polling>
//! - Async event read eg: <https://github.com/crossterm-rs/crossterm/blob/master/examples/event-stream-tokio.rs>
//! - Async event read src: <https://github.com/crossterm-rs/crossterm/blob/master/src/event/stream.rs#L23>
//!
//! terminal:
//! - Video: <https://youtu.be/Q-te_UBzzjo?t=849>
//! - Raw mode: <https://en.wikipedia.org/wiki/POSIX_terminal_interface#Non-canonical_mode_processing>
//! - Canonical mode: <https://en.wikipedia.org/wiki/POSIX_terminal_interface#Canonical_mode_processing>
//! - Control characters & escape codes:
//!   - ANSI escape codes: <https://en.wikipedia.org/wiki/ANSI_escape_code>
//!     - Windows support: <https://en.wikipedia.org/wiki/ANSI_escape_code#DOS,_OS/2,_and_Windows>
//!     - Colors: <https://en.wikipedia.org/wiki/ANSI_escape_code#Colors>
//!   - ASCII control chars: <https://www.asciitable.com/>
//!   - VT100 Control codes: <https://vt100.net/docs/vt100-ug/chapter3.html#ED>
//! - ANSI (8-bit) vs ASCII (7-bit): <http://www.differencebetween.net/technology/web-applications/difference-between-ansi-and-ascii/>
//! - Windows Terminal (bash): <https://www.makeuseof.com/windows-terminal-vs-powershell/>
//!
//! Examples of TUI / CLI editor:
//! - reedline
//!   - repo: <https://github.com/nushell/reedline>
//!   - live stream videos: <https://www.youtube.com/playlist?list=PLP2yfE2-FXdQw0I6O4YdIX_mzBeF5TDdv>
//! - kilo
//!   - blog: <http://antirez.com/news/108>
//!   - repo (C code): <https://github.com/antirez/kilo>
//! - Sodium:
//!   - repo: <https://github.com/redox-os/sodium>
//!
//! [`offscreen_buffer_paint_impl`]: mod@crossterm_backend::offscreen_buffer_paint_impl
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
