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

use std::{collections::HashMap,
          fmt::Debug,
          ops::{AddAssign, Deref, DerefMut}};

use serde::{Deserialize, Serialize};

use super::*;
use crate::{tui::DEBUG_SHOW_PIPELINE_EXPANDED, *};

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ render_pipeline! │
// ╯                  ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
/// This works w/ [RenderOp] items. It allows them to be added in sequence, and then flushed at the
/// end.
/// 1. This pipeline is meant to hold a list of [RenderOp] items.
/// 2. Once all the [RenderOp] items are added to the correct [ZOrder]s they can then be flushed at
///    the end in order to [paint](RenderPipeline::paint) them to the screen.
/// 3. All the paint operations mutate the global [cursor_position](TWData::cursor_position).
/// 4. The [RENDER_ORDERED_Z_ORDER_ARRAY] contains the priority that is used to paint the different
///    groups of [RenderOp] items.
///
/// This adds given [RenderOp]s to a [RenderOps] and adds that the the pipeline, but does not flush
/// anything. It will return a [RenderPipeline].
///
/// Here's an example.
///
/// ```rust
/// use r3bl_rs_utils::*;
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
/// use r3bl_rs_utils::*;
///
/// let mut pipeline = render_pipeline!(@new_empty); // Returns the newly created pipeline.
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
  // Create a new pipeline & return it. If any ($element)* are passed, then add it to new pipeline.
  (
    @new $arg_z_order: expr
    => $(                   /* Start a repetition. */
      $element:expr         /* Expression. */
    )                       /* End repetition. */
    ,                       /* Comma separated. */
    *                       /* Zero or more times. */
  ) => {
    /* Enclose the expansion in a block so that we can use multiple statements. */
    {
      let mut render_pipeline = RenderPipeline::default();
      /* Start a repetition. */
      $(
        /* Each repeat will contain the following statement, with $element replaced. */
        match render_pipeline.entry($arg_z_order) {
          std::collections::hash_map::Entry::Occupied(mut entry) => {
            entry.get_mut().list.push($element);
          }
          std::collections::hash_map::Entry::Vacant(entry) => {
            entry.insert(render_ops!(@new $arg_z_order => $element));
          }
        }
      )*
      render_pipeline
    }
  };
  // Add a bunch of RenderOps $element+ to the existing $arg_pipeline & return nothing.
  (
    @push_into $arg_pipeline:ident at $arg_z_order: expr
    => $($element:expr),+
  ) => {
    $(
      /* Each repeat will contain the following statement, with $element replaced. */
      match $arg_pipeline.entry($arg_z_order) {
        std::collections::hash_map::Entry::Occupied(mut entry) => {
          entry.get_mut().list.push($element);
        }
        std::collections::hash_map::Entry::Vacant(entry) => {
          entry.insert(render_ops!(@new $arg_z_order => $element));
        }
      }
    )*
  };
  // Add a bunch of RenderPipelines $element+ to the new pipeline, drop them, and return pipeline.
  (@join_and_drop $($element:expr),+) => {{
    let mut pipeline = RenderPipeline::default();
    $(
      /* Each repeat will contain the following statement, with $element replaced. */
      pipeline.join_into($element);
    )*
    pipeline
  }};
  // New.
  (@new_empty) => {
    RenderPipeline::default()
  };
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ RenderPipeline │
// ╯                ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
/// Here's an example. Consider using the macro for convenience (see [render_pipeline!]).
///
/// ```rust
/// use r3bl_rs_utils::*;
///
/// let mut pipeline = RenderPipeline::default();
/// pipeline.push(&ZOrder::Normal, RenderOp::ClearScreen);
/// pipeline.push(&ZOrder::Caret, RenderOp::CursorShow);
/// let len = pipeline.len();
/// let iter = pipeline.iter();
/// ```
#[derive(Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RenderPipeline {
  /// [RenderOps] to paint for each [ZOrder].
  pub pipeline_map: PipelineMap,
}

type PipelineMap = HashMap<ZOrder, RenderOps>;

impl RenderPipeline {
  /// This will add `rhs` to `self`.
  pub fn join_into(&mut self, mut rhs: RenderPipeline) {
    for (z_order, mut render_ops) in rhs.drain() {
      match self.entry(z_order) {
        std::collections::hash_map::Entry::Occupied(mut entry) => {
          entry.get_mut().append(&mut render_ops.list);
        }
        std::collections::hash_map::Entry::Vacant(entry) => {
          entry.insert(render_ops);
        }
      }
    }
  }

  /// This will add the [RenderOp] to the correct [ZOrder].
  pub fn push(&mut self, z_order: &ZOrder, cmd_wrapper: RenderOp) -> &mut Self {
    match self.entry(*z_order) {
      std::collections::hash_map::Entry::Occupied(mut entry) => {
        entry.get_mut().push(cmd_wrapper);
      }
      std::collections::hash_map::Entry::Vacant(entry) => {
        entry.insert(render_ops!(@new *z_order => cmd_wrapper));
      }
    }

    self
  }

  /// Some of the paint operations mutate the global [cursor_position](TWData::cursor_position).
  pub async fn paint(&self, flush_kind: FlushKind, shared_tw_data: &SharedTWData) {
    paint(self, flush_kind, shared_tw_data).await;
    // FUTURE: support termion, along w/ crossterm, by providing another impl of this fn #24
  }
}

pub mod z_order_impl {
  use super::*;

  /// Contains the priority that is used to paint the different groups of [RenderOp] items.
  pub const RENDER_ORDERED_Z_ORDER_ARRAY: [ZOrder; 4] =
    [ZOrder::Normal, ZOrder::High, ZOrder::Caret, ZOrder::Glass];

  #[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
  pub enum ZOrder {
    Normal,
    High,
    Caret,
    Glass,
  }

  impl Default for ZOrder {
    fn default() -> Self { Self::Normal }
  }
}
pub use z_order_impl::*;

mod render_pipeline_helpers {
  use super::*;

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
      if DEBUG_SHOW_PIPELINE_EXPANDED {
        for (z_order, render_ops) in &**self {
          let line: String = format!("[{:?}] {:?}", z_order, render_ops);
          vec_lines.push(line);
        }
      } else {
        for (z_order, render_ops) in &**self {
          let line: String = format!("[{:?}] {:?} ops", z_order, render_ops.len());
          vec_lines.push(line);
        }
      }
      write!(f, "  - {}", vec_lines.join("\n  - "))
    }
  }

  impl AddAssign for RenderPipeline {
    fn add_assign(&mut self, other: RenderPipeline) { self.join_into(other); }
  }

  impl AddAssign<(ZOrder, RenderOp)> for RenderPipeline {
    fn add_assign(&mut self, other: (ZOrder, RenderOp)) { self.push(&other.0, other.1); }
  }
}
