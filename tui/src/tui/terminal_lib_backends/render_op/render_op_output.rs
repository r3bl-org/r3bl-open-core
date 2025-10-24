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
use std::ops::{AddAssign, Deref, DerefMut};

/// Terminal output operations for backend/execution layer.
///
/// These operations are optimized for terminal execution. They are generated
/// by backend converters (e.g., OffscreenBufferPaint) after processing the IR
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
    /// after the OffscreenBuffer has been fully processed. The compositor has
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
pub struct RenderOpsOutput {
    pub list: InlineVec<RenderOpOutput>,
}

impl RenderOpsOutput {
    /// Create a new empty collection of output operations.
    pub fn new() -> Self {
        Self {
            list: InlineVec::new(),
        }
    }

    /// Add a single operation to the collection.
    pub fn push(&mut self, op: RenderOpOutput) { self.list.push(op); }

    /// Add multiple operations to the collection.
    pub fn extend(&mut self, ops: impl IntoIterator<Item = RenderOpOutput>) {
        self.list.extend(ops);
    }

    /// Get the number of operations in the collection.
    pub fn len(&self) -> usize { self.list.len() }

    /// Check if the collection is empty.
    pub fn is_empty(&self) -> bool { self.list.is_empty() }

    /// Iterate over the operations.
    pub fn iter(&self) -> impl Iterator<Item = &RenderOpOutput> { self.list.iter() }
}

impl Deref for RenderOpsOutput {
    type Target = InlineVec<RenderOpOutput>;

    fn deref(&self) -> &Self::Target { &self.list }
}

impl DerefMut for RenderOpsOutput {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.list }
}

impl AddAssign<RenderOpOutput> for RenderOpsOutput {
    fn add_assign(&mut self, rhs: RenderOpOutput) { self.list.push(rhs); }
}

impl std::fmt::Debug for RenderOpsOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const DELIM: &str = "\n  - ";

        let mut iter = self.iter();

        // We don't care about the result of this operation.
        f.write_str("RenderOpsOutput.len(): ").ok();
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

impl From<Vec<RenderOpOutput>> for RenderOpsOutput {
    fn from(ops: Vec<RenderOpOutput>) -> Self { Self { list: ops.into() } }
}

impl FromIterator<RenderOpOutput> for RenderOpsOutput {
    fn from_iter<I: IntoIterator<Item = RenderOpOutput>>(iter: I) -> Self {
        Self {
            list: iter.into_iter().collect(),
        }
    }
}
