// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Terminal output operations for backend/execution layer.
//!
//! # You Are Here
//!
//! ```text
//! [S1: App/Component] → [S2: Pipeline] → [S3: Compositor] →
//! [S4: Backend Converter] → [S5: Backend Executor] ← [S6: Terminal]
//!                              ↓↓↓↓
//!                        RenderOpOutput
//! ```
//!
//! These operations are optimized for terminal execution. They're generated
//! by backend converters (Stage 4) after processing IR and don't require
//! additional clipping or validation.
//!
//! # Type Safety
//!
//! This enum ensures only Output-appropriate operations are used in backend code.
//! Operations like `CompositorNoClipTruncPaintTextWithAttributes` (assumes clipping
//! is already done) cannot be accidentally used in component code.

use super::RenderOpCommon;
use crate::{InlineString, InlineVec, TuiStyle, ok};
use std::{fmt::{Debug, Formatter, Result},
          ops::{AddAssign, Deref, DerefMut}};

/// Terminal output operations for backend/execution layer.
///
/// These operations are optimized for terminal execution. They are generated
/// by backend converters (e.g., `OffscreenBufferPaint`) after processing the IR
/// and don't require additional clipping or validation.
///
/// # Type Safety
///
/// This enum type ensures that only Output-appropriate operations are used
/// in backend code. Operations like `CompositorNoClipTruncPaintTextWithAttributes`
/// (which assumes clipping is already done) cannot be accidentally used in
/// component code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderOpOutput {
    /// Shared operation used identically in IR and Output contexts.
    Common(RenderOpCommon),

    /// Paint text without clipping/truncation (already handled by compositor).
    ///
    /// **Internal use only** - this operation is used by backend converters
    /// after the `OffscreenBuffer` has been fully processed. The compositor has
    /// already handled:
    /// - Clipping text to available width
    /// - Unicode and emoji display width
    /// - Style application
    ///
    /// The backend just needs to paint the result as-is to the terminal.
    CompositorNoClipTruncPaintTextWithAttributes(InlineString, Option<TuiStyle>),
}

/// Collection of terminal output operations for backend rendering.
///
/// This type wraps `RenderOpOutput` values and provides ergonomic collection methods.
/// Used by backend converters and the terminal execution layer.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct RenderOpOutputVec {
    pub list: InlineVec<RenderOpOutput>,
}

impl RenderOpOutputVec {
    /// Create a new empty collection of output operations.
    #[must_use]
    pub fn new() -> Self {
        Self {
            list: InlineVec::new(),
        }
    }

    /// Add a single operation to the collection.
    pub fn push(&mut self, arg_op: impl Into<RenderOpOutput>) {
        self.list.push(arg_op.into());
    }

    /// Add multiple operations to the collection.
    pub fn extend(&mut self, ops: impl IntoIterator<Item = RenderOpOutput>) {
        self.list.extend(ops);
    }

    /// Get the number of operations in the collection.
    #[must_use]
    pub fn len(&self) -> usize { self.list.len() }

    /// Check if the collection is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool { self.list.is_empty() }

    /// Iterate over the operations.
    pub fn iter(&self) -> impl Iterator<Item = &RenderOpOutput> { self.list.iter() }
}

impl From<RenderOpCommon> for RenderOpOutput {
    fn from(op: RenderOpCommon) -> Self { RenderOpOutput::Common(op) }
}

impl Deref for RenderOpOutputVec {
    type Target = InlineVec<RenderOpOutput>;

    fn deref(&self) -> &Self::Target { &self.list }
}

impl DerefMut for RenderOpOutputVec {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.list }
}

/// Ergonomic operator for adding a single operation to the collection.
///
/// This allows using the `+=` operator instead of `.push()` for more readable and
/// concise code. The `Into<RenderOpOutput>` conversion is automatically applied, so types
/// like `RenderOpCommon` can be used directly.
///
/// # Example
///
/// ```
/// # use r3bl_tui::{RenderOpCommon, RenderOpOutputVec, RenderOpOutput, Pos, row, col};
/// let mut render_ops = RenderOpOutputVec::new();
///
/// // Using += operator (more ergonomic)
/// render_ops += RenderOpOutput::Common(RenderOpCommon::MoveCursorPositionAbs(Pos::new((row(5), col(10)))));
///
/// assert_eq!(render_ops.len(), 1);
/// ```
impl AddAssign<RenderOpOutput> for RenderOpOutputVec {
    fn add_assign(&mut self, rhs: RenderOpOutput) { self.list.push(rhs); }
}

impl AddAssign<RenderOpCommon> for RenderOpOutputVec {
    fn add_assign(&mut self, rhs: RenderOpCommon) {
        self.list.push(RenderOpOutput::Common(rhs));
    }
}

