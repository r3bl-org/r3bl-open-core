// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::RenderOpsLocalData;
use crate::{LockedOutputDevice, RenderOpOutput, Size};

/// Trait for executing individual [`RenderOpOutput`] operations on a terminal.
///
/// This trait is the bridge between the rendering pipeline and terminal libraries.
/// Each backend (Crossterm, DirectAnsi, etc.) implements this trait to translate
/// [`RenderOpOutput`] operations into backend-specific terminal commands.
///
/// # You Are Here
///
/// ```text
/// [S1: App/Component] → [S2: Pipeline] → [S3: Compositor] →
/// [S4: Backend Converter] → [S5: Backend Executor] ← YOU ARE HERE
/// [S6: Terminal]
/// ```
///
/// - **Input**: [`RenderOpOutput`] operations (from backend converter)
/// - **Output**: Terminal commands via backend-specific implementation
/// - **Role**: Define how different backends execute individual render operations
///
/// See [`crate::render_op`] module documentation for shared architectural patterns
/// and the rendering pipeline overview.
///
/// # Purpose
///
/// Defines a common interface for executing [`RenderOpOutput`] operations
/// across different terminal libraries (`crossterm`, `termion`, etc.) and [`DirectAnsi`]
/// module.
///
/// Rather than having each backend handle entire operation collections, this trait
/// allows **per-operation execution**, enabling:
/// - Consistent state tracking across backends via [`RenderOpsLocalData`]
/// - Optimization of redundant commands (skip if cursor/color unchanged)
/// - Flexible backend routing without duplicating logic
///
/// # How It Works
///
/// The rendering pipeline flows through these stages:
///
/// ```text
/// RenderOpOutputVec (collection of operations)
///     ↓
/// For each RenderOpOutput in the collection:
///     ↓
/// Route to backend implementation
///     ↓
/// Backend's RenderOpPaint::paint() method called
///     ↓
/// Operation executed with shared RenderOpsLocalData
///     ↓
/// Final flush() when done
/// ```
///
/// # Multiple Implementations
///
/// Different backends implement this trait independently:
/// - **Crossterm**: Uses crossterm library to queue ANSI commands
/// - **DirectAnsi**: Generates ANSI sequences directly
/// - **Termion**: (future) Uses termion library
///
/// Each implementation handles its backend's specific command format while
/// maintaining the same semantic behavior.
///
/// # Example Usage Pattern
///
/// ```text
/// // In backend converter (e.g., OffscreenBufferPaintImplCrossterm):
/// for render_op_output in &render_ops_collection {
///     let mut painter = CrosstermPainter::new();
///     painter.paint(
///         &mut skip_flush,
///         render_op_output,
///         window_size,
///         &mut local_data,  // Shared state for optimization
///         locked_output,
///         is_mock,
///     );
/// }
/// ```
///
/// # Design Philosophy
///
/// Rather than passing entire collections to backends, we pass individual operations
/// to allow:
/// 1. **Shared state tracking** via [`RenderOpsLocalData`] across all operations
/// 2. **Per-operation optimization** (skip redundant color/position commands)
/// 3. **Consistent behavior** across different terminal library backends
/// 4. **Easy addition** of new backends without changing core pipeline
///
/// # Implementations
///
/// - `PaintRenderOpImplCrossterm` - Crossterm backend
/// - `RenderOpImplDirectAnsi` - DirectAnsi backend
///
/// [`RenderOpOutput`]: crate::RenderOpOutput
/// [`RenderOpsLocalData`]: crate::RenderOpsLocalData
/// [`DirectAnsi`]: crate::terminal_lib_backends::direct_ansi
pub trait RenderOpPaint {
    /// Execute a single render operation on the terminal.
    ///
    /// # Parameters
    ///
    /// - `skip_flush`: Mutable reference controlling whether to flush output
    ///   - If `true`, the backend should skip flushing (another operation will do it)
    ///   - If `false`, normal flush behavior applies
    ///
    /// - `render_op`: The specific output operation to execute
    ///   - Can be a common operation (cursor movement, color changes)
    ///   - Or a paint operation (styled text to terminal)
    ///
    /// - `window_size`: Current terminal window dimensions
    ///   - Used for bounds checking and clamping
    ///   - Prevents off-screen rendering
    ///
    /// - `render_local_data`: Shared mutable state for optimization
    ///   - Tracks current cursor position and colors
    ///   - Allows skipping redundant ANSI commands
    ///   - Modified by this method to reflect new state
    ///
    /// - `locked_output_device`: Thread-safe access to terminal output
    ///   - Backend queues commands to this output device
    ///   - Operations are buffered, not immediately flushed
    ///
    /// - `is_mock`: Testing flag
    ///   - If `true`, backend may skip actual terminal I/O
    ///   - Used for unit tests and benchmarks
    ///
    /// # Behavior
    ///
    /// This method should:
    /// 1. Extract the operation type and parameters
    /// 2. Check if operation is redundant (in `render_local_data`) and skip if so
    /// 3. Queue appropriate terminal command(s) to `locked_output_device`
    /// 4. Update `render_local_data` to reflect the new state
    /// 5. NOT flush (flushing is coordinated at a higher level)
    fn paint(
        &mut self,
        skip_flush: &mut bool,
        render_op: &RenderOpOutput,
        window_size: Size,
        render_local_data: &mut RenderOpsLocalData,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    );
}
