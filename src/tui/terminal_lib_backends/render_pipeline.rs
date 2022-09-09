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

use std::{collections::HashMap, fmt::Debug};

use serde::{Deserialize, Serialize};

use crate::{terminal_lib_backends::DEBUG_SHOW_PIPELINE, *};

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ RenderPipeline │
// ╯                ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
/// This works w/ [RenderOp] items. It allows them to be added in sequence, and then flushed at the
/// end. Here's an example.
///
/// ```ignore
/// let mut pipeline = RenderPipeline::default();
/// pipeline.add(RenderOp::ClearScreen);
/// pipeline.add(RenderOp::CursorShow);
/// pipeline.flush();
/// ```
#[derive(Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RenderPipeline {
  /// [RenderOps] to paint for each [ZOrder].
  pub pipeline_map: PipelineMap,
}

type PipelineMap = HashMap<ZOrder, RenderOps>;

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ZOrder {
  Normal,
  High,
  Caret,
  Glass,
}

const RENDER_ORDERED_Z_ORDER_ARRAY: [ZOrder; 4] =
  [ZOrder::Normal, ZOrder::High, ZOrder::Caret, ZOrder::Glass];

impl Default for ZOrder {
  fn default() -> Self { Self::Normal }
}

impl RenderPipeline {
  /// This will add `rhs` to `self`.
  pub fn join_into(&mut self, mut rhs: RenderPipeline) {
    for (z_order, mut render_ops) in rhs.pipeline_map.drain() {
      match self.pipeline_map.entry(z_order) {
        std::collections::hash_map::Entry::Occupied(mut entry) => {
          entry.get_mut().list.append(&mut render_ops.list);
        }
        std::collections::hash_map::Entry::Vacant(entry) => {
          entry.insert(render_ops);
        }
      }
    }
  }

  pub fn push(&mut self, z_order: &ZOrder, cmd_wrapper: RenderOp) -> &mut Self {
    match self.pipeline_map.entry(*z_order) {
      std::collections::hash_map::Entry::Occupied(mut entry) => {
        entry.get_mut().list.push(cmd_wrapper);
      }
      std::collections::hash_map::Entry::Vacant(entry) => {
        entry.insert(render_ops!(@new *z_order => cmd_wrapper));
      }
    }

    self
  }

  // FUTURE: support termion, along w/ crossterm, by providing another impl of this fn #24
  pub async fn paint(&self, flush_kind: FlushKind, shared_tw_data: &SharedTWData) {
    pipeline::paint(self, flush_kind, shared_tw_data).await;
  }
}

mod render_pipeline_helpers {
  use std::ops::AddAssign;

  use super::*;
  use crate::tui::terminal_lib_backends::render_pipeline::RenderPipeline;

  impl Debug for RenderPipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      let mut vec_lines: Vec<String> = vec![];
      for (z_order, render_ops) in &self.pipeline_map {
        let line: String = format!("[{:?}] {:?}", z_order, render_ops);
        vec_lines.push(line);
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

pub mod pipeline {
  use super::*;

  pub async fn paint(
    pipeline: &RenderPipeline, flush_kind: FlushKind, shared_tw_data: &SharedTWData,
  ) {
    let mut skip_flush = false;

    if let FlushKind::ClearBeforeFlush = flush_kind {
      RenderOp::default().clear_before_flush();
    }

    // List of special commands that should only be rendered at the very end.
    let mut hoisted_op_vec: Vec<RenderOp> = vec![];

    // Execute the RenderOps, in the correct order of the ZOrder enum.
    for z_order in RENDER_ORDERED_Z_ORDER_ARRAY.iter() {
      if let Some(render_ops) = pipeline.pipeline_map.get(z_order) {
        for command_ref in &render_ops.list {
          if let RenderOp::RequestShowCaretAtPositionAbs(_)
          | RenderOp::RequestShowCaretAtPositionRelTo(_, _) = command_ref
          {
            hoisted_op_vec.push(command_ref.clone());
          } else {
            route_paint_render_op_to_backend(&mut skip_flush, command_ref, shared_tw_data).await;
          }
        }
      }
    }

    // Log error if hoisted_commands has more than one item.
    if hoisted_op_vec.len() > 1 {
      log_no_err!(
        WARN,
        "🥕 Too many requests to draw caret (some will be clobbered): {:?}",
        hoisted_op_vec,
      );
    }

    // Execute the hoisted commands (at the very end).
    for command_ref in &hoisted_op_vec {
      route_paint_render_op_to_backend(&mut skip_flush, command_ref, shared_tw_data).await;
    }

    // Flush everything to the terminal.
    if !skip_flush {
      RenderOp::default().flush()
    };

    // Debug output.
    call_if_true!(DEBUG_SHOW_PIPELINE, {
      log_no_err!(
        INFO,
        "🎨 render_pipeline::paint() ok ✅: pipeline: \n{:?}",
        pipeline,
      );
    });
  }

  /// 1. Ensure that the [Position] is within the bounds of the terminal window using
  ///    [SharedTWData].
  /// 2. If the [Position] is outside of the bounds of the window then it is clamped to the nearest
  ///    edge of the window. This clamped [Position] is returned.
  /// 3. This also saves the clamped [Position] to [SharedTWData].
  pub async fn sanitize_abs_position(
    orig_abs_pos: Position, shared_tw_data: &SharedTWData,
  ) -> Position {
    let Size {
      col: max_cols,
      row: max_rows,
    } = shared_tw_data.read().await.size;

    let mut new_abs_pos: Position = orig_abs_pos;

    if orig_abs_pos.col > max_cols {
      new_abs_pos.col = max_cols;
    }

    if orig_abs_pos.row > max_rows {
      new_abs_pos.row = max_rows;
    }

    // Save the cursor position.
    shared_tw_data.write().await.cursor_position = new_abs_pos;

    debug_sanitize_abs_position(orig_abs_pos, new_abs_pos);

    return new_abs_pos;

    fn debug_sanitize_abs_position(orig_pos: Position, sanitized_pos: Position) {
      call_if_debug_true!({
        if sanitized_pos != orig_pos {
          log_no_err!(INFO, "🔏 Attempt to set position {:?} outside of terminal window. Clamping to nearest edge of window {:?}.", 
          orig_pos,
          sanitized_pos);
        }
      });
    }
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ render_pipeline! │
// ╯                  ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
/// This adds given [RenderOp]s to a [RenderOps] and adds that the the pipeline, but does not flush
/// anything. It will return a [RenderPipeline].
///
/// Here's an example.
///
/// ```ignore
/// let mut pipeline = render_pipeline!(@new ZOrder::Normal =>
///   RenderOp::ClearScreen,
///   RenderOp::ResetColor
/// ); // Returns the newly created pipeline.
/// ```
///
/// Another example.
///
/// ```ignore
/// let mut pipeline = render_pipeline!(@new_empty); // Returns the newly created pipeline.
/// render_pipeline!(@push pipeline =>
///   RenderOp::ClearScreen,
///   RenderOp::ResetColor
/// ); // Returns nothing.
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
        match render_pipeline.pipeline_map.entry($arg_z_order) {
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
      match $arg_pipeline.pipeline_map.entry($arg_z_order) {
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
