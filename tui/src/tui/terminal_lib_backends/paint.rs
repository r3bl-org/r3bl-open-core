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

use async_trait::async_trait;
use r3bl_rs_utils_core::*;

use super::*;
use crate::{tui::DEBUG_SHOW_PIPELINE, *};

// ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
// ┃ Paint the render pipeline ┃
// ┛                           ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// The render pipeline contains a list of [RenderOps] for each [ZOrder]. This function is
/// responsible for:
/// 1. Actually executing those [RenderOps] in the correct order.
/// 2. And routing the execution to the correct backend specified in [TERMINAL_LIB_BACKEND].
///
/// The following [RenderOp]s get special treatment and are "hoisted" (which means they are painted
/// last, ie, on top of all else) regardless of the [ZOrder] they were added to in the pipeline:
/// 1. [RenderOp::RequestShowCaretAtPositionAbs]
/// 2. [RenderOp::RequestShowCaretAtPositionRelTo]
///
/// See [RenderOps] for more details of "atomic paint operations".
pub async fn paint(pipeline: &RenderPipeline, flush_kind: FlushKind, shared_global_data: &SharedGlobalData) {
  let maybe_saved_pipeline = shared_global_data.read().await.maybe_render_pipeline.clone();

  if let Some(saved_pipeline) = maybe_saved_pipeline {
    optimized_paint::paint(pipeline, &saved_pipeline, flush_kind, shared_global_data).await;
  } else {
    no_optimize_paint(pipeline, flush_kind, shared_global_data).await;
  }

  shared_global_data.write().await.maybe_render_pipeline = Some(pipeline.clone());
}

pub mod optimized_paint {
  use super::*;

  impl RenderOps {
    async fn paint(&self, skip_flush: &mut bool, shared_global_data: &SharedGlobalData) {
      for op in self.iter() {
        route_paint_render_op_to_backend(skip_flush, op, shared_global_data).await;
      }
    }

    fn contains_hoisted_ops(&self) -> bool {
      self.iter().any(|op| {
        matches!(
          op,
          RenderOp::RequestShowCaretAtPositionAbs(_) | RenderOp::RequestShowCaretAtPositionRelTo(_, _)
        )
      })
    }

    fn contains_paint_caret_ops(&self) -> bool {
      self.iter().any(|op| {
        if let RenderOp::PrintTextWithAttributes(_, Some(style)) = op {
          style.reverse
        } else {
          false
        }
      })
    }

    fn contains_clear_screen_ops(&self) -> bool { self.iter().any(|op| matches!(op, RenderOp::ClearScreen)) }

    fn get_text_with_attributes_ops(&self) -> Vec<usize> {
      let vec_op: Vec<&RenderOp> = self
        .iter()
        .filter(|op| matches!(op, RenderOp::PrintTextWithAttributes(_, _)))
        .collect();

      vec_op
        .iter()
        .map(|op| {
          if let RenderOp::PrintTextWithAttributes(string, _) = op {
            string.len()
          } else {
            0
          }
        })
        .collect()
    }

    /// Text can be more or less but not equal. Heuristic to determine if we should paint the entire
    /// screen or just the changed parts.
    fn has_less_or_more_text_than(&self, other: &Self) -> bool {
      let lhs_ops = self.get_text_with_attributes_ops();
      let rhs_ops = other.get_text_with_attributes_ops();
      lhs_ops.iter().sum::<usize>() != rhs_ops.iter().sum::<usize>()
    }

    // Heuristic to determine if we should paint the entire screen or just the changed parts.
    fn has_different_lines_than(&self, other: &Self) -> bool {
      let lhs_ops = self.get_text_with_attributes_ops();
      let rhs_ops = other.get_text_with_attributes_ops();
      lhs_ops.len() != rhs_ops.len()
    }

    fn has_fewer_ops_than(&self, other: &Self) -> bool { self.len() < other.len() }
  }