impl AddAssign<RenderOpOutput> for &mut RenderOpOutputVec {
    fn add_assign(&mut self, rhs: RenderOpOutput) { self.list.push(rhs); }
}

impl AddAssign<RenderOpCommon> for &mut RenderOpOutputVec {
    fn add_assign(&mut self, rhs: RenderOpCommon) {
        self.list.push(RenderOpOutput::Common(rhs));
    }
}

impl Debug for RenderOpOutputVec {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        const DELIM: &str = "\n  - ";

        let mut iter = self.iter();

        // We don't care about the result of this operation.
        f.write_str("RenderOpOutputVec.len(): ").ok();

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

impl From<Vec<RenderOpOutput>> for RenderOpOutputVec {
    fn from(ops: Vec<RenderOpOutput>) -> Self { Self { list: ops.into() } }
}

impl FromIterator<RenderOpOutput> for RenderOpOutputVec {
    fn from_iter<I: IntoIterator<Item = RenderOpOutput>>(iter: I) -> Self {
        Self {
            list: iter.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_assign_single_op_via_render_op_output() {
        let mut render_ops = RenderOpOutputVec::new();

        assert_eq!(render_ops.len(), 0);

        // Add a single RenderOpOutput variant
        let op = RenderOpOutput::Common(RenderOpCommon::EnterRawMode);

        render_ops += op.clone();

        assert_eq!(render_ops.len(), 1);

        assert_eq!(render_ops[0], op);
    }

    #[test]
    fn test_add_assign_converts_render_op_common_to_output() {
        let mut render_ops = RenderOpOutputVec::new();

        // Add via Into conversion from RenderOpCommon
        let op_common = RenderOpCommon::ExitRawMode;
        render_ops += RenderOpOutput::Common(op_common.clone());

        assert_eq!(render_ops.len(), 1);

        match &render_ops[0] {
            RenderOpOutput::Common(RenderOpCommon::ExitRawMode) => {
                // Test passed - the operation was converted and stored correctly
            }
            _ => panic!("Expected ExitRawMode"),
        }
    }

    #[test]
    fn test_add_assign_multiple_operations() {
        let mut render_ops = RenderOpOutputVec::new();

        // Add multiple operations using += operator
        let op1 = RenderOpCommon::EnterRawMode;
        let op2 = RenderOpCommon::ExitRawMode;
        let op3 = RenderOpCommon::ClearScreen;

        render_ops += RenderOpOutput::Common(op1);
        render_ops += RenderOpOutput::Common(op2);
        render_ops += RenderOpOutput::Common(op3);

        assert_eq!(render_ops.len(), 3);
    }

    #[test]
    fn test_add_assign_vs_push_are_equivalent() {
        let mut render_ops_push = RenderOpOutputVec::new();

        let mut render_ops_add_assign = RenderOpOutputVec::new();

        let op = RenderOpCommon::ClearScreen;

        // Using push (which accepts Into<RenderOpOutput>)
        render_ops_push.push(op.clone());

        // Using += operator (note: RenderOpCommon implements Into<RenderOpOutput>)
        render_ops_add_assign += RenderOpOutput::Common(op);

        // Both should produce the same result
        assert_eq!(render_ops_push.len(), render_ops_add_assign.len());

        assert_eq!(render_ops_push[0], render_ops_add_assign[0]);
    }

    #[test]
    fn test_add_assign_is_ergonomic() {
        let mut render_ops = RenderOpOutputVec::new();

        // This demonstrates the ergonomic improvement over .push()
        // Push accepts Into<RenderOpOutput>, so RenderOpCommon works directly
        render_ops.push(RenderOpCommon::EnterRawMode);

        assert_eq!(render_ops.len(), 1);
    }

    #[test]
    fn test_push_and_add_assign_work_together() {
        let mut render_ops = RenderOpOutputVec::new();

        // Mix push() and += operator
        // push() accepts Into<RenderOpOutput>, but += expects RenderOpOutput directly
        render_ops.push(RenderOpCommon::EnterRawMode);

        render_ops += RenderOpOutput::Common(RenderOpCommon::ExitRawMode);

        render_ops.push(RenderOpCommon::ClearScreen);

        assert_eq!(render_ops.len(), 3);
    }

    #[test]
    fn test_add_assign_render_op_common_directly() {
        let mut render_ops = RenderOpOutputVec::new();

        // Add RenderOpCommon directly without wrapping in RenderOpOutput::Common
        render_ops += RenderOpCommon::EnterRawMode;
        render_ops += RenderOpCommon::ExitRawMode;
        render_ops += RenderOpCommon::ClearScreen;

        assert_eq!(render_ops.len(), 3);

        // Verify the operations were wrapped correctly
        match &render_ops[0] {
            RenderOpOutput::Common(RenderOpCommon::EnterRawMode) => {
                // First operation is correct
            }
            _ => panic!("Expected EnterRawMode at index 0"),
        }
    }
}
