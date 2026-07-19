// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # Stage 4: Backend Converter (Shared)
//!
//! This module implements **Stage 4 of the rendering pipeline**: converting the
//! [`OfsBuf`] (produced by the Compositor in Stage 3) into optimized
//! [`RenderOpOutputVec`] operations for backend execution.
//!
//! <div class="warning">
//!
//! **For the complete 6-stage rendering pipeline with visual diagrams and stage
//! reference table**, see the [rendering pipeline overview].
//!
//! </div>
//!
//! ## Why This Lives in [`ofs_buf/`]
//!
//! Stage 4 is fundamentally an **`OfsBuf` operation**:
//! - It reads FROM the buffer (like other buffer operations)
//! - It uses [`diff_chunks`] (also in this module) for selective redraw optimization
//! - It's buffer-centric, not backend-specific
//!
//! ## Backend Independence
//!
//! This converter is **shared by both Crossterm and `DirectToAnsi` backends**.
//! The backends only differ in Stage 5 (Backend Executor):
//! - Crossterm: [`crossterm_backend` mod docs]
//! - `DirectToAnsi`: [`direct_to_ansi` mod docs]
//!
//! # You Are Here: **Stage 4** (Backend Converter/Shared)
//!
//! ```text
//! [Stage 1: App/Component]
//!   ↓
//! [Stage 2: Pipeline]
//!   ↓
//! [Stage 3: Compositor]
//!   ↓
//! [Stage 4: Backend Converter] ← YOU ARE HERE (shared by all backends)
//!   ↓
//! [Stage 5: Backend Executor]
//!   ↓
//! [Stage 6: Terminal]
//! ```
//!
//! **Input**: [`OfsBuf`] (rendered pixels from compositor)
//! **Output**: [`RenderOpOutputVec`] (optimized terminal operations)
//! **Role**: Convert [`OfsBuf`] to backend-agnostic rendering operations
//!
//! ## What This Stage Does
//!
//! The Backend Converter scans the [`OfsBuf`] and generates optimized
//! [`RenderOpOutputVec`] operations ready for terminal execution. It can:
//! - Perform diff calculations against the previous buffer for selective redraw
//! - Convert grid of styled characters to styled text painting operations
//! - Optimize by grouping adjacent operations with the same styling
//! - Handle state tracking via [`RenderOpsLocalData`]
//!
//! This stage is crucial for performance: by diffing buffers, only changed pixels are
//! rendered in subsequent frames, eliminating unnecessary terminal updates.
//!
//! # Type Safety Note
//!
//! This stage works with [`RenderOpOutputVec`] (post-Compositor operations), not
//! [`RenderOpIRVec`]. The Compositor has already applied all necessary transformations
//! (clipping, Unicode handling, etc.) when these methods are called.
//!
//! [`crossterm_backend` mod docs]: mod@crate::crossterm_backend
//! [`diff_chunks`]: mod@crate::ofs_buf::diff_chunks
//! [`direct_to_ansi` mod docs]: mod@crate::direct_to_ansi
//! [`ofs_buf/`]: mod@crate::ofs_buf
//! [`OfsBuf`]: crate::tui::OfsBuf
//! [`RenderOpIRVec`]: crate::tui::RenderOpIRVec
//! [`RenderOpOutputVec`]: crate::tui::RenderOpOutputVec
//! [`RenderOpsLocalData`]: crate::tui::RenderOpsLocalData
//! [rendering pipeline overview]: mod@crate::terminal_lib_backends#rendering-pipeline-architecture

// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.
use crate::{DEBUG_TUI_COMPOSITOR, DEBUG_TUI_SHOW_PIPELINE, FlushKind, GCStringOwned,
            InlineString, LockedOutputDevice, NarrowingCastToU16, OfsBuf, PixelChar,
            PixelCharDiffChunks, RenderOpCommon, RenderOpFlush, RenderOpOutput,
            RenderOpOutputVec, RenderOpsExec, TERMINAL_LIB_BACKEND, TerminalLibBackend,
            TuiStyle, VPCol, VPRow, VPSize,
            glyphs::SPACER_GLYPH,
            terminal_lib_backends::{crossterm_backend::PaintRenderOpImplCrossterm,
                                    direct_to_ansi::RenderOpPaintImplDirectToAnsi},
            vp_col, vp_pos, vp_row};

