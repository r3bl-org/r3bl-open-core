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
use crate::*;

/// Paint the render pipeline. The render pipeline contains a list of [RenderOps] for each [ZOrder].
/// This function is responsible for:
/// 1. Actually executing those [RenderOps] in the correct order.
/// 2. And routing the execution to the correct backend specified in [TERMINAL_LIB_BACKEND].
///
/// See [RenderOps] for more details of "atomic paint operations".
pub async fn paint(
    pipeline: &RenderPipeline,
    flush_kind: FlushKind,
    shared_global_data: &SharedGlobalData,
) {
    let maybe_saved_offscreen_buffer = shared_global_data
        .read()
        .await
        .maybe_saved_offscreen_buffer
        .clone();
    let offscreen_buffer = pipeline.convert(shared_global_data).await;
    match maybe_saved_offscreen_buffer {
        None => {
            perform_full_paint(&offscreen_buffer, flush_kind, shared_global_data).await;
        }
        Some(saved_offscreen_buffer) => {
            // Compare offscreen buffers & paint only the diff.
            match saved_offscreen_buffer.diff(&offscreen_buffer) {
                OffscreenBufferDiffResult::NotComparable => {
                    perform_full_paint(&offscreen_buffer, flush_kind, shared_global_data).await;
                }
                OffscreenBufferDiffResult::Comparable(ref diff_chunks) => {
                    perform_diff_paint(diff_chunks, shared_global_data).await;
                }
            }
        }
    }
    shared_global_data
        .write()
        .await
        .maybe_saved_offscreen_buffer = Some(offscreen_buffer);

    async fn perform_diff_paint(
        diff_chunks: &PixelCharDiffChunks,
        shared_global_data: &SharedGlobalData,
    ) {
        match TERMINAL_LIB_BACKEND {
            TerminalLibBackend::Crossterm => {
                let mut crossterm_impl = OffscreenBufferPaintImplCrossterm {};
                let render_ops = crossterm_impl.render_diff(diff_chunks).await;
                crossterm_impl
                    .paint_diff(render_ops, shared_global_data)
                    .await;
            }
            TerminalLibBackend::Termion => todo!(), // FUTURE: implement OffscreenBufferPaint trait for termion
        }
    }

    async fn perform_full_paint(
        offscreen_buffer: &OffscreenBuffer,
        flush_kind: FlushKind,
        shared_global_data: &SharedGlobalData,
    ) {
        match TERMINAL_LIB_BACKEND {
            TerminalLibBackend::Crossterm => {
                let mut crossterm_impl = OffscreenBufferPaintImplCrossterm {};
                let render_ops = crossterm_impl.render(offscreen_buffer).await;
                crossterm_impl
                    .paint(render_ops, flush_kind, shared_global_data)
                    .await;
            }
            TerminalLibBackend::Termion => todo!(), // FUTURE: implement OffscreenBufferPaint trait for termion
        }
    }
}

/// 1. Ensure that the [Position] is within the bounds of the terminal window using
///    [RenderOpsLocalData].
/// 2. If the [Position] is outside of the bounds of the window then it is clamped to the nearest
///    edge of the window. This clamped [Position] is returned.
/// 3. This also saves the clamped [Position] to [RenderOpsLocalData].
pub async fn sanitize_and_save_abs_position(
    orig_abs_pos: Position,
    shared_global_data: &SharedGlobalData,
    local_data: &mut RenderOpsLocalData,
) -> Position {
    let Size {
        col_count: max_cols,
        row_count: max_rows,
    } = shared_global_data.read().await.window_size;

    let mut sanitized_abs_pos: Position = orig_abs_pos;

    if orig_abs_pos.col_index > max_cols {
        sanitized_abs_pos.col_index = max_cols;
    }

    if orig_abs_pos.row_index > max_rows {
        sanitized_abs_pos.row_index = max_rows;
    }

    // Save the cursor position to local data.
    local_data.cursor_position = sanitized_abs_pos;

    debug(orig_abs_pos, sanitized_abs_pos);

    return sanitized_abs_pos;

    fn debug(orig_pos: Position, sanitized_pos: Position) {
        call_if_true!(DEBUG_TUI_MOD, {
            if sanitized_pos != orig_pos {
                let msg = format!(
                    "pipeline : 📍🗜️ Attempt to set cursor position {orig_pos:?} \
                    outside of terminal window; clamping to nearest edge of window {sanitized_pos:?}"
                );
                log_info(msg);
            }
        });

        call_if_true!(DEBUG_TUI_SHOW_PIPELINE_EXPANDED, {
            let msg = format!(
                "pipeline : 📍 Save the cursor position {sanitized_pos:?} \
                to SharedGlobalData"
            );
            log_info(msg);
        });
    }
}

pub mod paint_exports {
    use super::*;

    #[async_trait]
    pub trait PaintRenderOp {
        async fn paint(
            &mut self,
            skip_flush: &mut bool,
            render_op: &RenderOp,
            shared_global_data: &SharedGlobalData,
            local_data: &mut RenderOpsLocalData,
        );
    }
}
pub use paint_exports::*;
