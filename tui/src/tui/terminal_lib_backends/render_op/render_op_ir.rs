// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Intermediate Representation operations for app/component layer.
//!
//! # You Are Here: **Stage 1** (App/Component Layer)
//!
//! ```text
//! [Stage 1: App/Component] ← YOU ARE HERE
//!   ↓
//! [Stage 2: Pipeline]
//!   ↓
//! [Stage 3: Compositor]
//!   ↓
//! [Stage 4: Backend Converter]
//!   ↓
//! [Stage 5: Backend Executor]
//!   ↓
//! [Stage 6: Terminal]
//! ```
//!
//! ## Navigation
//! - **See complete architecture**: [`terminal_lib_backends` mod docs] (source of truth)
//! - **Next stage**: [`render_pipeline` mod docs] (Stage 2: Organization)
//! - **Final stage**: [`compositor_render_ops_to_ofs_buf` mod docs] (Stage 3: Rendering)
//!
//! See [`render_op` mod docs] for shared architectural patterns and the rendering
//! pipeline overview.
//!
//! Components produce [`RenderOpIR`] operations with built-in clipping info.
//! These get processed by the Compositor (Stage 3) to populate the `OffscreenBuffer`.
//!
//! # Type Safety & Semantic Boundary
//!
//! This enum type ensures only IR-appropriate operations are used in component code.
//! Operations like `PaintTextWithAttributes` (which handles clipping) are IR-specific
//! and cannot be accidentally used in backend code.
//!
//! ## Architectural Note
//!
//! `execute_all()` and execution methods do **not** exist on [`RenderOpIR`] or
//! [`RenderOpIRVec`] to enforce a semantic boundary.
//!
//! **Why?** [`RenderOpIR`] operations must **NOT** be executed directly. All operations
//! must flow through the proper pipeline:
//!
//! ```text
//! RenderOpIR → Compositor → RenderOpOutput → Terminal
//! ```
//!
//! Removing these methods prevents IR from bypassing the compositor. The compositor is
//! critical for:
//! - Handling text clipping to terminal width
//! - Managing Unicode and emoji display widths
//! - Applying style information correctly
//!
//! [`compositor_render_ops_to_ofs_buf` mod docs]: mod@crate::tui::terminal_lib_backends::compositor_render_ops_to_ofs_buf
//! [`render_op_ir` mod docs]: mod@crate::tui::terminal_lib_backends::render_op::render_op_ir
//! [`render_op` mod docs]: mod@crate::tui::terminal_lib_backends::render_op
//! [`render_pipeline` mod docs]: mod@crate::tui::terminal_lib_backends::render_pipeline
//! [`terminal_lib_backends` mod docs]: mod@crate::tui::terminal_lib_backends

use super::RenderOpCommon;
use crate::{InlineString, InlineVec, TuiStyle, ok};
use std::{fmt::{Debug, Formatter, Result},
          ops::{AddAssign, Deref, DerefMut}};

/// Intermediate Representation operations for app/component layer.
///
/// These operations are used by components and the app layer to describe
/// high-level rendering operations. They get processed by the compositor
/// to populate the offscreen buffer.
///
/// # Type Safety
///
/// This enum type ensures that only IR-appropriate operations are used
/// in component rendering code. Operations like `PaintTextWithAttributes`
/// (which handles clipping) are IR-specific and cannot be accidentally
/// used in backend code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderOpIR {
    /// Shared operation used identically in IR and Output contexts.
    Common(RenderOpCommon),

    /// Paint text with attributes (handles clipping, Unicode, emoji).
    ///
    /// This operation is used by components to render text with styling.
    /// The compositor is responsible for:
    /// - Clipping text to available terminal width
    /// - Handling Unicode and emoji display width
    /// - Applying styles correctly
    ///
    /// This is the **IR-specific** variant. The backend converter
    /// generates `CompositorNoClipTruncPaintTextWithAttributes` after
    /// clipping has been done by the compositor.
    PaintTextWithAttributes(InlineString, Option<TuiStyle>),
}

