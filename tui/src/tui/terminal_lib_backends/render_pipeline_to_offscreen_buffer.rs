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

use r3bl_rs_utils_core::*;

use super::*;
use crate::*;

impl RenderPipeline {
    /// Convert the render pipeline to an offscreen buffer.
    /// 1. This does not require any specific implementation of crossterm or termion.
    /// 2. This is the intermediate representation (IR) of a [RenderPipeline]. In order to turn
    ///    this IR into actual paint commands for the terminal, you must use the
    ///    [OffscreenBufferPaint] trait implementations.
    pub async fn convert(
        &self,
        shared_global_data: &SharedGlobalData,
    ) -> OffscreenBuffer {
        let my_window_size = shared_global_data.read().await.window_size;

        let mut my_offscreen_buffer =
            OffscreenBuffer::new_with_capacity_initialized(my_window_size);

        let mut local_data = RenderOpsLocalData::default();

        for z_order in ZOrder::get_render_order().iter() {
            if let Some(render_ops_vec) = self.get(z_order) {
                for (_render_ops_index, render_ops) in render_ops_vec.iter().enumerate() {
                    for (_render_op_index, render_op) in render_ops.iter().enumerate() {
                        process_render_op(
                            render_op,
                            shared_global_data,
                            &mut my_offscreen_buffer,
                            &mut local_data,
                        )
                        .await;
                    }
                }
            }
        }

        call_if_true!(DEBUG_TUI_COMPOSITOR, {
            let msg = format!("offscreen_buffer: \n🌟🌟🌟\n{my_offscreen_buffer:#?}");
            log_debug(msg);
        });

        my_offscreen_buffer
    }
}

async fn process_render_op(
    render_op: &RenderOp,
    shared_global_data: &SharedGlobalData,
    my_offscreen_buffer: &mut OffscreenBuffer,
    local_data: &mut RenderOpsLocalData,
) {
    match render_op {
        // Don't process these.
        RenderOp::Noop | RenderOp::EnterRawMode | RenderOp::ExitRawMode => {}
        // Do process these.
        RenderOp::ClearScreen => {
            my_offscreen_buffer.clear();
        }
        RenderOp::MoveCursorPositionAbs(new_abs_pos) => {
            my_offscreen_buffer.my_pos = sanitize_and_save_abs_position(
                *new_abs_pos,
                shared_global_data,
                local_data,
            )
            .await;
        }
        RenderOp::MoveCursorPositionRelTo(box_origin_pos_ref, content_rel_pos_ref) => {
            let new_abs_pos = *box_origin_pos_ref + *content_rel_pos_ref;
            my_offscreen_buffer.my_pos = sanitize_and_save_abs_position(
                new_abs_pos,
                shared_global_data,
                local_data,
            )
            .await;
        }
        RenderOp::SetFgColor(fg_color_ref) => {
            my_offscreen_buffer.my_fg_color = Some(*fg_color_ref);
        }
        RenderOp::SetBgColor(bg_color_ref) => {
            my_offscreen_buffer.my_bg_color = Some(*bg_color_ref);
        }
        RenderOp::ResetColor => {
            my_offscreen_buffer.my_fg_color = None;
            my_offscreen_buffer.my_bg_color = None;
        }
        RenderOp::ApplyColors(maybe_style_ref) => {
            if let Some(style_ref) = maybe_style_ref {
                my_offscreen_buffer.my_fg_color = style_ref.color_fg;
                my_offscreen_buffer.my_bg_color = style_ref.color_bg;
            }
        }
        RenderOp::CompositorNoClipTruncPaintTextWithAttributes(
            _arg_text_ref,
            _maybe_style_ref,
        ) => {
            // This is a no-op. This operation is executed by RenderOpImplCrossterm.
        }
        RenderOp::PaintTextWithAttributes(arg_text_ref, maybe_style_ref) => {
            let result_new_pos = print_text_with_attributes(
                shared_global_data,
                arg_text_ref,
                maybe_style_ref,
                my_offscreen_buffer,
                None,
            )
            .await;
            if let Ok(new_pos) = result_new_pos {
                my_offscreen_buffer.my_pos = sanitize_and_save_abs_position(
                    new_pos,
                    shared_global_data,
                    local_data,
                )
                .await;
            }
        }
    }
}

