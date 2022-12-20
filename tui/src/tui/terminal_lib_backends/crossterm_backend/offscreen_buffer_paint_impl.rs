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

use crate::*;

pub struct OffscreenBufferPaintImplCrossterm;

#[async_trait]
impl OffscreenBufferPaint for OffscreenBufferPaintImplCrossterm {
    async fn paint(
        &mut self,
        render_ops: RenderOps,
        flush_kind: FlushKind,
        shared_global_data: &SharedGlobalData,
    ) {
        let mut skip_flush = false;

        if let FlushKind::ClearBeforeFlush = flush_kind {
            RenderOp::default().clear_before_flush();
        }

        // Execute each RenderOp.
        render_ops
            .execute_all(&mut skip_flush, shared_global_data)
            .await;

        // Flush everything to the terminal.
        if !skip_flush {
            RenderOp::default().flush()
        };

        // Debug output.
        call_if_true!(DEBUG_TUI_SHOW_PIPELINE, {
            let msg = format!(
                "🎨 offscreen_buffer_paint_impl_crossterm::paint() ok ✅: render_ops: \n{render_ops:?}",
            );
            log_info(msg);
        });
    }

    async fn paint_diff(&mut self, render_ops: RenderOps, shared_global_data: &SharedGlobalData) {
        let mut skip_flush = false;

        // Execute each RenderOp.
        render_ops
            .execute_all(&mut skip_flush, shared_global_data)
            .await;

        // Flush everything to the terminal.
        if !skip_flush {
            RenderOp::default().flush()
        };

        // Debug output.
        call_if_true!(DEBUG_TUI_SHOW_PIPELINE, {
            let msg = format!(
                "🎨 offscreen_buffer_paint_impl_crossterm::paint() ok ✅: render_ops: \n{render_ops:?}"
            );
            log_info(msg);
        });
    }

    /// Process each [PixelChar] in [OffscreenBuffer] and generate a [RenderOp] for it. Return a
    /// [RenderOps] containing all the [RenderOp]s.
    ///
    /// > Note that each [PixelChar] gets the full [Style] embedded in it (not just a part of it
    /// > that is different than the previous char). This means that it is possible to quickly
    /// > "diff" between 2 of them, since the [Style] is part of the [PixelChar]. This is important
    /// > for selective re-rendering of the [OffscreenBuffer].
    ///
    /// Here's the algorithm used in this function using pseudo-code:
    /// - When going thru every `PixelChar` in a line:
    ///   - if the `PixelChar` is `Void`, `Spacer`, or `PlainText` then handle it like now
    ///     - `temp_line_buffer`: accumulates over loop iterations
    ///     - `flush_temp_line_buffer()`: flushes
    ///   - if the `PixelChar` is `AnsiText`
    ///     - `temp_ansi_line_buffer`: accumulates over loop iterations
    ///     - `flush_temp_ansi_line_buffer()`: flushes
    ///   - make sure to flush at the
    ///     - end of line
    ///     - when style changes
    ///     - when switchover from ANSI <-> PLAIN happens
    async fn render(&mut self, offscreen_buffer: &OffscreenBuffer) -> RenderOps {
        use render_helpers::*;

        let mut context = Context::new();

        // For each line in the offscreen buffer.
        for (row_index, line) in offscreen_buffer.buffer.iter().enumerate() {
            context.clear_for_new_line(row_index);

            // For each pixel char in the line.
            for (pixel_char_index, pixel_char) in line.iter().enumerate() {
                let (pixel_char_str, pixel_char_style, pixel_char_render_path): (
                    &str,
                    Option<Style>,
                    RenderPath,
                ) = match pixel_char {
                    PixelChar::Void => continue,
                    PixelChar::Spacer => (SPACER, None, RenderPath::PlainText),
                    PixelChar::PlainText {
                        content,
                        maybe_style,
                    } => (&content.string, maybe_style.clone(), RenderPath::PlainText),
                    PixelChar::AnsiText { content } => (content, None, RenderPath::AnsiText),
                };

                let is_style_same_as_prev =
                    render_helpers::style_eq(&pixel_char_style, &context.prev_style);
                let is_at_end_of_line = ch!(pixel_char_index) == (ch!(line.len() - 1));
                let is_first_loop_iteration = row_index == 0 && pixel_char_index == 0;
                let is_render_path_same_as_prev =
                    context.prev_render_path == Some(pixel_char_render_path);

                // Deal w/: fg and bg colors | text attrib style | ANSI <-> PLAIN switchover.
                if !is_style_same_as_prev || !is_render_path_same_as_prev {
                    // The style changed / render path has changed and something is already in the
                    // buffer, so flush it!
                    render_helpers::flush_all_buffers(&mut context);
                }

                // Deal w/: fg and bg colors | text attrib style
                if is_first_loop_iteration || !is_style_same_as_prev {
                    context.render_ops.push(RenderOp::ResetColor);
                    if let Some(style) = pixel_char_style.clone() {
                        if let Some(color) = style.color_fg {
                            context.render_ops.push(RenderOp::SetFgColor(color));
                        }
                    }
                    if let Some(style) = pixel_char_style.clone() {
                        if let Some(color) = style.color_bg {
                            context.render_ops.push(RenderOp::SetBgColor(color));
                        }
                    }
                    // Update prev_style.
                    context.prev_style = pixel_char_style.clone();
                }

                // Buffer it.
                match pixel_char_render_path {
                    RenderPath::PlainText => {
                        context.buffer_plain_text.push_str(pixel_char_str);
                    }
                    RenderPath::AnsiText => {
                        context.buffer_ansi_text.push_str(pixel_char_str);
                    }
                }

                // Flush it.
                if is_at_end_of_line {
                    render_helpers::flush_all_buffers(&mut context);
                }

                // Update prev_render_path.
                context.prev_render_path = Some(pixel_char_render_path);
            } // End for each pixel char in the line.
        } // End for each line in the offscreen buffer.

        // This handles the edge case when there is still something in the temp buffer, but the loop
        // has exited.
        if !context.buffer_plain_text.is_empty() || !context.buffer_ansi_text.is_empty() {
            render_helpers::flush_all_buffers(&mut context);
        }

        context.render_ops
    }