  impl<'a> From<&'a RenderPipeline> for Vec<&'a RenderOps> {
    fn from(pipeline: &'a RenderPipeline) -> Self {
      // Vec of special (hoisted) RenderOps that should only be rendered at the very end.
      let mut vec_hoisted_ops: Vec<&RenderOps> = vec![];

      // Vec of all other RenderOps that should be rendered in the order they were added to the
      // pipeline.
      let mut vec_ops: Vec<&RenderOps> = vec![];

      // Iterate over all ZOrder in the pipeline, populate vec_ops, and vec_hoisted_ops.
      for z_order in RENDER_ORDERED_Z_ORDER_ARRAY.iter() {
        if let Some(vec_render_ops) = pipeline.get(z_order) {
          for render_ops in vec_render_ops.iter() {
            if render_ops.contains_hoisted_ops() {
              vec_hoisted_ops.push(render_ops);
            } else {
              vec_ops.push(render_ops);
            }
          }
        }
      }

      // Log error if special_hoisted_op_vec has more than one item.
      if vec_hoisted_ops.len() > 1 {
        log_no_err!(
          WARN,
          "🥕 Too many requests to show caret at position (some will be clobbered): {:?}",
          vec_hoisted_ops,
        );
      }

      // Add the special (hoisted) RenderOps to the end of the ops Vec.
      vec_ops.extend(vec_hoisted_ops);

      vec_ops
    }
  }

  pub async fn paint(
    pipeline: &RenderPipeline,
    saved_pipeline: &RenderPipeline,
    flush_kind: FlushKind,
    shared_global_data: &SharedGlobalData,
  ) {
    let mut skip_flush = false;

    let new_vec_render_ops: Vec<&RenderOps> = pipeline.into();
    let saved_vec_render_ops: Vec<&RenderOps> = saved_pipeline.into();

    // Fewer new_vec_render_ops than saved_vec_render_ops so abort optimization. Eg: a dialog box in
    // the ZOrder::Glass layer has been removed, and this will require a full repaint, so the dialog
    // box itself doesn't "ghost".
    if new_vec_render_ops.len() < saved_vec_render_ops.len() {
      no_optimize_paint(pipeline, flush_kind, shared_global_data).await;
      return;
    }

    // More or equal new_vec_render_ops than saved_vec_render_ops so continue optimization.
    for (new_vec_index, new_ops) in new_vec_render_ops.iter().enumerate() {
      match saved_vec_render_ops.get(new_vec_index) {
        // RenderOps found at new_vec_index in new_vec_render_ops & saved_vec_render_ops.
        Some(saved_ops) => {
          if new_ops != saved_ops {
            new_ops.paint(&mut skip_flush, shared_global_data).await;
            print_debug_diff_render_ops(new_ops, saved_ops);
          }
        }
        // No RenderOps found at new_vec_index in saved_vec_render_ops.
        None => {
          // Paint the whole RenderOps.
          if new_ops.contains_clear_screen_ops() {
            no_optimize_paint(pipeline, flush_kind, shared_global_data).await;
            return;
          } else {
            new_ops.paint(&mut skip_flush, shared_global_data).await;
          }
        }
      }
    }

    // Flush everything to the terminal.
    if !skip_flush {
      RenderOp::default().flush()
    };

    // Debug output.
    call_if_true!(DEBUG_SHOW_PIPELINE, {
      log_no_err!(INFO, "🎨 render_pipeline::paint() ok ✅: pipeline: \n{:?}", pipeline,);
    });
  }

  fn print_debug_diff_render_ops(new_ops: &RenderOps, saved_ops: &RenderOps) {
    call_if_true!(DEBUG_SHOW_PAINT_OPTIMIZATION_HEURISTIC, {
      log_no_err!(
        DEBUG,
        r#"
🤔🧨🎨 [Repaint introspection] new_ops != saved_ops,
{:?} has_fewer_ops_than          {:?} has_less_or_more_text_than  {:?} has_different_lines_than
{:?} contains_paint_caret_ops    {:?} contains_clear_screen_ops   {:?} contains_hoisted_ops
new_ops: {:?}, 
saved_ops: {:?}
"#,
        if new_ops.has_fewer_ops_than(saved_ops) {
          "✅"
        } else {
          "🚫"
        },
        if new_ops.has_less_or_more_text_than(saved_ops) {
          "✅"
        } else {
          "🚫"
        },
        if new_ops.has_different_lines_than(saved_ops) {
          "✅"
        } else {
          "🚫"
        },
        if new_ops.contains_paint_caret_ops() {
          "✅"
        } else {
          "🚫"
        },
        if new_ops.contains_clear_screen_ops() {
          "✅"
        } else {
          "🚫"
        },
        if new_ops.contains_hoisted_ops() { "✅" } else { "🚫" },
        new_ops,
        saved_ops,
      );
    });
  }
}

