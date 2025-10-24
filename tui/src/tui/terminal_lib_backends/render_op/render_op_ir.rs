// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Intermediate Representation operations for app/component layer.
//!
//! # You Are Here
//!
//! ```text
//! [S1: App/Component] ← [S2: Pipeline] ← [S3: Compositor]
//! ↑↑↑↑
//! [S4: Backend Converter] → [S5: Backend Executor] → [S6: Terminal]
//!
//! RenderOpIR is used by components and app layer
//! ```
//!
//! Components produce [`RenderOpIR`] operations with built-in clipping info.
//! These get processed by the Compositor (Stage 3) to populate the OffscreenBuffer.
//!
//! # Type Safety
//!
//! This enum type ensures only IR-appropriate operations are used in component code.
//! Operations like `PaintTextWithAttributes` (which handles clipping) are IR-specific
//! and cannot be accidentally used in backend code.

use super::{RenderOpCommon, RenderOpsLocalData};
use crate::{InlineString, InlineVec, LockedOutputDevice, PaintRenderOpImplCrossterm,
            Size, TerminalLibBackend, TuiStyle, ok};
use std::ops::{AddAssign, Deref, DerefMut};

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
pub struct RenderOpsIR {
    pub list: InlineVec<RenderOpIR>,
}

impl RenderOpsIR {
    /// Create a new empty collection of IR operations.
    pub fn new() -> Self {
        Self {
            list: InlineVec::new(),
        }
    }

    /// Add a single operation to the collection.
    pub fn push(&mut self, op: RenderOpIR) { self.list.push(op); }

    /// Add multiple operations to the collection.
    pub fn extend(&mut self, ops: impl IntoIterator<Item = RenderOpIR>) {
        self.list.extend(ops);
    }

    /// Get the number of operations in the collection.
    pub fn len(&self) -> usize { self.list.len() }

    /// Check if the collection is empty.
    pub fn is_empty(&self) -> bool { self.list.is_empty() }

    /// Iterate over the operations.
    pub fn iter(&self) -> impl Iterator<Item = &RenderOpIR> { self.list.iter() }

    /// Executes all render operations in the collection sequentially.
    ///
    /// This method processes each [`RenderOpIR`] in the list, maintaining local state
    /// for optimization and routing each operation to the appropriate backend
    /// implementation based on the configured terminal library.
    ///
    /// # Parameters
    /// - `skip_flush`: Mutable reference to control flush behavior
    /// - `window_size`: Current terminal window dimensions
    /// - `locked_output_device`: Locked terminal output for thread-safe writing
    /// - `is_mock`: Whether this is a mock execution for testing
    pub fn execute_all(
        &self,
        skip_flush: &mut bool,
        window_size: Size,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) {
        let mut render_local_data = RenderOpsLocalData::default();
        for render_op_ir in &self.list {
            RenderOpsIR::route_paint_render_op_ir_to_backend(
                &mut render_local_data,
                skip_flush,
                render_op_ir,
                window_size,
                locked_output_device,
                is_mock,
            );
        }
    }

    /// Routes a single IR render operation to the appropriate backend implementation.
    ///
    /// This method acts as a dispatcher, selecting the correct terminal library
    /// backend (currently Crossterm) and delegating the actual rendering work
    /// to the backend-specific implementation.
    ///
    /// # Parameters
    /// - `render_local_data`: Mutable state for render optimization
    /// - `skip_flush`: Mutable reference to control flush behavior
    /// - `render_op_ir`: The specific IR operation to execute
    /// - `window_size`: Current terminal window dimensions
    /// - `locked_output_device`: Locked terminal output for thread-safe writing
    /// - `is_mock`: Whether this is a mock execution for testing
    pub fn route_paint_render_op_ir_to_backend(
        render_local_data: &mut RenderOpsLocalData,
        skip_flush: &mut bool,
        render_op_ir: &RenderOpIR,
        window_size: Size,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) {
        match super::TERMINAL_LIB_BACKEND {
            TerminalLibBackend::Crossterm => {
                // Convert RenderOpIR to something the paint method can understand.
                // For now, we'll implement this in Phase 5+ when we handle the
                // compositor. This is a placeholder that will be
                // filled in later.
                match render_op_ir {
                    RenderOpIR::Common(common_op) => {
                        PaintRenderOpImplCrossterm {}.paint_common(
                            skip_flush,
                            common_op,
                            window_size,
                            render_local_data,
                            locked_output_device,
                            is_mock,
                        );
                    }
                    RenderOpIR::PaintTextWithAttributes(text, style) => {
                        // IR-level text painting with clipping handled by Compositor
                        // The Compositor has already applied clipping, so we just
                        // paint the text as-is using the
                        // unified renderer.
                        PaintRenderOpImplCrossterm::paint_text_with_attributes(
                            text,
                            *style,
                            window_size,
                            render_local_data,
                            locked_output_device,
                        );
                    }
                }
            }
            TerminalLibBackend::Termion => unimplemented!(),
        }
    }
}

impl Deref for RenderOpsIR {
    type Target = InlineVec<RenderOpIR>;

    fn deref(&self) -> &Self::Target { &self.list }
}

impl DerefMut for RenderOpsIR {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.list }
}

impl AddAssign<RenderOpIR> for RenderOpsIR {
    fn add_assign(&mut self, rhs: RenderOpIR) { self.list.push(rhs); }
}

impl std::fmt::Debug for RenderOpsIR {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

impl From<Vec<RenderOpIR>> for RenderOpsIR {
    fn from(ops: Vec<RenderOpIR>) -> Self { Self { list: ops.into() } }
}

impl FromIterator<RenderOpIR> for RenderOpsIR {
    fn from_iter<I: IntoIterator<Item = RenderOpIR>>(iter: I) -> Self {
        Self {
            list: iter.into_iter().collect(),
        }
    }
}