/// This diagram shows what happens per line of text.
///
/// `my_offscreen_buffer[my_pos.row_index]` is the line.
/// ```text
///             my_pos.col_index
///             ↓
///             <------------------ usable space ----------------->
/// <---------------- maybe_max_display_col_count ---------------->
/// C0123456789012345678901234567890123456789012345678901234567890
/// ```
pub async fn print_plain_text(
    arg_text_ref: &str,
    maybe_style_ref: &Option<Style>,
    my_offscreen_buffer: &mut OffscreenBuffer,
    maybe_max_display_col_count: Option<ChUnit>,
) -> CommonResult<Position> {
    // Get col and row index from `my_pos`.
    let display_col_index = ch!(@to_usize my_offscreen_buffer.my_pos.col_index);
    let display_row_index = ch!(@to_usize my_offscreen_buffer.my_pos.row_index);

    // If `maybe_max_display_col_count` is `None`, then clip to the max bounds of the window
    // 1. take the pos into account when determining clip
    // 2. even if `maybe_max_display_col_count` is `None`, still clip to the max bounds of the
    //    window

    // ✂️Clip `arg_text_ref` (if needed) and make `text`.
    let mut text: UnicodeString =
        if let Some(max_display_col_count) = maybe_max_display_col_count {
            let adj_max = max_display_col_count - (ch!(display_col_index));
            let unicode_string = arg_text_ref.unicode_string();
            let trunc_unicode_str = unicode_string.truncate_end_to_fit_width(adj_max);
            trunc_unicode_str.unicode_string()
        } else {
            arg_text_ref.unicode_string()
        };

    // ✂️Clip `text` (if needed) to the max display col count of the window.
    let window_max_display_col_count = my_offscreen_buffer.window_size.col_count;
    let text_fits_in_window =
        text.display_width <= window_max_display_col_count - (ch!(display_col_index));
    if !text_fits_in_window {
        let adj_max = window_max_display_col_count - (ch!(display_col_index));
        let trunc_unicode_str = text.truncate_end_to_fit_width(adj_max);
        text = trunc_unicode_str.unicode_string();
    }

    call_if_true!(DEBUG_TUI_COMPOSITOR, {
        let msg = format!(
            "\n🚀🚀🚀 print_plain_text():
            insertion at: display_row_index: {}, display_col_index: {}, window_size: {:?},
            text: '{}',
            width: {}",
            display_row_index,
            display_col_index,
            my_offscreen_buffer.window_size,
            text.string,
            text.display_width
        );
        log_debug(msg);
    });

    // Try to get the line at `row_index`.
    let mut line_copy = {
        if my_offscreen_buffer.buffer.get(display_row_index).is_none() {
            // Clip vertically.
            CommonError::new_err_with_only_type(CommonErrorType::DisplaySizeTooSmall)
        } else {
            let line_copy = my_offscreen_buffer
                .buffer
                .get(display_row_index)
                .unwrap()
                .clone();
            Ok(line_copy)
        }
    }?;

    // Insert clipped `text_ref_us` into `line` at `insertion_col_index`. Ok to use
    // `line_copy[insertion_col_index]` syntax because we know that row and col indices are valid.
    let mut insertion_col_index = display_col_index;
    let mut already_inserted_display_width = ch!(0);

    let maybe_style: Option<Style> = {
        if let Some(maybe_style) = maybe_style_ref {
            // We get the attributes from `maybe_style_ref`.
            let mut it = *maybe_style;
            // We get the colors from `my_fg_color` and `my_bg_color`.
            it.color_fg = my_offscreen_buffer.my_fg_color;
            it.color_bg = my_offscreen_buffer.my_bg_color;
            Some(it)
        } else if my_offscreen_buffer.my_fg_color.is_some()
            || my_offscreen_buffer.my_bg_color.is_some()
        {
            Some(Style {
                color_fg: my_offscreen_buffer.my_fg_color,
                color_bg: my_offscreen_buffer.my_bg_color,
                ..Default::default()
            })
        } else {
            None
        }
    };

    call_if_true!(
        DEBUG_TUI_COMPOSITOR,
        if maybe_style.is_some() {
            let msg = format!(
                "\n🔴🔴🔴\n[row: {display_row_index}, col: {display_col_index}] - style: {maybe_style:?}",
            );
            log_debug(msg);
        } else {
            let msg = format!(
                "\n🟣🟣🟣\n[row: {display_row_index}, col: {display_col_index}] - style: None",
            );
            log_debug(msg);
        }
    );

    // Loop over each grapheme cluster segment (the character) in `text_ref_us` (text in a line).
    // For each GraphemeClusterSegment, create a PixelChar.
    for gc_segment in text.iter() {
        let segment_display_width = ch!(@to_usize gc_segment.unicode_width);
        if segment_display_width == 0 {
            continue;
        }

        // Set the `PixelChar` at `insertion_col_index`.
        if line_copy.get(insertion_col_index).is_some() {
            let pixel_char = {
                let new_gc_segment =
                    GraphemeClusterSegment::from(gc_segment.string.as_ref());
                match (&maybe_style, new_gc_segment.string.as_str()) {
                    (None, SPACER) => PixelChar::Spacer,
                    _ => PixelChar::PlainText {
                        content: new_gc_segment,
                        maybe_style,
                    },
                }
            };

            if line_copy.get(insertion_col_index).is_some() {
                line_copy[insertion_col_index] = pixel_char;
            }

            // Deal w/ the display width of the `PixelChar` > 1. This is the equivalent of
            // `jump_cursor()` in RenderOpImplCrossterm.
            //
            // Move cursor "manually" to cover "extra" (display) width of a single character. This
            // is a necessary precautionary measure, to make sure the behavior is the same on all
            // terminals. In practice this means that terminals will be "broken" in the same way
            // across multiple terminal emulators and OSes.
            // 1. Terminals vary in their support of complex grapheme clusters (joined emoji). This
            //    code uses the crate unicode_width to display a given UTF-8 character "correctly"
            //    in all terminals. The number reported by this crate and the actual display width
            //    that the specific terminal emulator + OS combo will display may be different.
            // 2. This means that in some terminals, the caret itself has to be manually "jumped" to
            //    covert the special case of a really wide UTF-8 character. This happens by adding
            //    Void pixel chars.
            // 3. The insertion_col_index is calculated & updated based on the unicode_width crate
            //    values.
            let segment_display_width = ch!(@to_usize gc_segment.unicode_width);
            if segment_display_width > 1 {
                // Deal w/ `gc_segment` display width that is > 1 => pad w/ Void.
                let num_of_extra_display_cols_to_inject_void_into =
                    segment_display_width - 1; // Safe subtract.
                for _ in 0..num_of_extra_display_cols_to_inject_void_into {
                    // Make sure insertion_col_index is safe to access.
                    if line_copy.get(insertion_col_index + 1).is_some() {
                        // Move insertion_col_index forward & inject a PixelChar::Void.
                        insertion_col_index += 1;
                        line_copy[insertion_col_index] = PixelChar::Void;
                    }
                }
                // Move insertion_col_index forward.
                insertion_col_index += 1;
            } else {
                // `gc_segment` width is 1 => move `insertion_col_index` forward by 1.
                insertion_col_index += 1;
            }

            already_inserted_display_width += gc_segment.unicode_width;
        } else {
            // Run out of space in the line of the offscreen buffer.
            break;
        }
    }

    // Mimic what stdout does and move the position.col_index forward by the width of the text that
    // was added to display.
    let new_pos = my_offscreen_buffer
        .my_pos
        .add_col(ch!(@to_usize already_inserted_display_width));

    // 🥊Deal w/ padding SPACERs padding to end of line (if `maybe_max_display_col_count` is some).
    if let Some(max_display_col_count) = maybe_max_display_col_count {
        let adj_max = max_display_col_count - (ch!(display_col_index));
        while already_inserted_display_width < adj_max {
            if line_copy.get(insertion_col_index).is_some() {
                line_copy[insertion_col_index] = PixelChar::Spacer;
                insertion_col_index += 1;
                already_inserted_display_width += 1;
            } else {
                break;
            }
        }
    }

    // Replace the line in `my_offscreen_buffer` with the new line.
    my_offscreen_buffer.buffer[display_row_index] = line_copy;

    Ok(new_pos)
}