/// Paints the given rendering operations to the terminal using the active backend.
///
/// This function coordinates Stage 5 (Backend Executor) of the rendering pipeline,
/// executing the paint operations on the backend configured by [`TERMINAL_LIB_BACKEND`].
///
/// # Behavior
/// - **Screen Clearing:** If `flush_kind` is [`ClearBeforeFlush`], the screen is cleared
///   before any operations are executed.
/// - **Operation Execution:** Executes all operations in `render_ops` on the
///   `locked_output_device`.
/// - **Flushing:** Flushes the output device to synchronize the terminal display and
///   ensure all rendering changes are visible.
///
/// [`ClearBeforeFlush`]: FlushKind::ClearBeforeFlush
/// [`LockedOutputDevice`]: LockedOutputDevice
/// [`TERMINAL_LIB_BACKEND`]: crate::tui::TERMINAL_LIB_BACKEND
pub fn paint_ofs_buf(
    render_ops: RenderOpOutputVec,
    flush_kind: FlushKind,
    window_size: VPSize,
    locked_output_device: LockedOutputDevice<'_>,
) {
    match TERMINAL_LIB_BACKEND {
        TerminalLibBackend::Crossterm => {
            if let FlushKind::ClearBeforeFlush = flush_kind {
                PaintRenderOpImplCrossterm.clear_before_flush(locked_output_device);
            }
        }
        TerminalLibBackend::DirectToAnsi => {
            if let FlushKind::ClearBeforeFlush = flush_kind {
                RenderOpPaintImplDirectToAnsi.clear_before_flush(locked_output_device);
            }
        }
    }

    // Execute each RenderOpOutput using the ExecutableRenderOps trait.
    render_ops.execute_all(window_size, locked_output_device);

    // Flush everything to the terminal.
    match TERMINAL_LIB_BACKEND {
        TerminalLibBackend::Crossterm => {
            PaintRenderOpImplCrossterm.flush(locked_output_device);
        }
        TerminalLibBackend::DirectToAnsi => {
            RenderOpPaintImplDirectToAnsi.flush(locked_output_device);
        }
    }

    // Debug output.
    DEBUG_TUI_SHOW_PIPELINE.then(|| {
        // % is Display, ? is Debug.
        tracing::info!(
            message = "🎨 paint_ofs_buf ok 🟢",
            render_ops = ?render_ops
        );
    });
}

/// Paints the given differential rendering operations to the terminal using the active
/// backend.
///
/// This is the differential counterpart to [`paint_ofs_buf`], executing selective redraws
/// (Stage 5 of the rendering pipeline) on the backend configured by
/// [`TERMINAL_LIB_BACKEND`].
///
/// # Behavior
/// - **Selective Rendering:** Unlike a full paint, it does not support clearing the
///   screen before flushing, as it only writes the changed pixels to the terminal.
/// - **Operation Execution:** Executes all operations in `render_ops` on the
///   `locked_output_device`.
/// - **Flushing:** Flushes the output device to synchronize the terminal display and
///   ensure all rendering changes are visible.
///
/// [`LockedOutputDevice`]: LockedOutputDevice
/// [`paint_ofs_buf`]: fn@paint_ofs_buf
/// [`TERMINAL_LIB_BACKEND`]: crate::tui::TERMINAL_LIB_BACKEND
pub fn paint_ofs_buf_diff(
    render_ops: RenderOpOutputVec,
    window_size: VPSize,
    locked_output_device: LockedOutputDevice<'_>,
) {
    // Execute each RenderOpOutput using the ExecutableRenderOps trait.
    render_ops.execute_all(window_size, locked_output_device);

    // Flush everything to the terminal.
    match TERMINAL_LIB_BACKEND {
        TerminalLibBackend::Crossterm => {
            PaintRenderOpImplCrossterm.flush(locked_output_device);
        }
        TerminalLibBackend::DirectToAnsi => {
            RenderOpPaintImplDirectToAnsi.flush(locked_output_device);
        }
    }

    // Debug output.
    DEBUG_TUI_SHOW_PIPELINE.then(|| {
        // % is Display, ? is Debug.
        tracing::info!(
            message = "🎨 paint_diff_ofs_buf ok 🟢",
            render_ops = ?render_ops
        );
    });
}

