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
use r3bl_core::{call_if_true,
                ch,
                col,
                glyphs::SPACER_GLYPH,
                row,
                ColIndex,
                LockedOutputDevice,
                RowIndex,
                Size,
                StringStorage,
                TuiStyle,
                UnicodeString};

use crate::{diff_chunks::PixelCharDiffChunks,
            render_ops,
            Flush as _,
            FlushKind,
            OffscreenBuffer,
            OffscreenBufferPaint,
            PixelChar,
            RenderOp,
            RenderOps,
            DEBUG_TUI_COMPOSITOR,
            DEBUG_TUI_SHOW_PIPELINE};

pub struct OffscreenBufferPaintImplCrossterm;

impl OffscreenBufferPaint for OffscreenBufferPaintImplCrossterm {
    fn paint(
        &mut self,
        render_ops: RenderOps,
        flush_kind: FlushKind,
        window_size: Size,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) {
        let mut skip_flush = false;

        if let FlushKind::ClearBeforeFlush = flush_kind {
            RenderOp::default().clear_before_flush(locked_output_device);
        }

        // Execute each RenderOp.
        render_ops.execute_all(
            &mut skip_flush,
            window_size,
            locked_output_device,
            is_mock,
        );

        // Flush everything to the terminal.
        if !skip_flush {
            RenderOp::default().flush(locked_output_device)
        };

        // Debug output.
        call_if_true!(DEBUG_TUI_SHOW_PIPELINE, {
            // % is Display, ? is Debug.
            tracing::info!(
                message = "🎨 offscreen_buffer_paint_impl_crossterm::paint() ok 🟢",
                render_ops = ?render_ops
            );
        });
    }

    fn paint_diff(
        &mut self,
        render_ops: RenderOps,
        window_size: Size,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) {
        let mut skip_flush = false;

        // Execute each RenderOp.
        render_ops.execute_all(
            &mut skip_flush,
            window_size,
            locked_output_device,
            is_mock,
        );

        // Flush everything to the terminal.
        if !skip_flush {
            RenderOp::default().flush(locked_output_device)
        };

        // Debug output.
        call_if_true!(DEBUG_TUI_SHOW_PIPELINE, {
            // % is Display, ? is Debug.
            tracing::info!(
                message = "🎨 offscreen_buffer_paint_impl_crossterm::paint_diff() ok 🟢",
                render_ops = ?render_ops
            );
        });
    }

    /// Process each [PixelChar] in [OffscreenBuffer] and generate a [RenderOp] for it.
    /// Return a [RenderOps] containing all the [RenderOp]s.
    ///
    /// > Note that each [PixelChar] gets the full [TuiStyle] embedded in it (not just a
    /// > part of it that is different than the previous char). This means that it is
    /// > possible to quickly "diff" between 2 of them, since the [TuiStyle] is part of
    /// > the [PixelChar]. This is important for selective re-rendering of the
    /// > [OffscreenBuffer].
    ///
    /// Here's the algorithm used in this function using pseudo-code:
    /// - When going thru every `PixelChar` in a line:
    ///   - If the `PixelChar` is `Void`, `Spacer`, or `PlainText` then handle
    ///     (pixel_char_str, pixel_char_style)
    ///     - `temp_line_buffer`: accumulates over loop iterations.
    ///     - `flush_temp_line_buffer()`: flushes.
    ///   - Make sure to flush at the:
    ///     - End of line.
    ///     - When style changes.
    fn render(&mut self, offscreen_buffer: &OffscreenBuffer) -> RenderOps {
        use render_helpers::*;

        let mut context = Context::new();

        // For each line in the offscreen buffer.
        for (row_index, line) in offscreen_buffer.buffer.iter().enumerate() {
            context.clear_for_new_line(row(row_index));

            // For each pixel char in the line.
            for (pixel_char_index, pixel_char) in line.iter().enumerate() {
                let (pixel_char_str, pixel_char_style): (&str, Option<TuiStyle>) =
                    match pixel_char {
                        PixelChar::Void => continue,
                        PixelChar::Spacer => (SPACER_GLYPH, None),
                        PixelChar::PlainText { text, maybe_style } => {
                            (text, *maybe_style)
                        }
                    };

                let is_style_same_as_prev =
                    render_helpers::style_eq(&pixel_char_style, &context.prev_style);
                let is_at_end_of_line = ch(pixel_char_index) == (ch(line.len()) - ch(1));
                let is_first_loop_iteration = row_index == 0 && pixel_char_index == 0;

                // Deal w/: fg and bg colors | text attrib style | ANSI <-> PLAIN switchover.
                if !is_style_same_as_prev {
                    // The style changed / render path has changed and something is already in the
                    // buffer, so flush it!
                    render_helpers::flush_all_buffers(&mut context);
                }

                // Deal w/: fg and bg colors | text attrib style
                if is_first_loop_iteration || !is_style_same_as_prev {
                    context.render_ops.push(RenderOp::ResetColor);
                    if let Some(style) = pixel_char_style {
                        if let Some(color) = style.color_fg {
                            context.render_ops.push(RenderOp::SetFgColor(color));
                        }
                    }
                    if let Some(style) = pixel_char_style {
                        if let Some(color) = style.color_bg {
                            context.render_ops.push(RenderOp::SetBgColor(color));
                        }
                    }
                    // Update prev_style.
                    context.prev_style = pixel_char_style;
                }

                // Buffer it.
                context.buffer_plain_text.push_str(pixel_char_str);

                // Flush it.
                if is_at_end_of_line {
                    render_helpers::flush_all_buffers(&mut context);
                }
            } // End for each pixel char in the line.
        } // End for each line in the offscreen buffer.

        // This handles the edge case when there is still something in the temp buffer, but the loop
        // has exited.
        if !context.buffer_plain_text.is_empty() {
            render_helpers::flush_all_buffers(&mut context);
        }

        context.render_ops
    }