/// Collection of IR-level render operations from app/component layer.
///
/// This type wraps `RenderOpIR` values and provides ergonomic collection methods.
/// Used throughout the app/component layer and passed to the compositor.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct RenderOpIRVec {
    pub list: InlineVec<RenderOpIR>,
}

impl RenderOpIRVec {
    /// Create a new empty collection of IR operations.
    #[must_use]
    pub fn new() -> Self {
        Self {
            list: InlineVec::new(),
        }
    }

    /// Add a single operation to the collection.
    pub fn push(&mut self, arg_op: impl Into<RenderOpIR>) {
        self.list.push(arg_op.into());
    }

    /// Add multiple operations to the collection.
    pub fn extend(&mut self, ops: impl IntoIterator<Item = RenderOpIR>) {
        self.list.extend(ops);
    }

    /// Get the number of operations in the collection.
    #[must_use]
    pub fn len(&self) -> usize { self.list.len() }

    /// Check if the collection is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool { self.list.is_empty() }

    /// Iterate over the operations.
    pub fn iter(&self) -> impl Iterator<Item = &RenderOpIR> { self.list.iter() }
}

impl From<RenderOpCommon> for RenderOpIR {
    fn from(op: RenderOpCommon) -> Self { RenderOpIR::Common(op) }
}

impl Deref for RenderOpIRVec {
    type Target = InlineVec<RenderOpIR>;

    fn deref(&self) -> &Self::Target { &self.list }
}

impl DerefMut for RenderOpIRVec {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.list }
}

/// Ergonomic operator for adding a single operation to the collection.
///
/// This allows using the `+=` operator instead of `.push()` for more readable and
/// concise code. The `Into<RenderOpIR>` conversion is automatically applied, so types
/// like `RenderOpCommon` can be used directly.
///
/// # Example
///
/// ```
/// # use r3bl_tui::{RenderOpCommon, RenderOpIRVec, Pos, row, col};
/// let mut render_ops = RenderOpIRVec::new();
///
/// // Using += operator (more ergonomic)
/// render_ops += RenderOpCommon::MoveCursorPositionAbs(Pos::new((row(5), col(10))));
///
/// assert_eq!(render_ops.len(), 1);
/// ```
impl AddAssign<RenderOpIR> for RenderOpIRVec {
    fn add_assign(&mut self, rhs: RenderOpIR) { self.list.push(rhs); }
}

impl AddAssign<RenderOpCommon> for RenderOpIRVec {
    fn add_assign(&mut self, rhs: RenderOpCommon) {
        self.list.push(RenderOpIR::Common(rhs));
    }
}

impl AddAssign<RenderOpIR> for &mut RenderOpIRVec {
    fn add_assign(&mut self, rhs: RenderOpIR) { self.list.push(rhs); }
}

impl AddAssign<RenderOpCommon> for &mut RenderOpIRVec {
    fn add_assign(&mut self, rhs: RenderOpCommon) {
        self.list.push(RenderOpIR::Common(rhs));
    }
}

impl Debug for RenderOpIRVec {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        const DELIM: &str = "\n  - ";

        let mut iter = self.iter();

        // We don't care about the result of this operation.
        f.write_str("RenderOpsIR.len(): ").ok();
        write!(f, "{}", self.list.len()).ok();

        // First line.
        if let Some(first) = iter.next() {
            // We don't care about the result of this operation.
            f.write_str("[").ok();
            write!(f, "{first:?}").ok();
            f.write_str("]").ok();
        }

        // Subsequent lines.
        for item in iter {
            // We don't care about the result of this operation.
            f.write_str(DELIM).ok();
            f.write_str("[").ok();
            write!(f, "{item:?}").ok();
            f.write_str("]").ok();
        }

        ok!()
    }
}