/// Processes each [`PixelChar`] and generates a [`RenderOpOutput`] for it. Returns a
/// [`RenderOpOutputVec`] containing all the [`RenderOpOutput`]s.
///
/// This method is highly optimized to iterate over the [`Flat2DArray`] memory using
/// [`.chunks_exact()`], effectively creating a double loop over a single contiguous 1D
/// slice. This linear traversal dramatically improves cache locality while explicitly
/// eliminating the massive CPU pipeline stalls caused by division (`/`) and modulo (`%`)
/// math that would otherwise be required to calculate 2D coordinates from a single 1D
/// index.
///
/// See the [Rule of Thumb for 1D vs 2D Memory Iteration] for more details.
///
/// > Note that each [`PixelChar`] gets the full [`TuiStyle`] embedded in it (not just a
/// > part of it that is different than the previous char). This means that it is possible
/// > to quickly "diff" between 2 of them, since the [`TuiStyle`] is part of the
/// > [`PixelChar`]. This is important for selective re-rendering of the offscreen buffer.
///
/// Here's the algorithm used in this function using pseudo-code:
/// - When iterating linearly through the memory slice:
///   - If the [`PixelChar`] is [`Void`], [`Spacer`], or [`PlainText`] then handle
///     (display character, [`TuiStyle`])
///     - line buffer - accumulates over loop iterations.
///     - [`render_helper::flush_all_buffers()`] - flushes.
///   - Make sure to flush at the:
///     - End of line (calculated using chunk bounds).
///     - When style changes.
///
/// [`.chunks_exact()`]: slice::chunks_exact
/// [`Flat1DSimd`]: crate::core::Flat1DSimd
/// [`Flat2DArray`]: crate::core::Flat2DArray
/// [`PlainText`]: PixelChar::PlainText
/// [`RenderOpOutput`]: RenderOpOutput
/// [`RenderOpOutputVec`]: crate::tui::RenderOpOutputVec
/// [`Spacer`]: PixelChar::Spacer
/// [`TuiStyle`]: TuiStyle
/// [`Void`]: PixelChar::Void
/// [Rule of Thumb for 1D vs 2D Memory Iteration]:
///     crate::core::Flat1DSimd#rule-of-thumb-for-1d-vs-2d-memory-iteration
/// [SIMD]: https://en.wikipedia.org/wiki/SIMD
#[must_use]
pub fn render_ofs_buf(ofs_buf: &OfsBuf) -> RenderOpOutputVec {
    let mut context = render_helper::Context::default();

    let width = ofs_buf.get_width().as_usize();

    // Iterate over the contiguous SIMD 1D slice, explicitly chunked by row width.
    // This maintains the extreme cache locality of a 1D slice while completely
    // eliminating the CPU pipeline stalls caused by division (/) and modulo (%).
    for (row_idx, row_slice) in ofs_buf
        .as_simd()
        .as_raw_slice()
        .chunks_exact(width)
        .enumerate()
    {
        context.clear_for_new_line(vp_row((row_idx).as_u16_narrowing()));

        for (col_idx, pixel_char) in row_slice.iter().enumerate() {
            let (pixel_char_content, pixel_char_style): (String, Option<TuiStyle>) =
                match pixel_char {
                    PixelChar::Void => continue,
                    PixelChar::Spacer => (SPACER_GLYPH.to_string(), None),
                    PixelChar::PlainText {
                        display_char,
                        style,
                    } => (display_char.to_string(), Some(*style)),
                };

            let is_style_same_as_prev = render_helper::style_eq(
                pixel_char_style.as_ref(),
                context.prev_style.as_ref(),
            );
            let is_at_end_of_line = col_idx == width - 1;
            let is_first_loop_iteration = row_idx == 0 && col_idx == 0;

            // Deal w/: fg and bg colors | text attrib style | ANSI <-> PLAIN.
            // switchover.
            if !is_style_same_as_prev {
                // The style changed / render path has changed and something is.
                // already in the buffer, so flush it!
                render_helper::flush_all_buffers(&mut context);
            }

            // Deal w/: fg and bg colors | text attrib style.
            if is_first_loop_iteration || !is_style_same_as_prev {
                context.render_ops += RenderOpCommon::ResetColor;
                if let Some(style) = pixel_char_style
                    && let Some(color) = style.color_fg
                {
                    context.render_ops += RenderOpCommon::SetFgColor(color);
                }
                if let Some(style) = pixel_char_style
                    && let Some(color) = style.color_bg
                {
                    context.render_ops += RenderOpCommon::SetBgColor(color);
                }
                // Update prev_style.
                context.prev_style = pixel_char_style;
            }

            // Buffer it.
            context.buffer_plain_text.push_str(&pixel_char_content);

            // Flush it.
            if is_at_end_of_line {
                render_helper::flush_all_buffers(&mut context);
            }
        } // End for each pixel char in the row chunk.
    } // End for each row chunk in the contiguous buffer.

    // This handles the edge case when there is still something in the temp buffer,
    // but the loop has exited.
    if !context.buffer_plain_text.is_empty() {
        render_helper::flush_all_buffers(&mut context);
    }

    context.render_ops
}

