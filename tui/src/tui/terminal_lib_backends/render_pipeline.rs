// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # Pipeline Stage 2: Collection & Organization
//!
//! # You Are Here
//!
//! ```text
//! [S1: App/Component] → [S2: Pipeline] ← YOU ARE HERE
//! [S3: Compositor] → [S4: Backend Converter] → [S5: Backend Executor] → [S6: Terminal]
//! ```
//!
//! **Input**: [`RenderOpsIR`] operations from components
//! **Output**: Organized operations sorted by [`ZOrder`] (layer depth)
//! **Role**: Aggregate and organize rendering operations before compositor processing
//!
//! > **For the complete rendering architecture**, see [`super`] (parent module).
//!
//! ## What This Stage Does
//!
//! The `RenderPipeline` collects render operations from multiple components and organizes them
//! into layers by Z-order. This ensures that when the compositor renders to the offscreen buffer,
//! components are drawn in the correct visual order (background to foreground).
//!
//! ### Key Operations
//! - **Collect**: Aggregate [`RenderOpsIR`] from multiple components
//! - **Organize**: Group by [`ZOrder`] to establish visual stacking
//! - **Prepare**: Structure data for the compositor's next stage
//!
//! ### No Rendering Yet
//! This stage is purely organizational. No actual rendering to the terminal (or even to the
//! offscreen buffer) happens here. That's the compositor's job.

use super::{ZOrder, paint::paint};
use crate::{FlushKind, GlobalData, InlineVec, LockedOutputDevice, RenderOpsIR, ok,
            tui::DEBUG_TUI_SHOW_PIPELINE_EXPANDED};
use smallvec::smallvec;
use std::{collections::{HashMap, hash_map::Entry},
          fmt::Debug,
          ops::{AddAssign, Deref, DerefMut}};