/// Render plain to an offscreen buffer. This will modify the `my_offscreen_buffer` argument.  For
/// plain text it supports counting [GraphemeClusterSegment]s. The display width of each segment is
/// taken into account when filling the offscreen buffer.
pub async fn print_text_with_attributes(
    _shared_global_data: &SharedGlobalData,
    arg_text_ref: &str,
    maybe_style_ref: &Option<Style>,
    my_offscreen_buffer: &mut OffscreenBuffer,
    maybe_max_display_col_count: Option<ChUnit>,
) -> CommonResult<Position> {
    print_plain_text(
        arg_text_ref,
        maybe_style_ref,
        my_offscreen_buffer,
        maybe_max_display_col_count,
    )
    .await
}

#[cfg(test)]
mod tests {
    use r3bl_rs_utils_macro::style;

    use super::*;
    use crate::test_editor::mock_real_objects_for_editor::make_shared_global_data;

    #[tokio::test]
    async fn test_print_plain_text_render_path_reuse_buffer() {
        let window_size = size! { col_count: 10, row_count: 2};
        let mut my_offscreen_buffer =
            OffscreenBuffer::new_with_capacity_initialized(window_size);
        let shared_global_data = make_shared_global_data(Some(window_size));

        // Input:  R0 "hello12345😃"
        //            C0123456789
        // Output: R0 "hello12345"
        //            C0123456789
        {
            let text = "hello12345😃";
            // The style colors should be overwritten by fg_color and bg_color.
            let maybe_style = Some(
                style! { attrib: [dim, bold, italic] color_fg: color!(@cyan) color_bg: color!(@cyan) },
            );
            my_offscreen_buffer.my_pos = position! { col_index: 0, row_index: 0 };
            my_offscreen_buffer.my_fg_color = Some(color!(@green));
            my_offscreen_buffer.my_bg_color = Some(color!(@blue));
            let maybe_max_display_col_count = Some(10.into());

            render_pipeline_to_offscreen_buffer::print_text_with_attributes(
                &shared_global_data,
                text,
                &maybe_style,
                &mut my_offscreen_buffer,
                maybe_max_display_col_count,
            )
            .await
            .ok();

            // println!("my_offscreen_buffer: \n{:#?}", my_offscreen_buffer);

            assert_eq2!(
                my_offscreen_buffer.buffer[0][0],
                PixelChar::PlainText {
                    content: GraphemeClusterSegment::from("h"),
                    maybe_style: Some(
                        style! { attrib: [dim, bold, italic] color_fg: color!(@green) color_bg: color!(@blue) }
                    ),
                }
            );
            assert_eq2!(
                my_offscreen_buffer.buffer[0][4],
                PixelChar::PlainText {
                    content: GraphemeClusterSegment::from("o"),
                    maybe_style: Some(
                        style! { attrib: [dim, bold, italic] color_fg: color!(@green) color_bg: color!(@blue) }
                    ),
                }
            );
            assert_eq2!(
                my_offscreen_buffer.buffer[0][5],
                PixelChar::PlainText {
                    content: GraphemeClusterSegment::from("1"),
                    maybe_style: Some(
                        style! { attrib: [dim, bold, italic] color_fg: color!(@green) color_bg: color!(@blue) }
                    ),
                }
            );
            assert_eq2!(
                my_offscreen_buffer.buffer[0][9],
                PixelChar::PlainText {
                    content: GraphemeClusterSegment::from("5"),
                    maybe_style: Some(
                        style! { attrib: [dim, bold, italic] color_fg: color!(@green) color_bg: color!(@blue) }
                    ),
                }
            );
        }

        // Input:  R0 "hello1234😃"
        //            C0123456789
        // Output: R0 "hello1234╳"
        //            C0123456789
        {
            let text = "hello1234😃";
            // The style colors should be overwritten by fg_color and bg_color.
            let maybe_style = Some(
                style! { attrib: [dim, bold] color_fg: color!(@cyan) color_bg: color!(@cyan) },
            );
            my_offscreen_buffer.my_pos = position! { col_index: 0, row_index: 0 };
            my_offscreen_buffer.my_fg_color = Some(color!(@green));
            my_offscreen_buffer.my_bg_color = Some(color!(@blue));
            let maybe_max_display_col_count = Some(10.into());

            render_pipeline_to_offscreen_buffer::print_text_with_attributes(
                &shared_global_data,
                text,
                &maybe_style,
                &mut my_offscreen_buffer,
                maybe_max_display_col_count,
            )
            .await
            .ok();

            // println!("my_offscreen_buffer: \n{:#?}", my_offscreen_buffer);

            assert_eq2!(
                my_offscreen_buffer.buffer[0][0],
                PixelChar::PlainText {
                    content: GraphemeClusterSegment::from("h"),
                    maybe_style: Some(
                        style! { attrib: [dim, bold] color_fg: color!(@green) color_bg: color!(@blue) }
                    ),
                }
            );
            assert_eq2!(
                my_offscreen_buffer.buffer[0][4],
                PixelChar::PlainText {
                    content: GraphemeClusterSegment::from("o"),
                    maybe_style: Some(
                        style! { attrib: [dim, bold] color_fg: color!(@green) color_bg: color!(@blue) }
                    ),
                }
            );
            assert_eq2!(
                my_offscreen_buffer.buffer[0][5],
                PixelChar::PlainText {
                    content: GraphemeClusterSegment::from("1"),
                    maybe_style: Some(
                        style! { attrib: [dim, bold] color_fg: color!(@green) color_bg: color!(@blue) }
                    ),
                }
            );
            assert_eq2!(my_offscreen_buffer.buffer[0][9], PixelChar::Spacer);
        }
    }