    fn render_diff(&mut self, diff_chunks: &PixelCharDiffChunks) -> RenderOps {
        call_if_true!(DEBUG_TUI_COMPOSITOR, {
            // % is Display, ? is Debug.
            tracing::info!(
                message = "🎨 offscreen_buffer_paint_impl_crossterm::render_diff() ok 🟢",
                diff_chunks = ?diff_chunks
            );
        });

        let mut it = render_ops!();

        for (position, pixel_char) in diff_chunks.iter() {
            it.push(RenderOp::MoveCursorPositionAbs(*position));
            it.push(RenderOp::ResetColor);
            match pixel_char {
                PixelChar::Void => continue,
                PixelChar::Spacer => {
                    it.push(RenderOp::CompositorNoClipTruncPaintTextWithAttributes(
                        SPACER_GLYPH.into(),
                        None,
                    ))
                }
                PixelChar::PlainText {
                    text, maybe_style, ..
                } => {
                    it.push(RenderOp::ApplyColors(*maybe_style));
                    it.push(RenderOp::CompositorNoClipTruncPaintTextWithAttributes(
                        // PERF: [ ] perf
                        StringStorage::from_str(text),
                        *maybe_style,
                    ))
                }
            }
        }

        it
    }
}

mod render_helpers {
    use super::*;

    #[derive(Debug, Clone)]
    pub struct Context {
        pub display_col_index_for_line: ColIndex,
        pub display_row_index: RowIndex,
        // PERF: [ ] remove String
        pub buffer_plain_text: StringStorage,
        pub prev_style: Option<TuiStyle>,
        pub render_ops: RenderOps,
    }