/// Converts the given differential chunks ([`PixelCharDiffChunks`]) into optimized
/// rendering operations.
///
/// This is Stage 4 (Backend Converter) for selective redraws. It translates the changed
/// pixels between the current frame and the previous frame into sequential,
/// coordinate-targeted [`RenderOpOutput`] instructions:
///
/// - Moves the cursor to the absolute coordinates of the changed cell.
/// - Resets the terminal color state.
/// - Generates a print operation for the updated character and its corresponding
///   [`TuiStyle`].
/// - Skips [`PixelChar::Void`] pixels.
///
/// [`PixelChar::Void`]: PixelChar::Void
/// [`PixelCharDiffChunks`]: PixelCharDiffChunks
/// [`RenderOpOutput`]: RenderOpOutput
/// [`RenderOpOutputVec`]: crate::tui::RenderOpOutputVec
/// [`TuiStyle`]: TuiStyle
#[must_use]
pub fn render_ofs_buf_diff(diff_chunks: &PixelCharDiffChunks) -> RenderOpOutputVec {
    DEBUG_TUI_COMPOSITOR.then(|| {
        // % is Display, ? is Debug.
        tracing::info!(
            message = "🎨 ofs_buf_paint_impl_crossterm::render_ofs_buf_diff() ok 🟢",
            diff_chunks = ?diff_chunks
        );
    });

    let mut it = RenderOpOutputVec::new();

    for (position, pixel_char) in diff_chunks.iter() {
        it.push(RenderOpCommon::MoveCursorPositionAbs(*position));
        it.push(RenderOpCommon::ResetColor);
        match pixel_char {
            PixelChar::Void => { /* continue */ }
            PixelChar::Spacer => {
                it.push(
                    RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes(
                        SPACER_GLYPH.into(),
                        None,
                    ),
                );
            }
            PixelChar::PlainText {
                display_char,
                style,
                ..
            } => {
                it.push(RenderOpCommon::ApplyColors(Some(*style)));
                it.push(
                    RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes(
                        InlineString::from_str(&display_char.to_string()),
                        Some(*style),
                    ),
                );
            }
        }
    }

    it
}

pub mod render_helper {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    #[derive(Debug, Clone, Default)]
    pub struct Context {
        pub display_col_index_for_line: VPCol,
        pub display_row_index: VPRow,
        pub buffer_plain_text: InlineString,
        pub prev_style: Option<TuiStyle>,
        pub render_ops: RenderOpOutputVec,
    }