    async fn render_diff(&mut self, diff_chunks: &PixelCharDiffChunks) -> RenderOps {
        call_if_true!(DEBUG_TUI_COMPOSITOR, {
            let msg = format!("🎨 offscreen_buffer_paint_impl_crossterm::render_diff() ok ✅: \ndiff_chunks: \n{}",
            diff_chunks.pretty_print());
            log_info(msg);
        });

        let mut it = render_ops!();

        for (position, pixel_char) in diff_chunks.iter() {
            it.push(RenderOp::MoveCursorPositionAbs(*position));
            it.push(RenderOp::ResetColor);
            match pixel_char {
                PixelChar::Void => continue,
                PixelChar::Spacer => it.push(
                    RenderOp::CompositorNoClipTruncPrintTextWithAttributes(SPACER.into(), None),
                ),
                PixelChar::PlainText {
                    content,
                    maybe_style,
                } => {
                    it.push(RenderOp::ApplyColors(maybe_style.clone()));
                    it.push(RenderOp::CompositorNoClipTruncPrintTextWithAttributes(
                        content.string.clone(),
                        maybe_style.clone(),
                    ))
                }
                PixelChar::AnsiText { content } => it.push(
                    RenderOp::CompositorNoClipTruncPrintTextWithAttributes(content.clone(), None),
                ),
            }
        }

        it
    }
}

mod render_helpers {
    use super::*;

    #[derive(Debug, Clone)]
    pub struct Context {
        pub display_col_index_for_line: ChUnit,
        pub display_row_index: ChUnit,
        pub buffer_ansi_text: String,
        pub buffer_plain_text: String,
        pub prev_style: Option<Style>,
        pub prev_render_path: Option<RenderPath>,
        pub render_ops: RenderOps,
    }

    impl Context {
        pub fn new() -> Self {
            Context {
                display_col_index_for_line: ch!(0),
                buffer_ansi_text: String::new(),
                buffer_plain_text: String::new(),
                render_ops: render_ops!(),
                display_row_index: ch!(0),
                prev_render_path: None,
                prev_style: None,
            }
        }