    impl Context {
        pub fn new() -> Self {
            Context {
                display_col_index_for_line: col(0),
                buffer_plain_text: StringStorage::new(),
                render_ops: render_ops!(),
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
    pub fn style_eq(this: &Option<TuiStyle>, other: &Option<TuiStyle>) -> bool {
        // REVIEW: [ ] replace use of bool w/ enum
        match (this.is_some(), other.is_some()) {
            (false, false) => true,
            (true, true) => {
                let this = (*this).unwrap();
                let other = (*other).unwrap();
                this.color_fg == other.color_fg
                    && this.color_bg == other.color_bg
                    && this.bold == other.bold
                    && this.dim == other.dim
                    && this.underline == other.underline
                    && this.reverse == other.reverse
                    && this.hidden == other.hidden
                    && this.strikethrough == other.strikethrough
            }
            (_, _) => false,
        }
    }

    pub fn flush_all_buffers(context: &mut Context) {
        if !context.buffer_plain_text.is_empty() {
            render_helpers::flush_plain_text_line_buffer(context);
        }
    }

    pub fn flush_plain_text_line_buffer(context: &mut Context) {
        // Generate `RenderOps` for each `PixelChar` and add it to `render_ops`.
        let pos = context.display_col_index_for_line + context.display_row_index;

        // Deal w/ position.
        context
            .render_ops
            .push(RenderOp::MoveCursorPositionAbs(pos));

        // Deal w/ style attribs & actually paint the `temp_line_buffer`.
        context
            .render_ops
            .push(RenderOp::CompositorNoClipTruncPaintTextWithAttributes(
                // PERF: [ ] remove String clone
                context.buffer_plain_text.clone(),
                context.prev_style,
            ));

        // Update `display_col_index_for_line`.
        let display_width = UnicodeString::str_display_width(&context.buffer_plain_text);
        *context.display_col_index_for_line += *display_width;

        // Clear the buffer!
        context.buffer_plain_text.clear()
    }
}

#[cfg(test)]
mod tests {
    use r3bl_core::{assert_eq2, color, height, width, ColWidth};
    use r3bl_macro::tui_style;

    use super::*;
    use crate::render_pipeline_to_offscreen_buffer::print_text_with_attributes;

    /// Helper function to make an `OffscreenBuffer`.
    fn make_offscreen_buffer_plain_text() -> OffscreenBuffer {
        let window_size = width(10) + height(2);
        let mut my_offscreen_buffer =
            OffscreenBuffer::new_with_capacity_initialized(window_size);

        // Input:  R0 "hello1234😃"
        //            C0123456789
        // Output: R0 "hello1234╳"
        //            C0123456789
        let text = "hello1234😃";
        // The style colors should be overwritten by fg_color and bg_color.
        let maybe_style = Some(
            tui_style! { attrib: [dim, bold] color_fg: color!(@cyan) color_bg: color!(@cyan) },
        );
        my_offscreen_buffer.my_pos = col(0) + row(0);
        my_offscreen_buffer.my_fg_color = Some(color!(@green));
        my_offscreen_buffer.my_bg_color = Some(color!(@blue));
        let maybe_max_display_col_count: Option<ColWidth> = Some(width(10));
        print_text_with_attributes(
            text,
            &maybe_style,
            &mut my_offscreen_buffer,
            maybe_max_display_col_count,
        )
        .ok();
        my_offscreen_buffer

        // Output:
        // my_offscreen_buffer:
        // window_size: [width:10, height:2],
        // row_index: [0]
        //   0: "h" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //   1: "e" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //   2: "l" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //   3: "l" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //   4: "o" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //   5: "1" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //   6: "2" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //   7: "3" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //   8: "4" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //   9: ╳
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
        // - [PrintTextWithAttributes(9 bytes, Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })]
        // - [ResetColor]
        // - [MoveCursorPositionAbs([col:9, row:0])]
        // - [PrintTextWithAttributes(1 bytes, None)]
        // - [MoveCursorPositionAbs([col:0, row:1])]
        // - [PrintTextWithAttributes(10 bytes, None)]

        assert_eq2!(render_ops.len(), 10);
        assert_eq2!(render_ops[0], RenderOp::ResetColor);
        assert_eq2!(render_ops[1], RenderOp::SetFgColor(color!(@green)));
        assert_eq2!(render_ops[2], RenderOp::SetBgColor(color!(@blue)));
        assert_eq2!(
            render_ops[3],
            RenderOp::MoveCursorPositionAbs(col(0) + row(0))
        );
        assert_eq2!(
            render_ops[4],
            RenderOp::CompositorNoClipTruncPaintTextWithAttributes(
                "hello1234".into(),
                Some(
                    tui_style! { attrib: [dim, bold] color_fg: color!(@green) color_bg: color!(@blue) }
                )
            )
        );
        assert_eq2!(render_ops[5], RenderOp::ResetColor);
        assert_eq2!(
            render_ops[6],
            RenderOp::MoveCursorPositionAbs(col(9) + row(0))
        );
        assert_eq2!(
            render_ops[7],
            RenderOp::CompositorNoClipTruncPaintTextWithAttributes(
                SPACER_GLYPH.into(),
                None
            )
        );
        assert_eq2!(
            render_ops[8],
            RenderOp::MoveCursorPositionAbs(col(0) + row(1))
        );
        assert_eq2!(
            render_ops[9],
            RenderOp::CompositorNoClipTruncPaintTextWithAttributes(
                (SPACER_GLYPH.repeat(10)).into(),
                None
            )
        );
    }
}
