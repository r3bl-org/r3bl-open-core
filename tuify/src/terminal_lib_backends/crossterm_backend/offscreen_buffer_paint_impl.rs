pub struct OffscreenBufferPaintImplCrossterm;
use r3bl_rs_utils_core::*;

use crate::*;

impl OffscreenBufferPaint for OffscreenBufferPaintImplCrossterm {
    fn paint(
        &mut self,
        render_ops: RenderOps,
        flush_kind: FlushKind,
        shared_global_data: &SharedGlobalData,
    ) {
        let mut skip_flush = false;

        if let FlushKind::ClearBeforeFlush = flush_kind {
            RenderOp::default().clear_before_flush(); }

        // Execute each RenderOp.
        render_ops
            .execute_all(&mut skip_flush, shared_global_data);

        // Flush everything to the terminal.
        if !skip_flush {
            RenderOp::default().flush()
        };
        // Debug output.
        call_if_true!(DEBUG_TUIFY_SHOW_PIPELINE, {
            let msg = format!(
                "🎨 offscreen_buffer_paint_impl_crossterm::paint() ok ✅: render_ops: \n{render_ops:?}",
            );
            log_info(msg);
        });
    }

    fn paint_diff(
        &mut self,
        render_ops: RenderOps,
        shared_global_data: &SharedGlobalData,
    ) {
        let mut skip_flush = false;

        // Execute each RenderOp.
        render_ops
            .execute_all(&mut skip_flush, shared_global_data);

        // Flush everything to the terminal.
        if !skip_flush {
            RenderOp::default().flush()
        };

        // Debug output.
        call_if_true!(DEBUG_TUIFY_SHOW_PIPELINE, {
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
    fn render(&mut self, offscreen_buffer: &OffscreenBuffer) -> RenderOps {
        use render_helpers::*;

        let mut context = Context::new();

        // For each line in the offscreen buffer.
        for (row_index, line) in offscreen_buffer.buffer.iter().enumerate() {
            context.clear_for_new_line(row_index);

            // For each pixel char in the line.
            for (pixel_char_index, pixel_char) in line.iter().enumerate() {
                let (pixel_char_str, pixel_char_style): (&str, Option<Style>) =
                    match pixel_char {
                        PixelChar::Void => continue,
                        PixelChar::Spacer => (SPACER, None),
                        PixelChar::PlainText {
                            content,
                            maybe_style,
                        } => (&content.string, *maybe_style),
                    };

                let is_style_same_as_prev =
                    render_helpers::style_eq(&pixel_char_style, &context.prev_style);
                let is_at_end_of_line = ch!(pixel_char_index) == (ch!(line.len() - 1));
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
        call_if_true!(DEBUG_TUIFY_COMPOSITOR, {
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
                PixelChar::Spacer => {
                    it.push(RenderOp::CompositorNoClipTruncPaintTextWithAttributes(
                        SPACER.into(),
                        None,
                    ))
                }
                PixelChar::PlainText {
                    content,
                    maybe_style,
                } => {
                    it.push(RenderOp::ApplyColors(*maybe_style));
                    it.push(RenderOp::CompositorNoClipTruncPaintTextWithAttributes(
                        content.string.clone(),
                        *maybe_style,
                    ))
                }
            }
        }

        it
    }
}


/// Render plain to an offscreen buffer. This will modify the `my_offscreen_buffer` argument.  For
/// plain text it supports counting [GraphemeClusterSegment]s. The display width of each segment is
/// taken into account when filling the offscreen buffer.
pub fn print_text_with_attributes(
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
pub fn print_plain_text(
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

    call_if_true!(DEBUG_TUIFY_COMPOSITOR, {
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
        DEBUG_TUIFY_COMPOSITOR,
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


mod render_helpers {
    use super::*;

    #[derive(Debug, Clone)]
    pub struct Context {
        pub display_col_index_for_line: ChUnit,
        pub display_row_index: ChUnit,
        pub buffer_plain_text: String,
        pub prev_style: Option<Style>,
        pub render_ops: RenderOps,
    }

    impl Context {
        pub fn new() -> Self {
            Context {
                display_col_index_for_line: ch!(0),
                buffer_plain_text: String::new(),
                render_ops: render_ops!(),
                display_row_index: ch!(0),
                prev_style: None,
            }
        }

        pub fn clear_for_new_line(&mut self, row_index: usize) {
            self.buffer_plain_text.clear();
            self.display_col_index_for_line = ch!(0);
            self.display_row_index = ch!(row_index);
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
    pub fn style_eq(this: &Option<Style>, other: &Option<Style>) -> bool {
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
        let pos = position! { col_index: context.display_col_index_for_line, row_index: context.display_row_index};

        // Deal w/ position.
        context
            .render_ops
            .push(RenderOp::MoveCursorPositionAbs(pos));

        // Deal w/ style attribs & actually paint the `temp_line_buffer`.
        context
            .render_ops
            .push(RenderOp::CompositorNoClipTruncPaintTextWithAttributes(
                context.buffer_plain_text.to_string(),
                context.prev_style,
            ));

        // Update `display_col_index_for_line`.
        let plain_text_display_width =
            UnicodeString::from(context.buffer_plain_text.as_str()).display_width;
        context.display_col_index_for_line += plain_text_display_width;

        // Clear the buffer!
        context.buffer_plain_text.clear()
    }
}
