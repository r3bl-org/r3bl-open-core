/*
 *   Copyright (c) 2022 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

use std::{collections::{hash_map::Entry, HashMap},
          fmt::Debug,
          ops::{AddAssign, Deref, DerefMut}};

use serde::{Deserialize, Serialize};

use super::*;
use crate::{tui::DEBUG_TUI_SHOW_PIPELINE_EXPANDED, *};

/// This macro is a convenience macro for creating a [RenderPipeline]. It works w/ [RenderOp] items.
/// It allows them to be added in sequence, and then flushed at the end.
/// 1. This pipeline is meant to hold a list of [RenderOp] items.
/// 2. Once all the [RenderOp] items are added to the correct [ZOrder]s they can then be
///    flushed at the end in order to [paint](RenderPipeline::paint()) them to the screen.
/// 3. [get_render_order()](ZOrder::get_render_order) contains the priority that is used
///    to paint the different groups of [RenderOp] items.
///
/// This adds given [RenderOp]s to a [RenderOps] and adds that the the pipeline, but does not flush
/// anything. It will return a [RenderPipeline].
///
/// Here's an example.
///
/// ```rust
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
/// ```rust
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
/// - <https://veykril.github.io/tlborm/decl-macros/macros-methodical.html#repetitions> HashMap
/// docs:
/// - <https://doc.rust-lang.org/std/collections/struct.HashMap.html#examples>
#[macro_export]
macro_rules! render_pipeline {
    // No args. Returns a new
    () => {
        RenderPipeline::default()
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
        let mut render_ops = RenderOps::default();
        /* Start a repetition. */
        $(
            /* Each repeat will contain the following statement, with $arg_render_op replaced. */
            render_ops.push($arg_render_op);
        )*
        let mut render_pipeline = RenderPipeline::default();
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
        let mut render_ops = RenderOps::default();
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
        let mut pipeline = render_pipeline!();
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
        let mut render_ops = RenderOps::default();
        $arg_styled_texts.render_into(&mut render_ops);
        $arg_pipeline.push($arg_z_order, render_ops);
      };
}

/// See [render_pipeline!] for the documentation. Also consider using it instead of this struct
/// directly for convenience.
///
/// Here's an example.
///
/// ```rust
/// use r3bl_tui::*;
///
/// let mut pipeline = render_pipeline!();
/// pipeline.push(ZOrder::Normal, render_ops!(@new RenderOp::ClearScreen));
/// pipeline.push(ZOrder::Glass, render_ops!(@new RenderOp::ClearScreen));
/// let len = pipeline.len();
/// let iter = pipeline.iter();
/// ```
#[derive(Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RenderPipeline {
    /// [RenderOps] to paint for each [ZOrder].
    pub pipeline_map: PipelineMap,
}

type PipelineMap = HashMap<ZOrder, Vec<RenderOps>>;

mod render_pipeline_impl {
    use super::*;

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

        /// Add the given [RenderOps] to the pipeline at the given [ZOrder].
        pub fn push(&mut self, z_order: ZOrder, render_ops: RenderOps) {
            match self.pipeline_map.entry(z_order) {
                // Insert render_ops into existing set.
                Entry::Occupied(mut existing_entry) => {
                    let render_ops_vec = existing_entry.get_mut();
                    render_ops_vec.push(render_ops);
                }
                // Create new set & insert render_ops in it.
                Entry::Vacant(new_entry) => {
                    new_entry.insert(vec![render_ops]);
                }
            }
        }

        /// At the given [ZOrder] there can be a [Vec] of [RenderOps]. Grab all the [RenderOps] in the
        /// set, get all their [RenderOp] and return them in a [Vec].
        pub fn get_all_render_op_in(&self, z_order: ZOrder) -> Option<Vec<RenderOp>> {
            let vec_render_ops = self.pipeline_map.get(&z_order)?;
            let mut vec_render_op: Vec<RenderOp> = vec![];
            for render_ops in vec_render_ops {
                for render_op in render_ops.iter() {
                    vec_render_op.push(render_op.clone());
                }
            }
            Some(vec_render_op)
        }

        pub fn paint<S, AS>(
            &self,
            flush_kind: FlushKind,
            global_data: &mut GlobalData<S, AS>,
        ) where
            S: Debug + Default + Clone + Sync + Send,
            AS: Debug + Default + Clone + Sync + Send,
        {
            paint(self, flush_kind, global_data);
            // FUTURE: support termion, along w/ crossterm, by providing another impl of this fn #24
        }

        /// Move the [RenderOps] in the 'from' [ZOrder] (in self) to the 'to' [ZOrder] (in self).
        pub fn hoist(&mut self, z_order_from: ZOrder, z_order_to: ZOrder) {
            // If the 'from' [ZOrder] is not in the pipeline, then there's nothing to do.
            if !self.pipeline_map.contains_key(&z_order_from) {
                return;
            }

            // Move the [RenderOps] from the 'from' [ZOrder] to the 'to' [ZOrder].
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
            let mut vec_lines: Vec<String> = vec![];
            if DEBUG_TUI_SHOW_PIPELINE_EXPANDED {
                for (z_order, render_ops) in &**self {
                    let line: String = format!("[{z_order:?}] {render_ops:?}");
                    vec_lines.push(line);
                }
            } else {
                for (z_order, vec_render_ops) in &**self {
                    let line: String =
                        format!("[{z_order:?}] {:?} RenderOps", vec_render_ops.len());
                    vec_lines.push(line);
                }
            }
            write!(f, "  - {}", vec_lines.join("\n  - "))
        }
    }

    impl AddAssign for RenderPipeline {
        fn add_assign(&mut self, other: RenderPipeline) { self.join_into(other); }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ZOrder {
    Normal,
    High,
    Glass,
}

mod z_order_impl {
    use super::*;

    impl ZOrder {
        /// Contains the priority that is used to paint the different groups of [RenderOp] items.
        pub fn get_render_order() -> [ZOrder; 3] {
            [ZOrder::Normal, ZOrder::High, ZOrder::Glass]
        }
    }

    impl Default for ZOrder {
        fn default() -> Self { Self::Normal }
    }
}
