// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # Pipeline Stage 2: Collection & Organization
//!
//! # You Are Here: **Stage 2** (Organization Layer)
//!
//! ```text
//! [Stage 1: App/Component]
//!   ↓
//! [Stage 2: Pipeline] ← YOU ARE HERE
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
//! **Input**: [`RenderOpIRVec`] operations from components
//! **Output**: Organized operations sorted by [`ZOrder`] (layer depth)
//! **Role**: Aggregate and organize rendering operations before compositor processing
//!
//! <div class="warning">
//!
//! **For the complete 6-stage rendering pipeline with visual diagrams and stage
//! reference table**, see the [rendering pipeline overview].
//!
//! </div>
//!
//! ## Navigation
//! - **Previous stage**: [`render_op_ir` mod docs] (Stage 1: IR Generation)
//! - **Next stage**: [`compositor_render_ops_to_ofs_buf` mod docs] (Stage 3: Rendering to
//!   Buffer)
//!
//! ## What This Stage Does
//!
//! The `RenderPipeline` collects render operations from multiple components and organizes
//! them into layers by Z-order. This ensures that when the compositor renders to the
//! offscreen buffer, components are drawn in the correct visual order (background to
//! foreground).
//!
//! ### Key Operations
//! - **Collect**: Aggregate [`RenderOpIRVec`] from multiple components
//! - **Organize**: Group by [`ZOrder`] to establish visual stacking
//! - **Prepare**: Structure data for the compositor's next stage
//!
//! ### No Rendering Yet
//! This stage is purely organizational. No actual rendering to the terminal (or even to
//! the offscreen buffer) happens here. That's the compositor's job.
//!
//! [`compositor_render_ops_to_ofs_buf` mod docs]: mod@crate::compositor_render_ops_to_ofs_buf
//! [`render_op_ir` mod docs]: mod@crate::render_op::render_op_ir
//! [rendering pipeline overview]: mod@crate::terminal_lib_backends#rendering-pipeline-architecture

use super::{ZOrder, paint::paint};
use crate::{FlushKind, GlobalData, LockedOutputDevice, RenderOpIRVec, ok,
            tui::DEBUG_TUI_SHOW_PIPELINE_EXPANDED};
use std::{fmt::Debug,
          ops::{AddAssign, Deref, DerefMut}};
use strum::EnumCount;

/// Macro to make it easier to create a [`RenderPipeline`]. It works w/ [`RenderOpIR`]
/// items. It allows them to be added in sequence, and then flushed at the end.
/// 1. This pipeline is meant to hold a list of [`RenderOpIR`] items.
/// 2. Once all the [`RenderOpIR`] items are added to the correct [`ZOrder`]s they can
///    then be flushed at the end in order to paint them to the screen.
/// 3. [`ZOrder::get_render_order`] contains the priority that is used to paint the
///    different groups of render operation items.
///
/// This adds given [`RenderOpIR`]s to a [`RenderOpIRVec`] and adds that the the pipeline,
/// but does not flush anything. It will return a [`RenderPipeline`].
///
/// Here's an example.
///
/// ```
/// use r3bl_tui::*;
///
/// let mut pipeline = render_pipeline!(@new ZOrder::Normal =>
///   RenderOpIR::from(RenderOpCommon::ClearScreen),
///   RenderOpIR::from(RenderOpCommon::ResetColor)
/// ); // Returns the newly created pipeline.
/// let len = pipeline.len();
/// let iter = pipeline.iter();
/// ```
///
/// Another example.
///
/// ```
/// use r3bl_tui::*;
///
/// let mut pipeline = render_pipeline!();
/// render_pipeline!(@push_into pipeline at ZOrder::Normal =>
///   RenderOpIR::from(RenderOpCommon::ClearScreen),
///   RenderOpIR::from(RenderOpCommon::ResetColor)
/// ); // Returns nothing.
/// let len = pipeline.len();
/// let iter = pipeline.iter();
/// ```
///
/// Decl macro docs:
/// - <https://veykril.github.io/tlborm/decl-macros/macros-methodical.html#repetitions>
///
/// [`RenderOpIR`]: crate::RenderOpIR
/// [`RenderOpIRVec`]: crate::RenderOpIRVec
/// [`RenderPipeline`]: crate::RenderPipeline
/// [`ZOrder::get_render_order`]: crate::ZOrder::get_render_order
/// [`ZOrder`]: crate::ZOrder
#[macro_export]
macro_rules! render_pipeline {
    // No args. Returns a new default pipeline.
    () => {
        $crate::RenderPipeline::default()
    };

    // @new: Create a new pipeline & return it. If any RenderOpIR ($arg_render_op)* are passed, then add it
    // to new pipeline.
    (
        @new
        $arg_z_order: expr
        => $(                   /* Start a repetition. */
        $arg_render_op: expr  /* Expression. */
        )                       /* End repetition. */
        ,                       /* Comma separated. */
        *                       /* Zero or more times. */
        $(,)*                   /* Optional trailing comma https://stackoverflow.com/a/43143459/2085356. */
    ) => {
        /* Enclose the expansion in a block so that we can use multiple statements. */
        {
        let mut render_ops = $crate::RenderOpIRVec::default();
        /* Start a repetition. */
        $(
            /* Each repeat will contain the following statement, with $arg_render_op replaced. */
            render_ops  += ($arg_render_op);
        )*
        let mut render_pipeline = $crate::RenderPipeline::default();
        render_pipeline.push($arg_z_order, render_ops);
        render_pipeline
        }
    };

    // @push_into: Add a bunch of RenderOpIR $arg_render_op+ to the existing $arg_pipeline & return nothing.
    (
        @push_into
        $arg_pipeline: ident
        at $arg_z_order: expr
        => $($arg_render_op: expr),+
    ) => {
        let mut render_ops = $crate::RenderOpIRVec::default();
        $(
        /* Each repeat will contain the following statement, with $arg_render_op replaced. */
        render_ops  += ($arg_render_op);
        )*
        $arg_pipeline.push($arg_z_order, render_ops);
    };

    // @join_and_drop: Add $arg_other_pipeline+ (a bunch of other RenderPipelines) to the new
    // pipeline, drop them, and return it.
    (
        @join_and_drop
        $($arg_other_pipeline: expr),+
    ) => {{
        let mut pipeline = $crate::render_pipeline!();
        $(
        /* Each repeat will contain the following statement, with $arg_other_pipeline replaced. */
        pipeline.join_into($arg_other_pipeline);
        )*
        pipeline
    }};

    // @push_styled_texts_into: Add a bunch of RenderOpIR $arg_render_op+ to the existing $arg_pipeline & return nothing.
    (
        @push_styled_texts_into
        $arg_pipeline: ident
        at $arg_z_order: expr
        => $arg_styled_texts: expr
      ) => {
        let mut render_ops = $crate::RenderOpIRVec::default();
        $crate::render_tui_styled_texts_into(&$arg_styled_texts, &mut render_ops);
        $arg_pipeline.push($arg_z_order, render_ops);
      };
}

