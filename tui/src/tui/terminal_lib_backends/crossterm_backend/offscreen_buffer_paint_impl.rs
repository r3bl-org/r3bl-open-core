// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # Pipeline Stage 4: Backend Converter
//!
//! # You Are Here
//!
//! ```text
//! [S1: App/Component] → [S2: Pipeline] → [S3: Compositor] →
//! [S4: Backend Converter] ← YOU ARE HERE
//! [S5: Backend Executor] → [S6: Terminal]
//! ```
//!
//! **Input**: [`OffscreenBuffer`] (rendered pixels from compositor)
//! **Output**: [`RenderOpOutputVec`] (optimized terminal operations)
//! **Role**: Convert [`OffscreenBuffer`] to backend-specific rendering operations
//!
//! > **For the complete rendering architecture**, see [`super::super`] (parent parent
//! > module).
//!
//! ## What This Stage Does
//!
//! The Backend Converter scans the [`OffscreenBuffer`] and generates optimized
//! [`RenderOpOutputVec`] operations ready for terminal execution. It can:
//! - Perform diff calculations against the previous buffer for selective redraw
//! - Convert grid of styled characters to styled text painting operations
//! - Optimize by grouping adjacent operations with the same styling
//! - Handle backend-specific optimizations (e.g., state tracking via
//!   [`RenderOpsLocalData`])
//!
//! This stage is crucial for performance: by diffing buffers, only changed pixels are
//! rendered in subsequent frames, eliminating unnecessary terminal updates.
//!
//! [`OffscreenBuffer`]: crate::OffscreenBuffer
//! [`RenderOpOutputVec`]: crate::RenderOpOutputVec
//! [`RenderOpsLocalData`]: crate::RenderOpsLocalData

// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.
use crate::{ColIndex, DEBUG_TUI_COMPOSITOR, DEBUG_TUI_SHOW_PIPELINE, FlushKind,
            GCStringOwned, InlineString, LockedOutputDevice, OffscreenBuffer,
            OffscreenBufferPaint, PaintRenderOpImplCrossterm, PixelChar, RenderOpCommon,
            RenderOpFlush, RenderOpOutput, RenderOpOutputVec, RenderOpsExec, RowIndex,
            Size, TuiStyle, ch, col, diff_chunks::PixelCharDiffChunks,
            glyphs::SPACER_GLYPH, row};

#[derive(Debug)]
pub struct OffscreenBufferPaintImplCrossterm;

impl OffscreenBufferPaint for OffscreenBufferPaintImplCrossterm {
    fn paint(
        &mut self,
        render_ops: RenderOpOutputVec,
        flush_kind: FlushKind,
        window_size: Size,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) {
        let mut skip_flush = false;

        if let FlushKind::ClearBeforeFlush = flush_kind {
            PaintRenderOpImplCrossterm.clear_before_flush(locked_output_device);
        }

        // Execute each RenderOp using the ExecutableRenderOps trait.
        render_ops.execute_all(
            &mut skip_flush,
            window_size,
            locked_output_device,
            is_mock,
        );

        // Flush everything to the terminal.
        if !skip_flush {
            PaintRenderOpImplCrossterm.flush(locked_output_device);
        }

        // Debug output.
        DEBUG_TUI_SHOW_PIPELINE.then(|| {
            // % is Display, ? is Debug.
            tracing::info!(
                message = "🎨 offscreen_buffer_paint_impl_crossterm::paint() ok 🟢",
                render_ops = ?render_ops
            );
        });
    }

    fn paint_diff(
        &mut self,
        render_ops: RenderOpOutputVec,
        window_size: Size,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) {
        let mut skip_flush = false;

        // Execute each RenderOp using the ExecutableRenderOps trait.
        render_ops.execute_all(
            &mut skip_flush,
            window_size,
            locked_output_device,
            is_mock,
        );

        // Flush everything to the terminal.
        if !skip_flush {
            PaintRenderOpImplCrossterm.flush(locked_output_device);
        }

        // Debug output.
        DEBUG_TUI_SHOW_PIPELINE.then(|| {
            // % is Display, ? is Debug.
            tracing::info!(
                message = "🎨 offscreen_buffer_paint_impl_crossterm::paint_diff() ok 🟢",
                render_ops = ?render_ops
            );
        });
    }

