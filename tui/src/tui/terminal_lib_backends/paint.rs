/*
 *   Copyright (c) 2022-2025 R3BL LLC
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

use std::fmt::Debug;

use r3bl_core::{call_if_true, Dim, LockedOutputDevice, Pos};

use super::{FlushKind, RenderOp, RenderOpsLocalData, RenderPipeline};
use crate::{diff_chunks::{OffscreenBufferDiffResult, PixelCharDiffChunks},
            GlobalData,
            OffscreenBuffer,
            OffscreenBufferPaint,
            OffscreenBufferPaintImplCrossterm,
            TerminalLibBackend,
            DEBUG_TUI_COMPOSITOR,
            DEBUG_TUI_SHOW_PIPELINE_EXPANDED,
            TERMINAL_LIB_BACKEND};

pub trait PaintRenderOp {
    fn paint(
        &mut self,
        skip_flush: &mut bool,
        render_op: &RenderOp,
        window_size: Dim,
        local_data: &mut RenderOpsLocalData,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    );
}

/// Paint the render pipeline. The render pipeline contains a list of [crate::RenderOps]
/// for each [crate::ZOrder]. This function is responsible for:
/// 1. Actually executing those [crate::RenderOps] in the correct order.
/// 2. And routing the execution to the correct backend specified in
///    [TERMINAL_LIB_BACKEND].
///
/// See [crate::RenderOps] for more details of "atomic paint operations".
pub fn paint<S, AS>(
    pipeline: &RenderPipeline,
    flush_kind: FlushKind,
    global_data: &mut GlobalData<S, AS>,
    locked_output_device: LockedOutputDevice<'_>,
    is_mock: bool,
) where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
{
    let maybe_saved_offscreen_buffer = global_data.maybe_saved_offscreen_buffer.clone();

    let window_size = global_data.window_size;

    // PERF: [ ] figure out how this affects offscreen buffer memory
    let offscreen_buffer = pipeline.convert(window_size);

    match maybe_saved_offscreen_buffer {
        None => {
            perform_full_paint(
                &offscreen_buffer,
                flush_kind,
                window_size,
                locked_output_device,
                is_mock,
            );
        }
        Some(saved_offscreen_buffer) => {
            // Compare offscreen buffers & paint only the diff.
            match saved_offscreen_buffer.diff(&offscreen_buffer) {
                OffscreenBufferDiffResult::NotComparable => {
                    perform_full_paint(
                        &offscreen_buffer,
                        flush_kind,
                        window_size,
                        locked_output_device,
                        is_mock,
                    );
                }
                OffscreenBufferDiffResult::Comparable(ref diff_chunks) => {
                    perform_diff_paint(
                        diff_chunks,
                        window_size,
                        locked_output_device,
                        is_mock,
                    );
                }
            }
        }
    }

    global_data.maybe_saved_offscreen_buffer = Some(offscreen_buffer);

    fn perform_diff_paint(
        diff_chunks: &PixelCharDiffChunks,
        window_size: Dim,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) {
        match TERMINAL_LIB_BACKEND {
            TerminalLibBackend::Crossterm => {
                let mut crossterm_impl = OffscreenBufferPaintImplCrossterm {};
                let render_ops = crossterm_impl.render_diff(diff_chunks);
                crossterm_impl.paint_diff(
                    render_ops,
                    window_size,
                    locked_output_device,
                    is_mock,
                );
            }
            TerminalLibBackend::Termion => unimplemented!(),
        }
    }

    fn perform_full_paint(
        offscreen_buffer: &OffscreenBuffer,
        flush_kind: FlushKind,
        window_size: Dim,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) {
        match TERMINAL_LIB_BACKEND {
            TerminalLibBackend::Crossterm => {
                let mut crossterm_impl = OffscreenBufferPaintImplCrossterm {};
                let render_ops = crossterm_impl.render(offscreen_buffer);
                crossterm_impl.paint(
                    render_ops,
                    flush_kind,
                    window_size,
                    locked_output_device,
                    is_mock,
                );
            }
            TerminalLibBackend::Termion => unimplemented!(),
        }
    }
}

/// 1. Ensure that the [Pos] is within the bounds of the terminal window using
///    [RenderOpsLocalData].
/// 2. If the [Pos] is outside of the bounds of the window then it is clamped to the
///    nearest edge of the window. This clamped [Pos] is returned.
/// 3. This also saves the clamped [Pos] to [RenderOpsLocalData].
///
/// Note that printing [r3bl_core::SPACER_GLYPH] by
/// [crate::render_pipeline_to_offscreen_buffer::process_render_op] will trigger clipping
/// the [Pos] to the nearest edge of the window. This is OK. This is because the spacer is
/// painted at the very last column of the terminal window due to the way in which the
/// spacers are repeated. No checks are supposed to be done when [crate::OffscreenBuffer]
/// is painting, so there is no clean way to skip this clipping check.
///
/// See the `test_sanitize_and_save_abs_pos` for more details on the behavior of this
/// function.
pub fn sanitize_and_save_abs_pos(
    orig_abs_pos: Pos,
    window_size: Dim,
    local_data: &mut RenderOpsLocalData,
) -> Pos {
    let Dim {
        col_width: window_width,
        row_height: window_height,
    } = window_size;

    let mut sanitized_abs_pos = orig_abs_pos;

    // Equivalent code:
    // if *orig_abs_pos.col_index >= *window_width {
    //     *sanitized_abs_pos.col_index = *window_width - 1;
    // }
    sanitized_abs_pos.col_index = sanitized_abs_pos
        .col_index
        .min(window_width.convert_to_col_index());

    // Equivalent code:
    // if *orig_abs_pos.row_index >= *window_height {
    //     *sanitized_abs_pos.row_index = *window_height - 1;
    // }
    sanitized_abs_pos.row_index = sanitized_abs_pos
        .row_index
        .min(window_height.convert_to_row_index());

    // Save the cursor position to local data.
    local_data.cursor_pos = sanitized_abs_pos;

    debug(orig_abs_pos, sanitized_abs_pos);

    sanitized_abs_pos
}

fn debug(orig_pos: Pos, sanitized_pos: Pos) {
    call_if_true!(DEBUG_TUI_COMPOSITOR, {
        if sanitized_pos != orig_pos {
            // % is Display, ? is Debug.
            tracing::info!(
                message = "pipeline : ⮻ Attempt to set cursor position (orig) outside of terminal window; clamping to nearest edge of window (sanitized)",
                orig = ?orig_pos,
                sanitized = ?sanitized_pos
            );
        }
    });

    call_if_true!(DEBUG_TUI_SHOW_PIPELINE_EXPANDED, {
        // % is Display, ? is Debug.
        tracing::info!(
            message = "pipeline : ⮺ Save the cursor position (sanitized) to SharedGlobalData",
            sanitized = ?sanitized_pos
        );
    });
}