type PipelineMap = [RenderOpIRVec; ZOrder::COUNT];

/// See [`render_pipeline`!] for the documentation. Also consider using it instead of this
/// struct directly for convenience.
#[derive(Default, Clone, PartialEq, Eq)]
pub struct RenderPipeline {
    /// [`RenderOpIRVec`] to paint for each [`ZOrder`].
    pub pipeline_map: PipelineMap,
}

impl RenderPipeline {
    /// This will add `rhs` to `self`.
    pub fn join_into(&mut self, mut rhs: RenderPipeline) {
        for z_order in ZOrder::get_render_order() {
            let rhs_render_ops = std::mem::take(&mut rhs.pipeline_map[z_order]);
            self.pipeline_map[z_order].list.extend(rhs_render_ops.list);
        }
    }

    /// Add the given [`RenderOpIRVec`] to the pipeline at the given [`ZOrder`].
    pub fn push(&mut self, z_order: ZOrder, mut render_ops: RenderOpIRVec) {
        self.pipeline_map[z_order].list.append(&mut render_ops.list);
    }

    /// Returns a reference to the operations for the given [`ZOrder`].
    #[must_use]
    pub fn get(&self, z_order: &ZOrder) -> &RenderOpIRVec { &self.pipeline_map[*z_order] }

    /// Returns the total count of all individual [`RenderOpIR`] operations in the
    /// [`ZOrder`].
    ///
    /// [`RenderOpIR`]: crate::RenderOpIR
    #[must_use]
    pub fn get_all_render_op_in(&self, z_order: ZOrder) -> usize {
        self.pipeline_map[z_order].len()
    }

    pub fn paint<S, AS>(
        &self,
        flush_kind: FlushKind,
        global_data: &mut GlobalData<S, AS>,
        locked_output_device: LockedOutputDevice<'_>,
    ) where
        S: Debug + Default + Clone + Sync + Send,
        AS: Debug + Default + Clone + Sync + Send,
    {
        paint(self, flush_kind, global_data, locked_output_device);
    }

    /// Move the [`RenderOpIRVec`] in the 'from' [`ZOrder`] (in self) to the 'to'
    /// [`ZOrder`] (in self).
    pub fn hoist(&mut self, z_order_from: ZOrder, z_order_to: ZOrder) {
        if z_order_from == z_order_to {
            return;
        }
        let mut from = std::mem::take(&mut self.pipeline_map[z_order_from]);
        self.pipeline_map[z_order_to].list.append(&mut from.list);
    }

    /// Clear all operations from the pipeline, but retain their heap capacities.
    pub fn clear(&mut self) {
        for render_ops in &mut self.pipeline_map {
            render_ops.list.clear();
        }
    }
}

impl Deref for RenderPipeline {
    type Target = PipelineMap;

    fn deref(&self) -> &Self::Target { &self.pipeline_map }
}

impl DerefMut for RenderPipeline {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.pipeline_map }
}

impl Debug for RenderPipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // If count > 0, then we need to add a delimiter.
        const DELIM: &str = "\n  - ";

        let map = &**self;

        for (count, z_order) in ZOrder::get_render_order().into_iter().enumerate() {
            let vec_render_ops = &map[z_order];

            if vec_render_ops.is_empty() {
                continue;
            }

            if count > 0 {
                write!(f, "{DELIM}")?;
            }
            if DEBUG_TUI_SHOW_PIPELINE_EXPANDED {
                write!(f, "[{z_order:?}] {vec_render_ops:?}")?;
            } else {
                write!(f, "[{z_order:?}] {:?} RenderOpIRVec", vec_render_ops.len())?;
            }
        }

        ok!()
    }
}

impl AddAssign for RenderPipeline {
    fn add_assign(&mut self, other: RenderPipeline) { self.join_into(other); }
}