    /// Process each [`PixelChar`] and generate a [`RenderOpOutput`]
    /// for it. Return a [`RenderOpOutputVec`] containing all the [`RenderOpOutput`]s.
    ///
    /// > Note that each [`PixelChar`] gets the full [`TuiStyle`] embedded in it (not just
    /// > a part of it that is different than the previous char). This means that it is
    /// > possible to quickly "diff" between 2 of them, since the [`TuiStyle`] is part of
    /// > the [`PixelChar`]. This is important for selective re-rendering of the
    /// > offscreen buffer.
    ///
    /// Here's the algorithm used in this function using pseudo-code:
    /// - When going thru every [`PixelChar`] in a line:
    ///   - If the [`PixelChar`] is [`Void`], [`Spacer`], or [`PlainText`] then handle
    ///     (display character, [`TuiStyle`])
    ///     - line buffer -  accumulates over loop iterations.
    ///     - `render_helper::flush_all_buffers()` - flushes.
    ///   - Make sure to flush at the:
    ///     - End of line.
    ///     - When style changes.
    ///
    /// [`RenderOpOutput`]: crate::RenderOpOutput
    /// [`RenderOpOutputVec`]: crate::RenderOpOutputVec
    /// [`TuiStyle`]: crate::TuiStyle
    /// [`Void`]: PixelChar::Void
    /// [`Spacer`]: PixelChar::Spacer
    /// [`PlainText`]: PixelChar::PlainText
    fn render(&mut self, ofs_buf: &OffscreenBuffer) -> RenderOpOutputVec {
        use render_helper::Context;

        let mut context = Context::new();

        // For each line in the offscreen buffer.
        for (row_index, line) in ofs_buf.buffer.iter().enumerate() {
            context.clear_for_new_line(row(row_index));

            // For each pixel char in the line.
            for (pixel_char_index, pixel_char) in line.iter().enumerate() {
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
                let is_at_end_of_line = ch(pixel_char_index) == (ch(line.len()) - ch(1));
                let is_first_loop_iteration = row_index == 0 && pixel_char_index == 0;

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
            } // End for each pixel char in the line.
        } // End for each line in the offscreen buffer.

        // This handles the edge case when there is still something in the temp buffer,
        // but the loop has exited.
        if !context.buffer_plain_text.is_empty() {
            render_helper::flush_all_buffers(&mut context);
        }

        context.render_ops
    }

    fn render_diff(&mut self, diff_chunks: &PixelCharDiffChunks) -> RenderOpOutputVec {
        DEBUG_TUI_COMPOSITOR.then(|| {
            // % is Display, ? is Debug.
            tracing::info!(
                message = "🎨 offscreen_buffer_paint_impl_crossterm::render_diff() ok 🟢",
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
}

mod render_helper {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    #[derive(Debug, Clone)]
    pub struct Context {
        pub display_col_index_for_line: ColIndex,
        pub display_row_index: RowIndex,
        pub buffer_plain_text: InlineString,
        pub prev_style: Option<TuiStyle>,
        pub render_ops: RenderOpOutputVec,
    }

    impl Context {
        pub fn new() -> Self {
            Context {
                display_col_index_for_line: col(0),
                buffer_plain_text: InlineString::new(),
                render_ops: RenderOpOutputVec::new(),
                display_row_index: row(0),
                prev_style: None,
            }
        }

        pub fn clear_for_new_line(&mut self, row_index: RowIndex) {
            self.buffer_plain_text.clear();
            self.display_col_index_for_line = col(0);
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
        // Generate `RenderOpsOutput` for each `PixelChar` and add it to `render_ops`.
        let pos = context.display_col_index_for_line + context.display_row_index;

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
        *context.display_col_index_for_line += *display_width;

        // Clear the buffer!
        context.buffer_plain_text.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::{render_helper::style_eq, *};
    use crate::{ColWidth, RenderOpsLocalData, assert_eq2,
                compositor_render_ops_to_ofs_buf::print_text_with_attributes, height,
                new_style, tui_color, width};

    /// Helper function to make an `OffscreenBuffer`.
    fn make_offscreen_buffer_plain_text() -> OffscreenBuffer {
        let window_size = width(10) + height(2);
        let mut ofs_buf = OffscreenBuffer::new_empty(window_size);

        // Input:  R0 "hello1234😃"
        //            C0123456789
        // Output: R0 "hello1234╳"
        //            C0123456789
        let text = "hello1234😃";
        // The style colors should be overwritten by fg_color and bg_color.
        let maybe_style = Some(
            new_style!(dim bold color_fg:{tui_color!(cyan)} color_bg:{tui_color!(cyan)}),
        );
        ofs_buf.cursor_pos = col(0) + row(0);
        let render_local_data = RenderOpsLocalData {
            fg_color: Some(tui_color!(green)),
            bg_color: Some(tui_color!(blue)),
            ..Default::default()
        };
        let maybe_max_display_col_count: Option<ColWidth> = Some(width(10));
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
        // my_offscreen_buffer:
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
        let my_offscreen_buffer = make_offscreen_buffer_plain_text();
        // println!("my_offscreen_buffer: \n{:#?}", my_offscreen_buffer);
        let mut paint = OffscreenBufferPaintImplCrossterm {};
        let render_ops = paint.render(&my_offscreen_buffer);
        // println!("render_ops: {:#?}", render_ops);

        // Output:
        // render_ops:
        // - RenderOps.len(): 10
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
                col(0) + row(0)
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
                col(9) + row(0)
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
                col(0) + row(1)
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
