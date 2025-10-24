//! # Rendering Pipeline Orchestration
//!
//! # You Are Here
//!
//! ```text
//! [S1: App/Component] → [S2: Pipeline] → [S3: Compositor] →
//! [S4: Backend Converter] → [S5: Backend Executor] → [S6: Terminal]
//!
//! ↑ paint.rs coordinates all these stages ↑
//! (Orchestration: ties everything together)
//! ```
//!
//! This module orchestrates the entire rendering pipeline:
//! 1. Takes [`RenderPipeline`] from the app
//! 2. Feeds it through the Compositor to create [`OffscreenBuffer`]
//! 3. Performs diff calculations for selective redraw
//! 4. Routes operations to the appropriate backend (Crossterm/Termion)
//! 5. Manages flushing and display synchronization
//!
//! > **For the complete rendering architecture**, see [`super`] (parent module).

// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{FlushKind, RenderOpIR, RenderOpsLocalData, RenderPipeline};
use crate::{DEBUG_TUI_COMPOSITOR, DEBUG_TUI_SHOW_PIPELINE_EXPANDED, GlobalData,
            LengthOps, LockedOutputDevice, OffscreenBuffer, OffscreenBufferPaint,
            OffscreenBufferPaintImplCrossterm, Pos, Size, TERMINAL_LIB_BACKEND,
            TerminalLibBackend, diff_chunks::PixelCharDiffChunks};
use std::fmt::Debug;

pub trait PaintRenderOp {
    fn paint(
        &mut self,
        skip_flush: &mut bool,
        render_op: &RenderOpIR,
        window_size: Size,
        render_local_data: &mut RenderOpsLocalData,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    );
}

/// Paint the render pipeline. The render pipeline contains a list of [`crate::RenderOps`]
/// for each [`crate::ZOrder`]. This function is responsible for:
/// 1. Actually executing those [`crate::RenderOps`] in the correct order.
/// 2. And routing the execution to the correct backend specified in
///    [`TERMINAL_LIB_BACKEND`].
///
/// See [`crate::RenderOps`] for more details of "atomic paint operations".
///
/// # Panics
///
/// This will panic if the lock is poisoned, which can happen if a thread
/// panics while holding the lock. To avoid panics, ensure that the code that
/// locks the mutex does not panic while holding the lock.
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
    fn perform_diff_paint(
        diff_chunks: &PixelCharDiffChunks,
        window_size: Size,
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
        ofs_buf: &OffscreenBuffer,
        flush_kind: FlushKind,
        window_size: Size,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) {
        match TERMINAL_LIB_BACKEND {
            TerminalLibBackend::Crossterm => {
                let mut crossterm_impl = OffscreenBufferPaintImplCrossterm {};
                let render_ops = crossterm_impl.render(ofs_buf);
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

    // If this is None, then a full paint will be performed & the offscreen buffer will be
    // saved to global_data.
    let maybe_saved_ofs_buf = global_data.maybe_saved_ofs_buf.clone();

    let window_size = global_data.window_size;

    let Some(mut buffer_from_pool) = global_data.offscreen_buffer_pool.take() else {
        panic!("All offscreen buffers are currently taken. This should never happen.");
    };

    pipeline.compose_render_ops_into_ofs_buf(
        window_size,
        &mut buffer_from_pool,
        &mut global_data.memoized_text_widths,
    );

    match maybe_saved_ofs_buf {
        None => {
            perform_full_paint(
                &buffer_from_pool,
                flush_kind,
                window_size,
                locked_output_device,
                is_mock,
            );
        }
        Some(saved_offscreen_buffer) => {
            // Compare offscreen buffers & paint only the diff.
            match saved_offscreen_buffer.diff(&buffer_from_pool) {
                None => {
                    perform_full_paint(
                        &buffer_from_pool,
                        flush_kind,
                        window_size,
                        locked_output_device,
                        is_mock,
                    );
                }
                Some(ref diff_chunks) => {
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

    // Give back the buffer to the pool.
    if let Some(old_buffer) = global_data.maybe_saved_ofs_buf.take() {
        global_data.offscreen_buffer_pool.give_back(old_buffer);
    }

    // Save the buffer to global_data.
    global_data.maybe_saved_ofs_buf = Some(buffer_from_pool);
}

/// 1. Ensure that the [Pos] is within the bounds of the terminal window using
///    [`RenderOpsLocalData`].
/// 2. If the [Pos] is outside the bounds of the window then it is clamped to the nearest
///    edge of the window. This clamped [Pos] is returned.
/// 3. This also saves the clamped [Pos] to [`RenderOpsLocalData`].
///
/// Note that printing [`crate::SPACER_GLYPH`] by
/// [`crate::compositor_render_ops_to_ofs_buf::process_render_op`] will trigger
/// clipping the [Pos] to the nearest edge of the window. This is OK. This is because the
/// spacer is painted at the very last column of the terminal window due to the way in
/// which the spacers are repeated. No checks are supposed to be done when
/// [`crate::OffscreenBuffer`] is painting, so there is no clean way to skip this clipping
/// check.
///
/// See the `test_sanitize_and_save_abs_pos` for more details on the behavior of this
/// function.
pub fn sanitize_and_save_abs_pos(
    orig_abs_pos: Pos,
    window_size: Size,
    render_local_data: &mut RenderOpsLocalData,
) -> Pos {
    let Size {
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
        .min(window_width.convert_to_index());

    // Equivalent code:
    // if *orig_abs_pos.row_index >= *window_height {
    //     *sanitized_abs_pos.row_index = *window_height - 1;
    // }
    sanitized_abs_pos.row_index = sanitized_abs_pos
        .row_index
        .min(window_height.convert_to_index());

    // Save the cursor position to local data.
    render_local_data.cursor_pos = sanitized_abs_pos;

    debug(orig_abs_pos, sanitized_abs_pos);

    sanitized_abs_pos
}

fn debug(orig_pos: Pos, sanitized_pos: Pos) {
    DEBUG_TUI_COMPOSITOR.then(|| {
        if sanitized_pos != orig_pos {
            // % is Display, ? is Debug.
            tracing::info!(
                message = "pipeline : ⮻ Attempt to set cursor position (orig) outside of terminal window; clamping to nearest edge of window (sanitized)",
                orig = ?orig_pos,
                sanitized = ?sanitized_pos
            );
        }
    });

    DEBUG_TUI_SHOW_PIPELINE_EXPANDED.then(|| {
        // % is Display, ? is Debug.
        tracing::info!(
            message = "pipeline : ⮺ Save the cursor position (sanitized) to SharedGlobalData",
            sanitized = ?sanitized_pos
        );
    });
}