    impl Context {
        pub fn clear_for_new_line(&mut self, row_index: VPRow) {
            self.buffer_plain_text.clear();
            self.display_col_index_for_line = vp_col(0);
            self.display_row_index = row_index;
        }
    }

    /// `this` is eq to `other` if they are both `Some` and their following fields are eq:
    /// - `color_fg`
    /// - `color_bg`
    /// - `bold`
    /// - `dim`
    /// - `underline`
    /// - `reverse`
    /// - `hidden`
    /// - `strikethrough`
    #[must_use]
    pub fn style_eq(this: Option<&TuiStyle>, other: Option<&TuiStyle>) -> bool {
        match (this, other) {
            (Some(this), Some(other)) => {
                this.color_fg == other.color_fg
                    && this.color_bg == other.color_bg
                    && this.attribs.bold == other.attribs.bold
                    && this.attribs.dim == other.attribs.dim
                    && this.attribs.underline == other.attribs.underline
                    && this.attribs.reverse == other.attribs.reverse
                    && this.attribs.hidden == other.attribs.hidden
                    && this.attribs.strikethrough == other.attribs.strikethrough
            }
            (None, None) => true,
            _ => false,
        }
    }

    pub fn flush_all_buffers(context: &mut Context) {
        if !context.buffer_plain_text.is_empty() {
            render_helper::flush_plain_text_line_buffer(context);
        }
    }