    #[tokio::test]
    async fn test_print_plain_text_render_path_new_buffer_for_each_paint() {
        let window_size = size! { col_count: 10, row_count: 2};
        let shared_global_data = make_shared_global_data(Some(window_size));

        // Input:  R0 "hello12345😃"
        //            C0123456789
        // Output: R0 "hello12345"
        //            C0123456789
        {
            let mut my_offscreen_buffer =
                OffscreenBuffer::new_with_capacity_initialized(window_size);
            let text = "hello12345😃";
            // The style colors should be overwritten by fg_color and bg_color.
            let maybe_style = Some(
                style! { attrib: [dim, bold] color_fg: color!(@cyan) color_bg: color!(@cyan) },
            );
            my_offscreen_buffer.my_pos = position! { col_index: 0, row_index: 0 };
            my_offscreen_buffer.my_fg_color = Some(color!(@green));
            my_offscreen_buffer.my_bg_color = Some(color!(@blue));
            let maybe_max_display_col_count = Some(10.into());

            render_pipeline_to_offscreen_buffer::print_text_with_attributes(
                &shared_global_data,
                text,
                &maybe_style,
                &mut my_offscreen_buffer,
                maybe_max_display_col_count,
            )
            .await
            .ok();

            // println!("my_offscreen_buffer: \n{:#?}", my_offscreen_buffer);

            assert_eq2!(
                my_offscreen_buffer.buffer[0][0],
                PixelChar::PlainText {
                    content: GraphemeClusterSegment::from("h"),
                    maybe_style: Some(
                        style! { attrib: [dim, bold] color_fg: color!(@green) color_bg: color!(@blue) }
                    ),
                }
            );
            assert_eq2!(
                my_offscreen_buffer.buffer[0][4],
                PixelChar::PlainText {
                    content: GraphemeClusterSegment::from("o"),
                    maybe_style: Some(
                        style! { attrib: [dim, bold] color_fg: color!(@green) color_bg: color!(@blue) }
                    ),
                }
            );
            assert_eq2!(
                my_offscreen_buffer.buffer[0][5],
                PixelChar::PlainText {
                    content: GraphemeClusterSegment::from("1"),
                    maybe_style: Some(
                        style! { attrib: [dim, bold] color_fg: color!(@green) color_bg: color!(@blue) }
                    ),
                }
            );
            assert_eq2!(
                my_offscreen_buffer.buffer[0][9],
                PixelChar::PlainText {
                    content: GraphemeClusterSegment::from("5"),
                    maybe_style: Some(
                        style! { attrib: [dim, bold] color_fg: color!(@green) color_bg: color!(@blue) }
                    ),
                }
            );
        }

        // Input:  R0 "hello1234😃"
        //            C0123456789
        // Output: R0 "hello1234╳"
        //            C0123456789
        {
            let mut my_offscreen_buffer =
                OffscreenBuffer::new_with_capacity_initialized(window_size);
            let text = "hello1234😃";
            // The style colors should be overwritten by fg_color and bg_color.
            let maybe_style = Some(
                style! { attrib: [dim, bold] color_fg: color!(@cyan) color_bg: color!(@cyan) },
            );
            my_offscreen_buffer.my_pos = position! { col_index: 0, row_index: 0 };
            my_offscreen_buffer.my_fg_color = Some(color!(@green));
            my_offscreen_buffer.my_bg_color = Some(color!(@blue));
            let maybe_max_display_col_count = Some(10.into());

            render_pipeline_to_offscreen_buffer::print_text_with_attributes(
                &shared_global_data,
                text,
                &maybe_style,
                &mut my_offscreen_buffer,
                maybe_max_display_col_count,
            )
            .await
            .ok();

            // println!("my_offscreen_buffer: \n{:#?}", my_offscreen_buffer);

            assert_eq2!(
                my_offscreen_buffer.buffer[0][0],
                PixelChar::PlainText {
                    content: GraphemeClusterSegment::from("h"),
                    maybe_style: Some(
                        style! { attrib: [dim, bold] color_fg: color!(@green) color_bg: color!(@blue) }
                    ),
                }
            );
            assert_eq2!(
                my_offscreen_buffer.buffer[0][4],
                PixelChar::PlainText {
                    content: GraphemeClusterSegment::from("o"),
                    maybe_style: Some(
                        style! { attrib: [dim, bold] color_fg: color!(@green) color_bg: color!(@blue) }
                    ),
                }
            );
            assert_eq2!(
                my_offscreen_buffer.buffer[0][5],
                PixelChar::PlainText {
                    content: GraphemeClusterSegment::from("1"),
                    maybe_style: Some(
                        style! { attrib: [dim, bold] color_fg: color!(@green) color_bg: color!(@blue) }
                    ),
                }
            );
            assert_eq2!(my_offscreen_buffer.buffer[0][9], PixelChar::Spacer);
        }

        // R0 "hello123😃"
        //    C0123456789
        {
            let mut my_offscreen_buffer =
                OffscreenBuffer::new_with_capacity_initialized(window_size);
            let text = "hello123😃";
            // The style colors should be overwritten by fg_color and bg_color.
            let maybe_style = Some(
                style! { attrib: [dim, bold] color_fg: color!(@cyan) color_bg: color!(@cyan) },
            );
            my_offscreen_buffer.my_pos = position! { col_index: 0, row_index: 0 };
            my_offscreen_buffer.my_fg_color = Some(color!(@green));
            my_offscreen_buffer.my_bg_color = Some(color!(@blue));
            let maybe_max_display_col_count = Some(10.into());

            render_pipeline_to_offscreen_buffer::print_text_with_attributes(
                &shared_global_data,
                text,
                &maybe_style,
                &mut my_offscreen_buffer,
                maybe_max_display_col_count,
            )
            .await
            .ok();

            // println!("my_offscreen_buffer: \n{:#?}", my_offscreen_buffer);

            assert_eq2!(
                my_offscreen_buffer.buffer[0][0],
                PixelChar::PlainText {
                    content: GraphemeClusterSegment::from("h"),
                    maybe_style: Some(
                        style! { attrib: [dim, bold] color_fg: color!(@green) color_bg: color!(@blue) }
                    ),
                }
            );
            assert_eq2!(
                my_offscreen_buffer.buffer[0][4],
                PixelChar::PlainText {
                    content: GraphemeClusterSegment::from("o"),
                    maybe_style: Some(
                        style! { attrib: [dim, bold] color_fg: color!(@green) color_bg: color!(@blue) }
                    ),
                }
            );
            assert_eq2!(
                my_offscreen_buffer.buffer[0][5],
                PixelChar::PlainText {
                    content: GraphemeClusterSegment::from("1"),
                    maybe_style: Some(
                        style! { attrib: [dim, bold] color_fg: color!(@green) color_bg: color!(@blue) }
                    ),
                }
            );
            assert_eq2!(
                my_offscreen_buffer.buffer[0][8],
                PixelChar::PlainText {
                    content: GraphemeClusterSegment::from("😃"),
                    maybe_style: Some(
                        style! { attrib: [dim, bold] color_fg: color!(@green) color_bg: color!(@blue) }
                    ),
                }
            );
            assert_eq2!(my_offscreen_buffer.buffer[0][9], PixelChar::Void);
        }

        // R0 "hello12😃"
        //    C0123456789
        {
            let mut my_offscreen_buffer =
                OffscreenBuffer::new_with_capacity_initialized(window_size);
            let text = "hello12😃";
            // The style colors should be overwritten by fg_color and bg_color.
            let maybe_style = Some(
                style! { attrib: [dim, bold] color_fg: color!(@cyan) color_bg: color!(@cyan) },
            );
            my_offscreen_buffer.my_pos = position! { col_index: 0, row_index: 0 };
            my_offscreen_buffer.my_fg_color = Some(color!(@green));
            my_offscreen_buffer.my_bg_color = Some(color!(@blue));
            let maybe_max_display_col_count = Some(10.into());

            render_pipeline_to_offscreen_buffer::print_text_with_attributes(
                &shared_global_data,
                text,
                &maybe_style,
                &mut my_offscreen_buffer,
                maybe_max_display_col_count,
            )
            .await
            .ok();

            // my_offscreen_buffer:
            // window_size: [width:10, height:2],
            // row_index: [0]
            // 	0: "h" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
            // 	1: "e" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
            // 	2: "l" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
            // 	3: "l" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
            // 	4: "o" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
            // 	5: "1" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
            // 	6: "2" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
            // 	7: "😃" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
            // 	8: ❯
            // 	9: ╳
            // row_index: [1]
            // 	0: ╳..
            // 	9: ╳

            // println!("my_offscreen_buffer: \n{:#?}", my_offscreen_buffer);

            assert_eq2!(
                my_offscreen_buffer.buffer[0][0],
                PixelChar::PlainText {
                    content: GraphemeClusterSegment::from("h"),
                    maybe_style: Some(
                        style! { attrib: [dim, bold] color_fg: color!(@green) color_bg: color!(@blue) }
                    ),
                }
            );
            assert_eq2!(
                my_offscreen_buffer.buffer[0][4],
                PixelChar::PlainText {
                    content: GraphemeClusterSegment::from("o"),
                    maybe_style: Some(
                        style! { attrib: [dim, bold] color_fg: color!(@green) color_bg: color!(@blue) }
                    ),
                }
            );
            assert_eq2!(
                my_offscreen_buffer.buffer[0][5],
                PixelChar::PlainText {
                    content: GraphemeClusterSegment::from("1"),
                    maybe_style: Some(
                        style! { attrib: [dim, bold] color_fg: color!(@green) color_bg: color!(@blue) }
                    ),
                }
            );
            assert_eq2!(
                my_offscreen_buffer.buffer[0][7],
                PixelChar::PlainText {
                    content: GraphemeClusterSegment::from("😃"),
                    maybe_style: Some(
                        style! { attrib: [dim, bold] color_fg: color!(@green) color_bg: color!(@blue) }
                    ),
                }
            );
            assert_eq2!(my_offscreen_buffer.buffer[0][8], PixelChar::Void);
            assert_eq2!(my_offscreen_buffer.buffer[0][9], PixelChar::Spacer);
        }
    }