/// Macro to make it easier to create a [`RenderPipeline`]. It works w/ [`RenderOp`]
/// items. It allows them to be added in sequence, and then flushed at the end.
/// 1. This pipeline is meant to hold a list of [`RenderOp`] items.
/// 2. Once all the [`RenderOp`] items are added to the correct [`ZOrder`]s they can then
///    be flushed at the end in order to [paint](RenderPipeline::paint()) them to the
///    screen.
/// 3. [`get_render_order()`](ZOrder::get_render_order) contains the priority that is used
///    to paint the different groups of [`RenderOp`] items.
///
/// This adds given [`RenderOp`]s to a [`RenderOpsIR`] and adds that the the pipeline, but
/// does not flush anything. It will return a [`RenderPipeline`].
///
/// Here's an example.
///
/// ```
/// use r3bl_tui::*;
///
/// let mut pipeline = render_pipeline!(@new ZOrder::Normal =>
///   RenderOp::ClearScreen,
///   RenderOp::ResetColor
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
///   RenderOp::ClearScreen,
///   RenderOp::ResetColor
/// ); // Returns nothing.
/// let len = pipeline.len();
/// let iter = pipeline.iter();
/// ```
///
/// Decl macro docs:
/// - <https://veykril.github.io/tlborm/decl-macros/macros-methodical.html#repetitions>
///
/// `HashMap` docs:
/// - <https://doc.rust-lang.org/std/collections/struct.HashMap.html#examples>
#[macro_export]
macro_rules! render_pipeline {
    // No args. Returns a new default pipeline.
    () => {
        $crate::RenderPipeline::default()
    };

    // @new: Create a new pipeline & return it. If any RenderOp ($arg_render_op)* are passed, then add it
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
        let mut render_ops = $crate::RenderOpsIR::default();
        /* Start a repetition. */
        $(
            /* Each repeat will contain the following statement, with $arg_render_op replaced. */
            render_ops.push($arg_render_op);
        )*
        let mut render_pipeline = $crate::RenderPipeline::default();
        render_pipeline.push($arg_z_order, render_ops);
        render_pipeline
        }
    };

    // @push_into: Add a bunch of RenderOp $arg_render_op+ to the existing $arg_pipeline & return nothing.
    (
        @push_into
        $arg_pipeline: ident
        at $arg_z_order: expr
        => $($arg_render_op: expr),+
    ) => {
        let mut render_ops = $crate::RenderOpsIR::default();
        $(
        /* Each repeat will contain the following statement, with $arg_render_op replaced. */
        render_ops.push($arg_render_op);
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

    // @push_styled_texts_into: Add a bunch of RenderOp $arg_render_op+ to the existing $arg_pipeline & return nothing.
    (
        @push_styled_texts_into
        $arg_pipeline: ident
        at $arg_z_order: expr
        => $arg_styled_texts: expr
      ) => {
        let mut render_ops = $crate::RenderOpsIR::default();
        $crate::render_tui_styled_texts_into(&$arg_styled_texts, &mut render_ops);
        $arg_pipeline.push($arg_z_order, render_ops);
      };
}

type PipelineMap = HashMap<ZOrder, InlineVec<RenderOpsIR>>;

/// See [`render_pipeline`!] for the documentation. Also consider using it instead of this
/// struct directly for convenience.
///
/// Here's an example.
///
/// ```
/// use r3bl_tui::*;
///
/// let mut pipeline = render_pipeline!();
/// pipeline.push(ZOrder::Normal, {
///     let mut ops = RenderOpsIR::new();
///     ops.push(RenderOpIR::Common(RenderOpCommon::ClearScreen));
///     ops
/// });
/// pipeline.push(ZOrder::Glass, {
///     let mut ops = RenderOpsIR::new();
///     ops.push(RenderOpIR::Common(RenderOpCommon::ClearScreen));
///     ops
/// });
/// let len = pipeline.len();
/// let iter = pipeline.iter();
/// ```
#[derive(Default, Clone, PartialEq, Eq)]
pub struct RenderPipeline {
    /// [`RenderOpsIR`] to paint for each [`ZOrder`].
    pub pipeline_map: PipelineMap,
}

impl RenderPipeline {
    /// This will add `rhs` to `self`.
    pub fn join_into(&mut self, mut rhs: RenderPipeline) {
        for (z_order, mut rhs_render_ops_vec) in rhs.drain() {
            // Insert rhs_render_ops_vec into self_render_ops_vec.
            match self.entry(z_order) {
                Entry::Occupied(mut self_existing_entry) => {
                    let self_render_ops_vec = self_existing_entry.get_mut();
                    rhs_render_ops_vec.drain(..).for_each(|render_ops| {
                        self_render_ops_vec.push(render_ops);
                    });
                }
                Entry::Vacant(self_new_entry) => {
                    self_new_entry.insert(rhs_render_ops_vec);
                }
            }
        }
    }

    /// Add the given [`RenderOpsIR`] to the pipeline at the given [`ZOrder`].
    pub fn push(&mut self, z_order: ZOrder, render_ops: RenderOpsIR) {
        match self.pipeline_map.entry(z_order) {
            // Insert render_ops into existing set.
            Entry::Occupied(mut existing_entry) => {
                let render_ops_vec = existing_entry.get_mut();
                render_ops_vec.push(render_ops);
            }
            // Create new set & insert render_ops in it.
            Entry::Vacant(new_entry) => {
                new_entry.insert(smallvec![render_ops]);
            }
        }
    }

    /// At the given [`ZOrder`] there can be a [`InlineVec`] of [`RenderOpsIR`]. Grab
    /// Count all the render operations in the set for the given z_order.
    /// Returns the total count of all individual RenderOpIR operations across all collections.
    #[must_use]
    pub fn get_all_render_op_in(&self, z_order: ZOrder) -> Option<usize> {
        let vec_render_ops = self.pipeline_map.get(&z_order)?;
        let mut total_count = 0;
        for render_ops in vec_render_ops {
            total_count += render_ops.len();
        }
        Some(total_count)
    }

    pub fn paint<S, AS>(
        &self,
        flush_kind: FlushKind,
        global_data: &mut GlobalData<S, AS>,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) where
        S: Debug + Default + Clone + Sync + Send,
        AS: Debug + Default + Clone + Sync + Send,
    {
        paint(self, flush_kind, global_data, locked_output_device, is_mock);
        // FUTURE: support termion, along w/ crossterm, by providing another impl of this
    }

    /// Move the [`RenderOpsIR`] in the 'from' [`ZOrder`] (in self) to the 'to' [`ZOrder`]
    /// (in self).
    pub fn hoist(&mut self, z_order_from: ZOrder, z_order_to: ZOrder) {
        // If the 'from' [ZOrder] is not in the pipeline, then there's nothing to do.
        if !self.pipeline_map.contains_key(&z_order_from) {
            return;
        }

        // Move the [RenderOpsIR] from the 'from' [ZOrder] to the 'to' [ZOrder].
        let mut from = self.pipeline_map.remove(&z_order_from).unwrap_or_default();

        match self.pipeline_map.entry(z_order_to) {
            Entry::Occupied(mut to_existing_entry) => {
                let to = to_existing_entry.get_mut();
                from.drain(..).for_each(|render_ops| {
                    to.push(render_ops);
                });
            }
            Entry::Vacant(to_new_entry) => {
                to_new_entry.insert(from);
            }
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

        for (count, (z_order, vec_render_ops)) in map.iter().enumerate() {
            if count > 0 {
                write!(f, "{DELIM}")?;
            }
            if DEBUG_TUI_SHOW_PIPELINE_EXPANDED {
                write!(f, "[{z_order:?}] {vec_render_ops:?}")?;
            } else {
                write!(f, "[{z_order:?}] {:?} RenderOpsIR", vec_render_ops.len())?;
            }
        }

        ok!()
    }
}

impl AddAssign for RenderPipeline {
    fn add_assign(&mut self, other: RenderPipeline) { self.join_into(other); }
}