pub async fn no_optimize_paint(
  pipeline: &RenderPipeline,
  flush_kind: FlushKind,
  shared_global_data: &SharedGlobalData,
) {
  let mut skip_flush = false;

  if let FlushKind::ClearBeforeFlush = flush_kind {
    RenderOp::default().clear_before_flush();
  }

  // Vec of special (hoisted) RenderOp that should only be rendered at the very end.
  let mut special_hoisted_op_vec: Vec<RenderOp> = vec![];

  // Execute the (unspecial) RenderOps, in the correct order of the ZOrder enum.
  for z_order in RENDER_ORDERED_Z_ORDER_ARRAY.iter() {
    if let Some(render_ops_vec) = pipeline.get(z_order) {
      for (_render_ops_index, render_ops) in render_ops_vec.iter().enumerate() {
        for (_render_op_index, render_op) in render_ops.iter().enumerate() {
          match render_op {
            RenderOp::RequestShowCaretAtPositionAbs(_) | RenderOp::RequestShowCaretAtPositionRelTo(_, _) => {
              special_hoisted_op_vec.push(render_op.clone());
            }
            _ => {
              route_paint_render_op_to_backend(&mut skip_flush, render_op, shared_global_data).await;
            }
          }
        }
      }
    }
  }

  // Log error if special_hoisted_op_vec has more than one item.
  if special_hoisted_op_vec.len() > 1 {
    log_no_err!(
      WARN,
      "🥕 Too many requests to show caret at position (some will be clobbered): {:?}",
      special_hoisted_op_vec,
    );
  }

  // Execute the special ops (at the very end).
  for special_render_op in &special_hoisted_op_vec {
    route_paint_render_op_to_backend(&mut skip_flush, special_render_op, shared_global_data).await;
  }

  // Flush everything to the terminal.
  if !skip_flush {
    RenderOp::default().flush()
  };

  // Debug output.
  call_if_true!(DEBUG_SHOW_PIPELINE, {
    log_no_err!(INFO, "🎨 render_pipeline::paint() ok ✅: pipeline: \n{:?}", pipeline,);
  });
}

/// 1. Ensure that the [Position] is within the bounds of the terminal window using
///    [SharedGlobalData].
/// 2. If the [Position] is outside of the bounds of the window then it is clamped to the nearest
///    edge of the window. This clamped [Position] is returned.
/// 3. This also saves the clamped [Position] to [SharedGlobalData].
pub async fn sanitize_and_save_abs_position(orig_abs_pos: Position, shared_global_data: &SharedGlobalData) -> Position {
  let Size {
    col_count: max_cols,
    row_count: max_rows,
  } = shared_global_data.read().await.size;

  let mut sanitized_abs_pos: Position = orig_abs_pos;

  if orig_abs_pos.col_index > max_cols {
    sanitized_abs_pos.col_index = max_cols;
  }

  if orig_abs_pos.row_index > max_rows {
    sanitized_abs_pos.row_index = max_rows;
  }

  // Save the cursor position.
  shared_global_data.write().await.cursor_position = sanitized_abs_pos;

  debug(orig_abs_pos, sanitized_abs_pos);

  return sanitized_abs_pos;

  fn debug(orig_pos: Position, sanitized_pos: Position) {
    call_if_true!(DEBUG_TUI_MOD, {
      if sanitized_pos != orig_pos {
        log_no_err!(
          INFO,
          "pipeline : 📍🗜️ Attempt to set cursor position {:?} \
          outside of terminal window; clamping to nearest edge of window {:?}",
          orig_pos,
          sanitized_pos
        );
      }
    });

    call_if_true!(DEBUG_SHOW_PIPELINE_EXPANDED, {
      log_no_err!(
        INFO,
        "pipeline : 📍 Save the cursor position {:?} \
          to SharedGlobalData",
        sanitized_pos
      );
    });
  }
}

// ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
// ┃ Route paint RenderOp to backend ┃
// ┛                                 ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
pub async fn route_paint_render_op_to_backend(
  skip_flush: &mut bool,
  render_op: &RenderOp,
  shared_global_data: &SharedGlobalData,
) {
  match TERMINAL_LIB_BACKEND {
    TerminalLibBackend::Crossterm => {
      RenderOpImplCrossterm {}
        .paint(skip_flush, render_op, shared_global_data)
        .await;
    }
    TerminalLibBackend::Termion => todo!(), // FUTURE: implement PaintRenderOp trait for termion
  }
}

// ┏━━━━━━━━━━━━━━━━━━━━━┓
// ┃ PaintRenderOp trait ┃
// ┛                     ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#[async_trait]
pub trait PaintRenderOp {
  async fn paint(&self, skip_flush: &mut bool, render_op: &RenderOp, shared_global_data: &SharedGlobalData);
}