    #[tokio::test]
    async fn test_convert() {
        let window_size = size! { col_count: 10, row_count: 2 };
        let shared_global_data = make_shared_global_data(window_size.into());

        // Create a RenderPipeline.
        // render_ops:
        //     - RenderOps.len(): 10
        //       - [ResetColor]
        //       - [SetFgColor(green)]
        //       - [SetBgColor(blue)]
        //       - [MoveCursorPositionAbs([col:0, row:0])]
        //       - PrintTextWithAttributes(8 bytes, Style { _id + bold + dim | fg: None | bg: None | padding: 0 }), postfix pad to 10,
        //                                 ↑
        //                                 "hello12😃" + "╳╳" ← postfix padding
        //                                 C01234567......89
        //                                         ↑
        //                                      This pixel char takes up 2 display cols.
        //                                      There are 2 extra PixelChar::Empty at display cols 8 & 9.
        //       - [ResetColor]
        let pipeline = render_pipeline!(@new ZOrder::Normal =>
            RenderOp::ClearScreen,
            RenderOp::ResetColor,
            RenderOp::SetFgColor(color!(@green)),
            RenderOp::SetBgColor(color!(@blue)),
            RenderOp::MoveCursorPositionAbs(position! { col_index: 0, row_index: 0 }),
            RenderOp::PaintTextWithAttributes(
                "hello12😃".to_string(), Some(style! { attrib: [dim, bold] })),
            RenderOp::ResetColor
        );
        // println!("pipeline: \n{:#?}", pipeline.get_all_render_op_in(ZOrder::Normal));

        // Convert it into an OffscreenBuffer.
        // my_offscreen_buffer:
        // window_size: [width:10, height:2],
        // row_index: [0]
        //     0: "h" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //     1: "e" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //     2: "l" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //     3: "l" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //     4: "o" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //     5: "1" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //     6: "2" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //     7: "😃" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //     8: ❯
        //     9: ╳
        // row_index: [1]
        //     0: ╳ ..
        //     9: ╳

        let my_offscreen_buffer = pipeline.convert(&shared_global_data).await;
        // println!("my_offscreen_buffer: \n{:#?}", my_offscreen_buffer);
        assert_eq2!(my_offscreen_buffer.buffer.len(), 2);
        assert_eq2!(
            my_offscreen_buffer.buffer[0][0],
            PixelChar::PlainText {
                content: GraphemeClusterSegment::from("h"),
                maybe_style: Some(
                    style! { attrib: [dim, bold] color_fg: color!(@green) color_bg: color!(@blue) }
                ),
            }
        );
        assert_eq2!(
            my_offscreen_buffer.buffer[0][7],
            PixelChar::PlainText {
                content: GraphemeClusterSegment::from("😃"),
                maybe_style: Some(
                    style! { attrib: [dim, bold] color_fg: color!(@green) color_bg: color!(@blue) }
                ),
            }
        );
        assert_eq2!(my_offscreen_buffer.buffer[0][8], PixelChar::Void);
        assert_eq2!(my_offscreen_buffer.buffer[0][9], PixelChar::Spacer);
    }