        pub fn clear_for_new_line(&mut self, row_index: usize) {
            self.buffer_ansi_text.clear();
            self.buffer_plain_text.clear();
            self.display_col_index_for_line = ch!(0);
            self.display_row_index = ch!(row_index);
        }
    }

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub enum RenderPath {
        PlainText,
        AnsiText,
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
    pub fn style_eq(this: &Option<Style>, other: &Option<Style>) -> bool {
        match (this.is_some(), other.is_some()) {
            (false, false) => true,
            (true, true) => {
                let this = this.clone().unwrap();
                let other = other.clone().unwrap();
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
        if !context.buffer_ansi_text.is_empty() {
            render_helpers::flush_ansi_text_line_buffer(context);
        }
    }

    pub fn flush_plain_text_line_buffer(context: &mut Context) {
        // Generate `RenderOps` for each `PixelChar` and add it to `render_ops`.
        let pos = position! { col_index: context.display_col_index_for_line, row_index: context.display_row_index };

        // Deal w/ position.
        context
            .render_ops
            .push(RenderOp::MoveCursorPositionAbs(pos));

        // Deal w/ style attribs & actually paint the `temp_line_buffer`.
        context
            .render_ops
            .push(RenderOp::CompositorNoClipTruncPrintTextWithAttributes(
                context.buffer_plain_text.to_string(),
                context.prev_style.clone(),
            ));

        // Update `display_col_index_for_line`.
        let plain_text_display_width =
            UnicodeString::from(context.buffer_plain_text.as_str()).display_width;
        context.display_col_index_for_line += plain_text_display_width;

        // Clear the buffer!
        context.buffer_plain_text.clear()
    }

    pub fn flush_ansi_text_line_buffer(context: &mut Context) {
        // Generate `RenderOps` for each `PixelChar` and add it to `render_ops`.
        let pos = position! { col_index: context.display_col_index_for_line, row_index: context.display_row_index };

        // Deal w/ position.
        context
            .render_ops
            .push(RenderOp::MoveCursorPositionAbs(pos));

        // Deal w/ style attribs & actually paint the `temp_line_buffer`.
        context
            .render_ops
            .push(RenderOp::CompositorNoClipTruncPrintTextWithAttributes(
                context.buffer_ansi_text.to_string(),
                None,
            ));

        // Update `display_col_index_for_line`.
        let ansi_text = ANSIText::new(context.buffer_ansi_text.as_str());
        let ansi_text_segments = ansi_text.filter_segments_by_display_width(None);
        let ansi_text_display_width = ch!(ansi_text_segments.display_width);

        context.display_col_index_for_line += ansi_text_display_width;

        // Clear the buffer!
        context.buffer_ansi_text.clear()
    }
}

#[cfg(test)]
mod tests {
    use r3bl_rs_utils_core::*;
    use r3bl_rs_utils_macro::*;

    use super::*;
    use crate::test_editor::mock_real_objects::make_shared_global_data;

    /// Helper function to make an `OffscreenBuffer`.
    async fn make_offscreen_buffer_plain_text() -> OffscreenBuffer {
        let window_size = size! { col_count: 10, row_count: 2};
        let mut my_offscreen_buffer = OffscreenBuffer::new(window_size);
        let shared_global_data = make_shared_global_data(Some(window_size));
        // Input:  R0 "hello1234😃"
        //            C0123456789
        // Output: R0 "hello1234╳"
        //            C0123456789
        let text = "hello1234😃";
        // The style colors should be overwritten by fg_color and bg_color.
        let maybe_style =
            Some(style! { attrib: [dim, bold] color_fg: color!(@cyan) color_bg: color!(@cyan) });
        my_offscreen_buffer.my_pos = position! { col_index: 0, row_index: 0 };
        my_offscreen_buffer.my_fg_color = Some(color!(@green));
        my_offscreen_buffer.my_bg_color = Some(color!(@blue));
        let maybe_max_display_col_count: Option<ChUnit> = Some(10.into());
        render_pipeline_to_offscreen_buffer::print_text_with_attributes(
            &shared_global_data,
            text,
            &maybe_style,
            &mut my_offscreen_buffer,
            maybe_max_display_col_count,
        )
        .await
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

    #[tokio::test]
    async fn test_render_plain_text() {
        let my_offscreen_buffer = make_offscreen_buffer_plain_text().await;
        // println!("my_offscreen_buffer: \n{:#?}", my_offscreen_buffer);
        let mut paint = OffscreenBufferPaintImplCrossterm {};
        let render_ops = paint.render(&my_offscreen_buffer).await;
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
            RenderOp::MoveCursorPositionAbs(position! { col_index: 0, row_index: 0 })
        );
        assert_eq2!(
            render_ops[4],
            RenderOp::CompositorNoClipTruncPrintTextWithAttributes(
                "hello1234".to_string(),
                Some(
                    style! { attrib: [dim, bold] color_fg: color!(@green) color_bg: color!(@blue) }
                )
            )
        );
        assert_eq2!(render_ops[5], RenderOp::ResetColor);
        assert_eq2!(
            render_ops[6],
            RenderOp::MoveCursorPositionAbs(position! { col_index: 9, row_index: 0 })
        );
        assert_eq2!(
            render_ops[7],
            RenderOp::CompositorNoClipTruncPrintTextWithAttributes(SPACER.to_string(), None)
        );
        assert_eq2!(
            render_ops[8],
            RenderOp::MoveCursorPositionAbs(position! { col_index: 0, row_index: 1 })
        );
        assert_eq2!(
            render_ops[9],
            RenderOp::CompositorNoClipTruncPrintTextWithAttributes(
                SPACER.to_string().repeat(10),
                None
            )
        );
    }

    /// Helper function to make an OffscreenBuffer.
    async fn make_offscreen_buffer_plain_ansi_text_mix() -> OffscreenBuffer {
        let window_size = size! { col_count: 10, row_count: 2};
        let mut my_offscreen_buffer = OffscreenBuffer::new(window_size);
        let shared_global_data = make_shared_global_data(Some(window_size));
        // Line1.
        {
            // Input:  R0 "hello1234😃"
            //            C0123456789
            // Output: R0 "hello1234╳"
            //            C0123456789
            let plain_text_string = "hello1234😃";
            // The style colors should be overwritten by fg_color and bg_color.
            let maybe_style = Some(
                style! { attrib: [dim, bold] color_fg: color!(@cyan) color_bg: color!(@cyan) },
            );
            my_offscreen_buffer.my_pos = position! { col_index: 0, row_index: 0 };
            my_offscreen_buffer.my_fg_color = Some(color!(@green));
            my_offscreen_buffer.my_bg_color = Some(color!(@blue));
            let maybe_max_display_col_count: Option<ChUnit> = Some(10.into());
            render_pipeline_to_offscreen_buffer::print_text_with_attributes(
                &shared_global_data,
                plain_text_string,
                &maybe_style,
                &mut my_offscreen_buffer,
                maybe_max_display_col_count,
            )
            .await
            .ok();
        }

        // Line2.
        {
            let mut lolcat = LolcatBuilder::new().build();
            let content = "world";
            let content_us = UnicodeString::from(content);
            let ansi_text_string =
                lolcat_each_char_in_unicode_string(&content_us, Some(&mut lolcat));

            my_offscreen_buffer.my_pos = position! { col_index: 2, row_index: 1 };
            let maybe_max_display_col_count: Option<ChUnit> = Some(10.into());
            render_pipeline_to_offscreen_buffer::print_text_with_attributes(
                &shared_global_data,
                ansi_text_string.as_str(),
                &None,
                &mut my_offscreen_buffer,
                maybe_max_display_col_count,
            )
            .await
            .ok();
        }

        my_offscreen_buffer

        // Output:
        // my_offscreen_buffer:
        // window_size: [width:10, height:2],
        // row_index: 0,
        //     PLAIN 0: "h" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //     PLAIN 1: "e" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //     PLAIN 2: "l" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //     PLAIN 3: "l" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //     PLAIN 4: "o" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //     PLAIN 5: "1" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //     PLAIN 6: "2" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //     PLAIN 7: "3" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //     PLAIN 8: "4" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //     spacer indices: 9
        // row_index: 1,
        //     ANSI 2: "w"
        //     ANSI 3: "o"
        //     ANSI 4: "r"
        //     ANSI 5: "l"
        //     ANSI 6: "d"
        //     spacer indices: 0,1,7,8,9
    }

    #[tokio::test]
    async fn test_render_plain_ansi_text_mix() {
        let my_offscreen_buffer = make_offscreen_buffer_plain_ansi_text_mix().await;
        // println!("my_offscreen_buffer: \n{:#?}", my_offscreen_buffer);
        let mut paint = OffscreenBufferPaintImplCrossterm {};
        let render_ops = paint.render(&my_offscreen_buffer).await;
        // println!("render_ops: {:#?}", render_ops);

        // Output:
        // render_ops:
        // - RenderOps.len(): 14
        //   - [ResetColor]
        //   - [SetFgColor(green)]
        //   - [SetBgColor(blue)]
        //   - [MoveCursorPositionAbs([col:0, row:0])]
        //   - [PrintTextWithAttributes(9 bytes, Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })]
        //   - [ResetColor]
        //   - [MoveCursorPositionAbs([col:9, row:0])]
        //   - [PrintTextWithAttributes(1 bytes, None)]
        //   - [MoveCursorPositionAbs([col:0, row:1])]
        //   - [PrintTextWithAttributes(2 bytes, None)]
        //   - [MoveCursorPositionAbs([col:2, row:1])]
        //   - [PrintTextWithAttributes("world", None)]
        //   - [MoveCursorPositionAbs([col:7, row:1])]
        //   - [PrintTextWithAttributes(3 bytes, None)]

        assert_eq2!(render_ops.len(), 14);

        // Line1.
        {
            assert_eq2!(render_ops[0], RenderOp::ResetColor);
            assert_eq2!(render_ops[1], RenderOp::SetFgColor(color!(@green)));
            assert_eq2!(render_ops[2], RenderOp::SetBgColor(color!(@blue)));
            assert_eq2!(
                render_ops[3],
                RenderOp::MoveCursorPositionAbs(position! { col_index: 0, row_index: 0 })
            );
            assert_eq2!(
                render_ops[4],
                RenderOp::CompositorNoClipTruncPrintTextWithAttributes(
                    "hello1234".to_string(),
                    Some(
                        style! { attrib: [dim, bold] color_fg: color!(@green) color_bg: color!(@blue) }
                    )
                )
            );
            assert_eq2!(render_ops[5], RenderOp::ResetColor);
            assert_eq2!(
                render_ops[6],
                RenderOp::MoveCursorPositionAbs(position! { col_index: 9, row_index: 0 })
            );
            assert_eq2!(
                render_ops[7],
                RenderOp::CompositorNoClipTruncPrintTextWithAttributes(" ".to_string(), None)
            );
        }

        // Line2.
        {
            assert_eq2!(
                render_ops[8],
                RenderOp::MoveCursorPositionAbs(position! { col_index: 0, row_index: 1 })
            );
            assert_eq2!(
                render_ops[9],
                RenderOp::CompositorNoClipTruncPrintTextWithAttributes("  ".to_string(), None)
            );
            assert_eq2!(
                render_ops[10],
                RenderOp::MoveCursorPositionAbs(position! { col_index: 2, row_index: 1 })
            );
            let ansi_text = "\u{1b}[38;2;132;235;15mw\u{1b}[39m\u{1b}[38;2;136;233;14mo\u{1b}[39m\u{1b}[38;2;140;231;12mr\u{1b}[39m\u{1b}[38;2;144;228;10ml\u{1b}[39m\u{1b}[38;2;149;225;9md";
            assert_eq2!(
                render_ops[11],
                RenderOp::CompositorNoClipTruncPrintTextWithAttributes(ansi_text.into(), None)
            );
            assert_eq2!(
                render_ops[12],
                RenderOp::MoveCursorPositionAbs(position! { col_index: 7, row_index: 1 })
            );
            assert_eq2!(
                render_ops[13],
                RenderOp::CompositorNoClipTruncPrintTextWithAttributes("   ".to_string(), None)
            );
        }
    }
}