impl From<Vec<RenderOpIR>> for RenderOpIRVec {
    fn from(ops: Vec<RenderOpIR>) -> Self { Self { list: ops.into() } }
}

impl FromIterator<RenderOpIR> for RenderOpIRVec {
    fn from_iter<I: IntoIterator<Item = RenderOpIR>>(iter: I) -> Self {
        Self {
            list: iter.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_assign_single_op_via_render_op_ir() {
        let mut render_ops = RenderOpIRVec::new();
        assert_eq!(render_ops.len(), 0);

        // Add a single RenderOpIR variant
        let op = RenderOpIR::Common(RenderOpCommon::EnterRawMode);
        render_ops += op.clone();

        assert_eq!(render_ops.len(), 1);
        assert_eq!(render_ops[0], op);
    }

    #[test]
    fn test_add_assign_converts_render_op_common_to_ir() {
        let mut render_ops = RenderOpIRVec::new();

        // Add via Into conversion from RenderOpCommon
        let op_common = RenderOpCommon::ExitRawMode;
        render_ops += RenderOpIR::Common(op_common.clone());

        assert_eq!(render_ops.len(), 1);
        match &render_ops[0] {
            RenderOpIR::Common(RenderOpCommon::ExitRawMode) => {
                // Test passed - the operation was converted and stored correctly
            }
            _ => panic!("Expected ExitRawMode"),
        }
    }

    #[test]
    fn test_add_assign_multiple_operations() {
        let mut render_ops = RenderOpIRVec::new();

        // Add multiple operations using += operator
        let op1 = RenderOpCommon::EnterRawMode;
        let op2 = RenderOpCommon::ExitRawMode;
        let op3 = RenderOpCommon::ClearScreen;

        render_ops += RenderOpIR::Common(op1);
        render_ops += RenderOpIR::Common(op2);
        render_ops += RenderOpIR::Common(op3);

        assert_eq!(render_ops.len(), 3);
    }

    #[test]
    fn test_add_assign_vs_push_are_equivalent() {
        let mut render_ops_push = RenderOpIRVec::new();
        let mut render_ops_add_assign = RenderOpIRVec::new();

        let op = RenderOpCommon::ClearScreen;

        // Using push
        render_ops_push.push(op.clone());

        // Using += operator
        render_ops_add_assign += RenderOpIR::Common(op);

        // Both should produce the same result
        assert_eq!(render_ops_push.len(), render_ops_add_assign.len());
        assert_eq!(render_ops_push[0], render_ops_add_assign[0]);
    }

    #[test]
    fn test_add_assign_is_ergonomic() {
        let mut render_ops = RenderOpIRVec::new();

        // This demonstrates the ergonomic improvement over .push()
        render_ops += RenderOpIR::Common(RenderOpCommon::EnterRawMode);

        assert_eq!(render_ops.len(), 1);
    }

    #[test]
    fn test_push_and_add_assign_work_together() {
        let mut render_ops = RenderOpIRVec::new();

        // Mix push() and += operator
        render_ops.push(RenderOpCommon::EnterRawMode);
        render_ops += RenderOpIR::Common(RenderOpCommon::ExitRawMode);
        render_ops.push(RenderOpCommon::ClearScreen);

        assert_eq!(render_ops.len(), 3);
    }

    #[test]
    fn test_add_assign_render_op_common_directly() {
        let mut render_ops = RenderOpIRVec::new();

        // Add RenderOpCommon directly without wrapping in RenderOpIR::Common
        render_ops += RenderOpCommon::EnterRawMode;
        render_ops += RenderOpCommon::ExitRawMode;
        render_ops += RenderOpCommon::ClearScreen;

        assert_eq!(render_ops.len(), 3);

        // Verify the operations were wrapped correctly
        match &render_ops[0] {
            RenderOpIR::Common(RenderOpCommon::EnterRawMode) => {
                // First operation is correct
            }
            _ => panic!("Expected EnterRawMode at index 0"),
        }
    }
}
