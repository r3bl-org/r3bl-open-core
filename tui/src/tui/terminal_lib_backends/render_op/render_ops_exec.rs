// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{LockedOutputDevice, Size};

/// Trait for render operation collections that can be executed.
///
/// This trait enforces that operations can only be executed after passing through
/// the compositor. Implemented only by [`RenderOpOutputVec`] to prevent bypassing
/// this critical processing stage.
///
/// See [`crate::render_op`] module documentation for shared architectural patterns
/// and the rendering pipeline overview.
///
/// # Semantic Boundary
///
/// This trait enforces a critical semantic boundary in the rendering pipeline. It is
/// implemented only by [`RenderOpOutputVec`] to enforce that operations can only be
/// executed after passing through the compositor. [`RenderOpIRVec`] cannot be executed
/// directly.
///
/// This prevents bypassing the compositor, which is essential for:
/// - Handling text clipping to terminal width
/// - Managing Unicode and emoji display widths
/// - Applying style information correctly
/// - Avoiding redundant terminal commands
///
/// # Architectural Guarantee
///
/// By implementing this trait **ONLY** for [`RenderOpOutputVec`], the type system
/// prevents any attempt to bypass the Compositor:
///
/// ```text
/// RenderOpIR (does NOT implement trait)
///   ↓ (Compositor required)
/// RenderOpOutput (implements ExecutableRenderOps)
///   ↓ (execute_all() only available here)
/// Terminal
/// ```
///
/// # Why This Matters
///
/// The Compositor is critical for:
/// - Handling text clipping to terminal width
/// - Managing Unicode and emoji display widths
/// - Applying style information correctly
/// - Avoiding redundant terminal commands
///
/// If IR were executed directly, these guarantees would be violated.
///
/// # Example
///
/// ```no_run
/// # use r3bl_tui::{RenderOpsExec, RenderOpOutputVec, Size};
/// # fn example(ops: &RenderOpOutputVec) {
/// // Only RenderOpOutputVec implements this trait
/// ops.execute_all(&mut false, Size::default(), todo!(), false);
/// # }
/// ```
///
/// If you try with `RenderOpIRVec`, it won't compile:
/// ```compile_fail
/// # use r3bl_tui::{RenderOpsExec, RenderOpIRVec, Size};
/// # fn example(ops: &RenderOpIRVec) {
/// ops.execute_all(&mut false, Size::default(), todo!(), false); // ❌ Compile error!
/// # }
/// ```
///
/// [`RenderOpOutputVec`]: crate::RenderOpOutputVec
/// [`RenderOpIRVec`]: crate::RenderOpIRVec
/// [`RenderOpOutput`]: crate::RenderOpOutput
/// [`RenderOpIR`]: crate::RenderOpIR
pub trait RenderOpsExec {
    /// Executes all render operations in the collection sequentially.
    ///
    /// This is the **ONLY** way to execute operations. The type system ensures
    /// operations must flow through the proper pipeline:
    /// ```text
    /// RenderOpIR → Compositor → RenderOpOutput → execute_all()
    /// ```
    ///
    /// # Parameters
    /// - `skip_flush`: Mutable reference to control flush behavior
    /// - `window_size`: Current terminal window dimensions
    /// - `locked_output_device`: Locked terminal output for thread-safe writing
    /// - `is_mock`: Whether this is a mock execution for testing
    fn execute_all(
        &self,
        skip_flush: &mut bool,
        window_size: Size,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    );
}
