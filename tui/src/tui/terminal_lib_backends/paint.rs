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
// TODO: accept Option<RenderPipeline> from shared_tw_data to compare pipeline against (speed up re-rendering)
pub async fn paint(pipeline: &RenderPipeline, flush_kind: FlushKind, shared_tw_data: &SharedTWData) {
  let mut skip_flush = false;

  if let FlushKind::ClearBeforeFlush = flush_kind {
    RenderOp::default().clear_before_flush();
  }

  // Vec of special (hoisted) RenderOp that should only be rendered at the very end.
  let mut special_hoisted_op_vec: Vec<RenderOp> = vec![];

  // Execute the (unspecial) RenderOps, in the correct order of the ZOrder enum.
  for z_order in RENDER_ORDERED_Z_ORDER_ARRAY.iter() {
    if let Some(render_ops_set) = pipeline.get(z_order) {
      for render_ops in render_ops_set.iter() {
        for render_op in render_ops.iter() {
          if let RenderOp::RequestShowCaretAtPositionAbs(_) | RenderOp::RequestShowCaretAtPositionRelTo(_, _) =
            render_op
          {
            special_hoisted_op_vec.push(render_op.clone());
          } else {
            route_paint_render_op_to_backend(&mut skip_flush, render_op, shared_tw_data).await;
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
    route_paint_render_op_to_backend(&mut skip_flush, special_render_op, shared_tw_data).await;
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
///    [SharedTWData].
/// 2. If the [Position] is outside of the bounds of the window then it is clamped to the nearest
///    edge of the window. This clamped [Position] is returned.
/// 3. This also saves the clamped [Position] to [SharedTWData].
pub async fn sanitize_and_save_abs_position(orig_abs_pos: Position, shared_tw_data: &SharedTWData) -> Position {
  let Size {
    cols: max_cols,
    rows: max_rows,
  } = shared_tw_data.read().await.size;

  let mut sanitized_abs_pos: Position = orig_abs_pos;

  if orig_abs_pos.col > max_cols {
    sanitized_abs_pos.col = max_cols;
  }

  if orig_abs_pos.row > max_rows {
    sanitized_abs_pos.row = max_rows;
  }

  // Save the cursor position.
  shared_tw_data.write().await.cursor_position = sanitized_abs_pos;

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
          to SharedTWData",
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
  shared_tw_data: &SharedTWData,
) {
  match TERMINAL_LIB_BACKEND {
    TerminalLibBackend::Crossterm => {
      RenderOpImplCrossterm {}
        .paint(skip_flush, render_op, shared_tw_data)
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
  async fn paint(&self, skip_flush: &mut bool, render_op: &RenderOp, shared_tw_data: &SharedTWData);
}