    pub fn flush_plain_text_line_buffer(context: &mut Context) {
        // Generate `RenderOpOutput` operations for each `PixelChar` and add it to
        // `render_ops`.
        let pos = vp_pos(
            context.display_col_index_for_line,
            context.display_row_index,
        );

        // Deal w/ position.
        context.render_ops += RenderOpCommon::MoveCursorPositionAbs(pos);

        // Deal w/ style attribs & actually paint the `temp_line_buffer`.
        context.render_ops +=
            RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes(
                context.buffer_plain_text.clone(),
                context.prev_style,
            );

        // Update `display_col_index_for_line`.
        let display_width = GCStringOwned::from(&context.buffer_plain_text).width();
        context.display_col_index_for_line += display_width;

        // Clear the buffer!
        context.buffer_plain_text.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::{render_helper::style_eq, *};
    use crate::{Flat2DArray, RenderOpsLocalData, VPWidth, assert_eq2,
                compositor_render_ops_to_ofs_buf::print_text_with_attributes, new_style,
                tui_color, vp_col, vp_height, vp_row, vp_width};

    /// Helper function to make an `OfsBuf`.
    fn make_ofs_buf_plain_text() -> OfsBuf {
        let window_size = vp_width(10) + vp_height(2);
        let mut ofs_buf =
            OfsBuf::new(Flat2DArray::new_empty(window_size, PixelChar::Spacer));

        // Input:  R0 "hello1234😃"
        //            C0123456789
        // Output: R0 "hello1234╳"
        //            C0123456789
        let text = "hello1234😃";
        // The style colors should be overwritten by fg_color and bg_color.
        let maybe_style = Some(
            new_style!(dim bold color_fg:{tui_color!(cyan)} color_bg:{tui_color!(cyan)}),
        );
        ofs_buf.set_cursor_pos(vp_col(0) + vp_row(0));
        let render_local_data = RenderOpsLocalData {
            fg_color: Some(tui_color!(green)),
            bg_color: Some(tui_color!(blue)),
            ..Default::default()
        };
        let maybe_max_display_col_count: Option<VPWidth> = Some(vp_width(10));
        print_text_with_attributes(
            text,
            maybe_style.as_ref(),
            &mut ofs_buf,
            maybe_max_display_col_count,
            &render_local_data,
        )
        .ok();
        ofs_buf

        // Output:
        // my_ofs_buf:
        // window_size: [width:10, height:2],
        // row_index: [0]
        //   0: "h" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) |
        // padding: 0 })   1: "e" Some(Style { _id + bold + dim | fg: Some(green)
        // | bg: Some(blue) | padding: 0 })   2: "l" Some(Style { _id + bold + dim
        // | fg: Some(green) | bg: Some(blue) | padding: 0 })   3: "l" Some(Style
        // { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //   4: "o" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) |
        // padding: 0 })   5: "1" Some(Style { _id + bold + dim | fg: Some(green)
        // | bg: Some(blue) | padding: 0 })   6: "2" Some(Style { _id + bold + dim
        // | fg: Some(green) | bg: Some(blue) | padding: 0 })   7: "3" Some(Style
        // { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //   8: "4" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) |
        // padding: 0 })   9: ╳
        // row_index: [1]
        //   0: ╳ ..
        //   9: ╳
    }

    #[test]
    fn test_render_plain_text() {
        let my_ofs_buf = make_ofs_buf_plain_text();
        // println!("my_ofs_buf: \n{:#?}", my_ofs_buf);
        let render_ops = render_ofs_buf(&my_ofs_buf);
        // println!("render_ops: {:#?}", render_ops);

        // Output:
        // render_ops:
        // - RenderOpOutputVec.len(): 10
        // - [ResetColor]
        // - [SetFgColor(green)]
        // - [SetBgColor(blue)]
        // - [MoveCursorPositionAbs([col:0, row:0])]
        // - [PrintTextWithAttributes(9 bytes, Style { _id + bold + dim | fg: Some(green)
        //   | bg: Some(blue) | padding: 0 })]
        // - [ResetColor]
        // - [MoveCursorPositionAbs([col:9, row:0])]
        // - [PrintTextWithAttributes(1 bytes, None)]
        // - [MoveCursorPositionAbs([col:0, row:1])]
        // - [PrintTextWithAttributes(10 bytes, None)]

        assert_eq2!(render_ops.len(), 10);
        assert_eq2!(
            render_ops[0],
            RenderOpOutput::Common(RenderOpCommon::ResetColor)
        );
        assert_eq2!(
            render_ops[1],
            RenderOpOutput::Common(RenderOpCommon::SetFgColor(tui_color!(green)))
        );
        assert_eq2!(
            render_ops[2],
            RenderOpOutput::Common(RenderOpCommon::SetBgColor(tui_color!(blue)))
        );
        assert_eq2!(
            render_ops[3],
            RenderOpOutput::Common(RenderOpCommon::MoveCursorPositionAbs(
                vp_col(0) + vp_row(0)
            ))
        );
        assert_eq2!(
            render_ops[4],
            RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes(
                "hello1234".into(),
                Some(
                    new_style!(dim bold color_fg:{tui_color!(green)} color_bg:{tui_color!(blue)})
                )
            )
        );
        assert_eq2!(
            render_ops[5],
            RenderOpOutput::Common(RenderOpCommon::ResetColor)
        );
        assert_eq2!(
            render_ops[6],
            RenderOpOutput::Common(RenderOpCommon::MoveCursorPositionAbs(
                vp_col(9) + vp_row(0)
            ))
        );
        assert_eq2!(
            render_ops[7],
            RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes(
                SPACER_GLYPH.into(),
                None
            )
        );
        assert_eq2!(
            render_ops[8],
            RenderOpOutput::Common(RenderOpCommon::MoveCursorPositionAbs(
                vp_col(0) + vp_row(1)
            ))
        );
        assert_eq2!(
            render_ops[9],
            RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes(
                (SPACER_GLYPH.repeat(10)).into(),
                None
            )
        );
    }

    #[test]
    fn test_render_helper_style_eq() {
        let style1 = Some(
            new_style!(dim bold color_fg:{tui_color!(cyan)} color_bg:{tui_color!(cyan)}),
        );
        let style2 = Some(
            new_style!(dim bold color_fg:{tui_color!(cyan)} color_bg:{tui_color!(cyan)}),
        );

        assert_eq2!(style_eq(style1.as_ref(), style2.as_ref()), true);

        let style_3 = Some(
            new_style!(italic color_fg:{tui_color!(black)} color_bg:{tui_color!(cyan)}),
        );

        assert_eq2!(style_eq(style1.as_ref(), style_3.as_ref()), false);
    }
}