    #[tokio::test]
    async fn test_convert_non_zero_position() {
        let window_size = size! { col_count: 10, row_count: 2 };

        // pipeline:
        // Some(
        //     [
        //         ClearScreen,
        //         ResetColor,
        //         SetFgColor(green),
        //         SetBgColor(blue),
        //         MoveCursorPositionAbs([col:2, row:0]),
        //         PrintTextWithAttributes(9 bytes, Style { _id + bold + dim | fg: None | bg: None | padding: 0 }), pad to width 10 col count,
        //         ResetColor,
        //         SetFgColor(green),
        //         SetBgColor(blue),
        //         MoveCursorPositionAbs([col:4, row:1]),
        //         PrintTextWithAttributes(5 bytes, Style { _id + bold + dim | fg: None | bg: None | padding: 0 }), pad to width 10 col count,
        //         ResetColor,
        //     ],
        // )
        let pipeline = render_pipeline!(@new ZOrder::Normal =>
            RenderOp::ClearScreen,
            RenderOp::ResetColor,
            RenderOp::SetFgColor(color!(@green)),
            RenderOp::SetBgColor(color!(@blue)),
            RenderOp::MoveCursorPositionAbs(position! { col_index: 2, row_index: 0 }),
            RenderOp::PaintTextWithAttributes(
                "hello😃".to_string(), Some(style! { attrib: [dim, bold] })),
            RenderOp::ResetColor,
            RenderOp::SetFgColor(color!(@green)),
            RenderOp::SetBgColor(color!(@blue)),
            RenderOp::MoveCursorPositionAbs(position! { col_index: 4, row_index: 1 }),
            RenderOp::PaintTextWithAttributes(
                "world".to_string(), Some(style! { attrib: [dim, bold] })),
            RenderOp::ResetColor,
        );
        // println!("pipeline: \n{:#?}", pipeline.get_all_render_op_in(ZOrder::Normal));

        let shared_global_data = make_shared_global_data(window_size.into());
        let my_offscreen_buffer = pipeline.convert(&shared_global_data).await;
        // my_offscreen_buffer:
        // window_size: [width:10, height:2],
        // row_index: [0]
        //   0: ╳
        //   1: ╳
        //   2: "h" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //   3: "e" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //   4: "l" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //   5: "l" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //   6: "o" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //   7: "😃" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //   8: ❯
        //   9: ╳
        // row_index: [1]
        //   0: ╳
        //   1: ╳
        //   2: ╳
        //   3: ╳
        //   4: "w" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //   5: "o" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //   6: "r" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //   7: "l" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //   8: "d" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //   9: ╳

        // println!("my_offscreen_buffer: \n{:#?}", my_offscreen_buffer);

        // Contains 2 lines.
        assert_eq2!(my_offscreen_buffer.buffer.len(), 2);

        // Line 1 (row_index = 0).
        {
            assert_eq2!(my_offscreen_buffer.buffer[0][0], PixelChar::Spacer);
            assert_eq2!(my_offscreen_buffer.buffer[0][1], PixelChar::Spacer);
            assert_eq2!(
                my_offscreen_buffer.buffer[0][2],
                PixelChar::PlainText {
                    content: GraphemeClusterSegment::from("h"),
                    maybe_style: Some(
                        style! { attrib: [dim, bold] color_fg: color!(@green) color_bg: color!(@blue) }
                    ),
                }
            );
            assert_eq2!(
                my_offscreen_buffer.buffer[0][7],
                PixelChar::PlainText {
                    content: GraphemeClusterSegment::from("😃"),
                    maybe_style: Some(
                        style! { attrib: [dim, bold] color_fg: color!(@green) color_bg: color!(@blue) }
                    ),
                }
            );
            assert_eq2!(my_offscreen_buffer.buffer[0][8], PixelChar::Void);
            assert_eq2!(my_offscreen_buffer.buffer[0][9], PixelChar::Spacer);
        }

        // Line 2 (row_index = 1)
        {
            assert_eq2!(my_offscreen_buffer.buffer[1][0], PixelChar::Spacer);
            assert_eq2!(my_offscreen_buffer.buffer[1][1], PixelChar::Spacer);
            assert_eq2!(my_offscreen_buffer.buffer[1][2], PixelChar::Spacer);
            assert_eq2!(my_offscreen_buffer.buffer[1][3], PixelChar::Spacer);
            assert_eq2!(
                my_offscreen_buffer.buffer[1][4],
                PixelChar::PlainText {
                    content: GraphemeClusterSegment::from("w"),
                    maybe_style: Some(
                        style! { attrib: [dim, bold] color_fg: color!(@green) color_bg: color!(@blue) }
                    ),
                }
            );
            assert_eq2!(
                my_offscreen_buffer.buffer[1][8],
                PixelChar::PlainText {
                    content: GraphemeClusterSegment::from("d"),
                    maybe_style: Some(
                        style! { attrib: [dim, bold] color_fg: color!(@green) color_bg: color!(@blue) }
                    ),
                }
            );
            assert_eq2!(my_offscreen_buffer.buffer[1][9], PixelChar::Spacer);
        }
    }
}
