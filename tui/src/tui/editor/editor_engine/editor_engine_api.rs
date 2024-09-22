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

use crossterm::style::Stylize;
use r3bl_rs_utils_core::*;
use r3bl_rs_utils_macro::tui_style;
use syntect::easy::HighlightLines;

use super::*;
use crate::{editor_buffer_clipboard_support::ClipboardService, *};

pub struct EditorEngineApi;

impl EditorEngineApi {
    /// Event based interface for the editor. This converts the [InputEvent] into an
    /// [EditorEvent] and then executes it. Returns a new [EditorBuffer] if the operation
    /// was applied otherwise returns [None].
    pub fn apply_event(
        editor_buffer: &mut EditorBuffer,
        editor_engine: &mut EditorEngine,
        input_event: InputEvent,
        clipboard_service_provider: &mut impl ClipboardService,
    ) -> CommonResult<EditorEngineApplyEventResult> {
        let editor_config = &editor_engine.config_options;

        if let EditMode::ReadOnly = editor_config.edit_mode {
            if !input_event.matches_any_of_these_keypresses(&[
                KeyPress::Plain {
                    key: Key::SpecialKey(SpecialKey::Up),
                },
                KeyPress::Plain {
                    key: Key::SpecialKey(SpecialKey::Down),
                },
                KeyPress::Plain {
                    key: Key::SpecialKey(SpecialKey::Left),
                },
                KeyPress::Plain {
                    key: Key::SpecialKey(SpecialKey::Right),
                },
                KeyPress::Plain {
                    key: Key::SpecialKey(SpecialKey::Home),
                },
                KeyPress::Plain {
                    key: Key::SpecialKey(SpecialKey::End),
                },
                KeyPress::Plain {
                    key: Key::SpecialKey(SpecialKey::PageUp),
                },
                KeyPress::Plain {
                    key: Key::SpecialKey(SpecialKey::PageDown),
                },
            ]) {
                return Ok(EditorEngineApplyEventResult::NotApplied);
            }
        }

        if let Ok(editor_event) = EditorEvent::try_from(input_event) {
            if editor_buffer.history.is_empty() {
                history::push(editor_buffer);
            }

            EditorEvent::apply_editor_event(
                editor_engine,
                editor_buffer,
                editor_event.clone(),
                clipboard_service_provider,
            );

            match editor_event {
                EditorEvent::InsertChar(_) => {
                    history::push(editor_buffer);
                }
                EditorEvent::InsertString(_) => {
                    history::push(editor_buffer);
                }
                EditorEvent::InsertNewLine => {
                    history::push(editor_buffer);
                }
                EditorEvent::Delete => {
                    history::push(editor_buffer);
                }
                EditorEvent::Backspace => {
                    history::push(editor_buffer);
                }
                EditorEvent::Copy => {
                    history::push(editor_buffer);
                }
                EditorEvent::Paste => {
                    history::push(editor_buffer);
                }
                EditorEvent::Cut => {
                    history::push(editor_buffer);
                }
                _ => {}
            }
            Ok(EditorEngineApplyEventResult::Applied)
        } else {
            Ok(EditorEngineApplyEventResult::NotApplied)
        }
    }

    pub fn render_engine(
        editor_engine: &mut EditorEngine,
        editor_buffer: &mut EditorBuffer,
        current_box: FlexBox,
        has_focus: &mut HasFocus,
        window_size: Size,
    ) -> CommonResult<RenderPipeline> {
        throws_with_return!({
            editor_engine.current_box = current_box.into();

            if editor_buffer.is_empty() {
                EditorEngineApi::render_empty_state(RenderArgs {
                    editor_buffer,
                    editor_engine,
                    has_focus,
                })
            } else {
                let mut render_ops = render_ops!();

                cache::render_content(
                    editor_buffer,
                    editor_engine,
                    window_size,
                    has_focus,
                    &mut render_ops,
                );

                EditorEngineApi::render_selection(
                    RenderArgs {
                        editor_buffer,
                        editor_engine,
                        has_focus,
                    },
                    &mut render_ops,
                );
                EditorEngineApi::render_caret(
                    RenderArgs {
                        editor_buffer,
                        editor_engine,
                        has_focus,
                    },
                    &mut render_ops,
                );

                let mut render_pipeline = render_pipeline!();
                render_pipeline.push(ZOrder::Normal, render_ops);
                render_pipeline
            }
        })
    }

    pub fn render_content(render_args: &RenderArgs<'_>, render_ops: &mut RenderOps) {
        let RenderArgs {
            editor_buffer,
            editor_engine,
            ..
        } = render_args;
        let Size {
            col_count: max_display_col_count,
            row_count: max_display_row_count,
        } = editor_engine.current_box.style_adjusted_bounds_size;

        let syntax_highlight_enabled = matches!(
            editor_engine.config_options.syntax_highlight,
            SyntaxHighlightMode::Enable
        );

        if !syntax_highlight_enabled {
            no_syn_hi_path::render_content(
                editor_buffer,
                max_display_row_count,
                render_ops,
                editor_engine,
                max_display_col_count,
            );
            return;
        }

        // BOOKM: Render using syntect first, then custom MD parser.

        call_if_true!(
            DEBUG_TUI_MOD,
            log_debug(format!(
                "\n🍉🍉🍉\n\t{0}\n\t{1}\n\t{2}\n🍉🍉🍉",
                /* 0 */
                format!(
                    "editor_buffer.is_file_extension_default(): {}",
                    editor_buffer.is_file_extension_default()
                )
                .to_string()
                .magenta(),
                /* 1 */
                format!(
                    "editor_engine.config_options.syntax_highlight: {:?}",
                    editor_engine.config_options.syntax_highlight
                )
                .to_string()
                .blue(),
                /* 2 */
                format!(
                    "editor_buffer.get_maybe_file_extension(): {:?}",
                    editor_buffer.get_maybe_file_extension()
                )
                .to_string()
                .green(),
            ))
        );

        match editor_buffer.is_file_extension_default() {
            // Render using custom MD parser.
            true => syn_hi_r3bl_path::render_content(
                editor_buffer,
                max_display_row_count,
                render_ops,
                editor_engine,
                max_display_col_count,
            ),
            // Render using syntect.
            false => syn_hi_syntect_path::render_content(
                editor_buffer,
                max_display_row_count,
                render_ops,
                editor_engine,
                max_display_col_count,
            ),
        };
    }

    // BOOKM: Render selection
    fn render_selection(render_args: RenderArgs<'_>, render_ops: &mut RenderOps) {
        let RenderArgs {
            editor_buffer,
            editor_engine,
            ..
        } = render_args;

        for (row_index, range_of_display_col_indices) in
            editor_buffer.get_selection_map().iter()
        {
            let row_index = *row_index;
            let lines = editor_buffer.get_lines();

            let scroll_offset = editor_buffer.get_scroll_offset();

            if let Some(line) = lines.get(ch!(@to_usize *row_index)) {
                // Take the scroll_offset into account when "slicing" the selection.
                let selection = match range_of_display_col_indices
                    .locate_scroll_offset_col(scroll_offset)
                {
                    ScrollOffsetColLocationInRange::Underflow => {
                        let it = line.clip_to_range(*range_of_display_col_indices);
                        if it.is_empty() {
                            continue;
                        };
                        it
                    }
                    ScrollOffsetColLocationInRange::Overflow => {
                        let scroll_offset_clipped_selection_range = SelectionRange {
                            start_display_col_index: scroll_offset.col_index,
                            ..*range_of_display_col_indices
                        };
                        let it =
                            line.clip_to_range(scroll_offset_clipped_selection_range);
                        if it.is_empty() {
                            continue;
                        };
                        it
                    }
                };

                call_if_true!(
                    DEBUG_TUI_COPY_PASTE,
                    log_debug(format!(
                        "\n🍉🍉🍉 selection_str_slice: \n\t{0}, \n\trange: {1}, \n\tscroll_offset: {2}",
                        /* 0 */ selection.to_string().black().on_white(),
                        /* 1 */ range_of_display_col_indices,
                        /* 2 */ scroll_offset,
                    ))
                );

                let position = {
                    // Convert scroll adjusted to raw.
                    let raw_row_index = {
                        let row_scroll_offset = scroll_offset.row_index;
                        row_index - row_scroll_offset
                    };

                    // Convert scroll adjusted to raw.
                    let raw_col_index = {
                        let col_scroll_offset = scroll_offset.col_index;
                        range_of_display_col_indices.start_display_col_index
                            - col_scroll_offset
                    };

                    let it =
                        position!(col_index: raw_col_index, row_index: raw_row_index);
                    it
                };

                render_ops.push(RenderOp::MoveCursorPositionRelTo(
                    editor_engine.current_box.style_adjusted_origin_pos,
                    position,
                ));

                render_ops.push(RenderOp::ApplyColors(Some(get_selection_style())));

                render_ops.push(RenderOp::PaintTextWithAttributes(
                    selection.to_string(),
                    None,
                ));

                render_ops.push(RenderOp::ResetColor);
            }
        }
    }

    fn render_caret(render_args: RenderArgs<'_>, render_ops: &mut RenderOps) {
        let RenderArgs {
            editor_buffer,
            editor_engine,
            has_focus,
        } = render_args;

        if has_focus.does_id_have_focus(editor_engine.current_box.id) {
            let str_at_caret: String = if let Some(UnicodeStringSegmentSliceResult {
                unicode_string_seg: str_seg,
                ..
            }) =
                EditorEngineInternalApi::string_at_caret(editor_buffer, editor_engine)
            {
                str_seg.string
            } else {
                DEFAULT_CURSOR_CHAR.into()
            };

            render_ops.push(RenderOp::MoveCursorPositionRelTo(
                editor_engine.current_box.style_adjusted_origin_pos,
                editor_buffer.get_caret(CaretKind::Raw),
            ));
            render_ops.push(RenderOp::PaintTextWithAttributes(
                str_at_caret,
                tui_style! { attrib: [reverse] }.into(),
            ));
            render_ops.push(RenderOp::MoveCursorPositionRelTo(
                editor_engine.current_box.style_adjusted_origin_pos,
                editor_buffer.get_caret(CaretKind::Raw),
            ));
            render_ops.push(RenderOp::ResetColor);
        }
    }

    pub fn render_empty_state(render_args: RenderArgs<'_>) -> RenderPipeline {
        let RenderArgs {
            has_focus,
            editor_engine,
            ..
        } = render_args;
        let mut pipeline = render_pipeline!();
        let mut content_cursor_pos = position! { col_index: 0 , row_index: 0 };

        // Only when the editor has focus.
        if has_focus.does_id_have_focus(editor_engine.current_box.id) {
            // Paint line 1.
            render_pipeline! {
                @push_into pipeline
                at ZOrder::Normal
                =>
                RenderOp::MoveCursorPositionRelTo(
                    editor_engine.current_box.style_adjusted_origin_pos,
                    position! { col_index: 0 , row_index: 0 }
                ),
                RenderOp::ApplyColors(tui_style! {
                    attrib: [dim]
                    color_fg: TuiColor::Basic(ANSIBasicColor::Green)
                }.into()),
                RenderOp::PaintTextWithAttributes("📝 Please start typing your MD content.".into(), None),
                RenderOp::ResetColor
            };

            // Paint line 2.
            render_pipeline! {
              @push_into pipeline
              at ZOrder::Normal
              =>
                RenderOp::MoveCursorPositionRelTo(
                    editor_engine.current_box.style_adjusted_origin_pos,
                    content_cursor_pos.add_row_with_bounds(
                        ch!(1),
                        editor_engine.current_box.style_adjusted_bounds_size.row_count
                    )
                ),
                RenderOp::ApplyColors(tui_style! {
                    attrib: [dim]
                    color_fg: TuiColor::Basic(ANSIBasicColor::DarkGrey)
                }.into()),
                RenderOp::PaintTextWithAttributes("🧭 Ctrl+S: Save your work. Ctrl+Q: Exit the app.".into(), None),
                RenderOp::ResetColor
            };
        }

        pipeline
    }
}

pub enum EditorEngineApplyEventResult {
    Applied,
    NotApplied,
}

mod syn_hi_r3bl_path {
    use super::*;

    /// Try convert [Vec] of [US] to [MdDocument]:
    /// - Step 1: Get the lines from the buffer using
    ///           [editor_buffer.get_lines()](EditorBuffer::get_lines()).
    /// - Step 2: Convert the lines into a [List] of [StyleUSSpanLine] using
    ///           [try_parse_and_highlight()]. If this fails then take the path of no
    ///           syntax highlighting else take the path of syntax highlighting.
    pub fn render_content(
        editor_buffer: &&EditorBuffer,
        max_display_row_count: ChUnit,
        render_ops: &mut RenderOps,
        editor_engine: &&mut EditorEngine,
        max_display_col_count: ChUnit,
    ) {
        // Try to parse the Vec<US> into an MDDocument & render it.
        try_render_content(
            editor_buffer,
            max_display_row_count,
            render_ops,
            editor_engine,
            max_display_col_count,
        )
        .ok();
    }

    /// Path of syntax highlighting:
    /// - Step 1: Iterate the `List<StyleUSSpanLine>` from: `ch!(@to_usize
    ///           editor_buffer.get_scroll_offset().row_index)` to: `ch!(@to_usize
    ///           max_display_row_count)`
    /// - Step 2: For each, call `StyleUSSpanLine::clip()` which returns a `StyledTexts`
    /// - Step 3: Render the `StyledTexts` into `render_ops`
    fn try_render_content(
        editor_buffer: &&EditorBuffer,
        max_display_row_count: ChUnit,
        render_ops: &mut RenderOps,
        editor_engine: &&mut EditorEngine,
        max_display_col_count: ChUnit,
    ) -> CommonResult<()> {
        throws!({
            let lines = try_parse_and_highlight(
                editor_buffer.get_lines(),
                &editor_engine.current_box.get_computed_style(),
                Some((&editor_engine.syntax_set, &editor_engine.theme)),
            )?;

            call_if_true!(DEBUG_TUI_SYN_HI, {
                log_debug(format!(
                    "\n🎯🎯🎯\neditor_buffer.lines.len(): {} vs md_document.lines.len(): {}\n{}\n{}🎯🎯🎯",
                    editor_buffer.get_lines().len().to_string().cyan(),
                    lines.len().to_string().yellow(),
                    editor_buffer.get_as_string_with_comma_instead_of_newlines().cyan(),
                    lines.pretty_print_debug().yellow(),
                ));
            });

            for (row_index, line) in lines
                .iter()
                .skip(ch!(@to_usize editor_buffer.get_scroll_offset().row_index))
                .enumerate()
            {
                // Clip the content to max rows.
                if ch!(row_index) > max_display_row_count {
                    break;
                }

                render_single_line(
                    line,
                    editor_buffer,
                    editor_engine,
                    row_index,
                    max_display_col_count,
                    render_ops,
                );
            }
        });
    }

    fn render_single_line(
        line: &List<StyleUSSpan>,
        editor_buffer: &&EditorBuffer,
        editor_engine: &&mut EditorEngine,
        row_index: usize,
        max_display_col_count: ChUnit,
        render_ops: &mut RenderOps,
    ) {
        render_ops.push(RenderOp::MoveCursorPositionRelTo(
            editor_engine.current_box.style_adjusted_origin_pos,
            position! { col_index: 0 , row_index: ch!(@to_usize row_index) },
        ));
        let scroll_offset_col = editor_buffer.get_scroll_offset().col_index;
        let styled_texts: TuiStyledTexts =
            line.clip(scroll_offset_col, max_display_col_count);
        render_tui_styled_texts_into(&styled_texts, render_ops);
        render_ops.push(RenderOp::ResetColor);
    }
}

mod syn_hi_syntect_path {
    use super::*;

    pub fn render_content(
        editor_buffer: &&EditorBuffer,
        max_display_row_count: ChUnit,
        render_ops: &mut RenderOps,
        editor_engine: &&mut EditorEngine,
        max_display_col_count: ChUnit,
    ) {
        // Paint each line in the buffer (skipping the scroll_offset.row).
        // https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.skip
        for (row_index, line) in editor_buffer
            .get_lines()
            .iter()
            .skip(ch!(@to_usize editor_buffer.get_scroll_offset().row_index))
            .enumerate()
        {
            // Clip the content to max rows.
            if ch!(row_index) > max_display_row_count {
                break;
            }

            render_single_line(
                render_ops,
                row_index,
                editor_engine,
                editor_buffer,
                line,
                max_display_col_count,
            );
        }
    }

    fn render_single_line(
        render_ops: &mut RenderOps,
        row_index: usize,
        editor_engine: &&mut EditorEngine,
        editor_buffer: &&EditorBuffer,
        line: &UnicodeString,
        max_display_col_count: ChUnit,
    ) {
        render_ops.push(RenderOp::MoveCursorPositionRelTo(
            editor_engine.current_box.style_adjusted_origin_pos,
            position! { col_index: 0 , row_index: ch!(@to_usize row_index) },
        ));

        let it =
            try_get_syntect_highlighted_line(editor_engine, editor_buffer, &line.string);

        match it {
            // If enabled, and we have a SyntaxReference then try and highlight the line.
            Some(syntect_highlighted_line) => {
                render_line_with_syntect(
                    syntect_highlighted_line,
                    editor_buffer,
                    max_display_col_count,
                    render_ops,
                );
            }
            // Otherwise, fallback.
            None => {
                no_syn_hi_path::render_line_no_syntax_highlight(
                    line,
                    editor_buffer,
                    max_display_col_count,
                    render_ops,
                    editor_engine,
                );
            }
        }
    }

    fn render_line_with_syntect(
        syntect_highlighted_line: Vec<(syntect::highlighting::Style, &str)>,
        editor_buffer: &&EditorBuffer,
        max_display_col_count: ChUnit,
        render_ops: &mut RenderOps,
    ) {
        let scroll_offset_col = editor_buffer.get_scroll_offset().col_index;
        let list: List<StyleUSSpan> =
            convert_syntect_to_styled_text::convert_highlighted_line_from_syntect_to_tui(
                syntect_highlighted_line,
            );
        let styled_texts: TuiStyledTexts =
            list.clip(scroll_offset_col, max_display_col_count);
        render_tui_styled_texts_into(&styled_texts, render_ops);
        render_ops.push(RenderOp::ResetColor);
    }

    /// Try and load syntax highlighting for the current line. It might seem lossy to
    /// create a new [HighlightLines] for each line, but if this struct is re-used then it
    /// will not be able to highlight the lines correctly in the editor component. This
    /// struct is mutated when it is used to highlight a line, so it must be re-created
    /// for each line.
    fn try_get_syntect_highlighted_line<'a>(
        editor_engine: &'a &mut EditorEngine,
        editor_buffer: &&EditorBuffer,
        line: &'a str,
    ) -> Option<Vec<(syntect::highlighting::Style, &'a str)>> {
        let file_ext = editor_buffer.get_maybe_file_extension()?;
        let syntax_ref = try_get_syntax_ref(&editor_engine.syntax_set, file_ext)?;
        let theme = &editor_engine.theme;
        let mut highlighter = HighlightLines::new(syntax_ref, theme);
        highlighter
            .highlight_line(line, &editor_engine.syntax_set)
            .ok()
    }
}

mod no_syn_hi_path {
    use super::*;

    pub fn render_content(
        editor_buffer: &&EditorBuffer,
        max_display_row_count: ChUnit,
        render_ops: &mut RenderOps,
        editor_engine: &&mut EditorEngine,
        max_display_col_count: ChUnit,
    ) {
        // Paint each line in the buffer (skipping the scroll_offset.row).
        // https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.skip
        for (row_index, line) in editor_buffer
            .get_lines()
            .iter()
            .skip(ch!(@to_usize editor_buffer.get_scroll_offset().row_index))
            .enumerate()
        {
            // Clip the content to max rows.
            if ch!(row_index) > max_display_row_count {
                break;
            }

            render_single_line(
                render_ops,
                row_index,
                editor_engine,
                editor_buffer,
                line,
                max_display_col_count,
            );
        }
    }

    fn render_single_line(
        render_ops: &mut RenderOps,
        row_index: usize,
        editor_engine: &&mut EditorEngine,
        editor_buffer: &&EditorBuffer,
        line: &UnicodeString,
        max_display_col_count: ChUnit,
    ) {
        render_ops.push(RenderOp::MoveCursorPositionRelTo(
            editor_engine.current_box.style_adjusted_origin_pos,
            position! { col_index: 0 , row_index: ch!(@to_usize row_index) },
        ));

        no_syn_hi_path::render_line_no_syntax_highlight(
            line,
            editor_buffer,
            max_display_col_count,
            render_ops,
            editor_engine,
        );
    }

    /// This is used as a fallback by other render paths.
    pub fn render_line_no_syntax_highlight(
        line: &UnicodeString,
        editor_buffer: &&EditorBuffer,
        max_display_col_count: ChUnit,
        render_ops: &mut RenderOps,
        editor_engine: &&mut EditorEngine,
    ) {
        let scroll_offset_col_index = editor_buffer.get_scroll_offset().col_index;

        // Clip the content [scroll_offset.col .. max cols].
        let truncated_line =
            line.clip_to_width(scroll_offset_col_index, max_display_col_count);

        render_ops.push(RenderOp::ApplyColors(
            editor_engine.current_box.get_computed_style(),
        ));

        render_ops.push(RenderOp::PaintTextWithAttributes(
            truncated_line.into(),
            editor_engine.current_box.get_computed_style(),
        ));

        render_ops.push(RenderOp::ResetColor);
    }
}

#[cfg(test)]
mod test_cache {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn test_render_content() {
        let render_ops = &mut render_ops!();

        let editor_buffer = &mut EditorBuffer::default();
        let editor_engine = &mut EditorEngine::default();
        let window_size = Size {
            col_count: ch!(70),
            row_count: ch!(15),
        };
        let has_focus = &mut HasFocus::default();

        // Creating expected cache data structure
        let mut cache = HashMap::new();

        // The very first request to cache is always missed since cache is empty.
        cache::render_content(
            editor_buffer,
            editor_engine,
            window_size,
            has_focus,
            render_ops,
        );
        test_cache_miss(editor_buffer, window_size, render_ops, &mut cache);

        // Render the caret to screen. This should not change the content and result in a cache hit.
        EditorEngineApi::render_caret(
            RenderArgs {
                editor_buffer,
                editor_engine,
                has_focus,
            },
            render_ops,
        );
        cache::render_content(
            editor_buffer,
            editor_engine,
            window_size,
            has_focus,
            render_ops,
        );
        test_cache_hit(editor_buffer, &mut cache);

        // Change in window size should invalidate the cache and result in a cache miss.
        let window_size = Size {
            col_count: ch!(50),
            row_count: ch!(15),
        };
        cache::render_content(
            editor_buffer,
            editor_engine,
            window_size,
            has_focus,
            render_ops,
        );
        test_cache_miss(editor_buffer, window_size, render_ops, &mut cache);

        // Render the selection of text to screen. This should not change the content and result in a cache hit.
        EditorEngineApi::render_selection(
            RenderArgs {
                editor_buffer,
                editor_engine,
                has_focus,
            },
            render_ops,
        );
        cache::render_content(
            editor_buffer,
            editor_engine,
            window_size,
            has_focus,
            render_ops,
        );
        test_cache_hit(editor_buffer, &mut cache);

        // Change in scroll_offset should invalidate the cache and result in a cache miss.
        editor_buffer.editor_content.scroll_offset = ScrollOffset {
            col_index: ch!(1),
            row_index: ch!(1),
        };
        cache::render_content(
            editor_buffer,
            editor_engine,
            window_size,
            has_focus,
            render_ops,
        );
        test_cache_miss(editor_buffer, window_size, render_ops, &mut cache);

        // Change in content should invalidate the cache and result in a cache miss.
        editor_buffer.set_lines(vec!["r3bl".to_string()]);
        cache::render_content(
            editor_buffer,
            editor_engine,
            window_size,
            has_focus,
            render_ops,
        );
        test_cache_miss(editor_buffer, window_size, render_ops, &mut cache);
    }

    fn test_cache_miss(
        editor_buffer: &mut EditorBuffer,
        window_size: Size,
        render_ops: &mut RenderOps,
        cache: &mut HashMap<String, RenderOps>,
    ) {
        cache.clear(); // invalidating cache
        let key = format!("{}{}", editor_buffer.get_scroll_offset(), window_size); // generating key
        cache.insert(key, render_ops.clone()); // enter the new entry into cache
        assert_eq2!(editor_buffer.render_cache, cache.clone());
    }

    fn test_cache_hit(
        editor_buffer: &mut EditorBuffer,
        cache: &mut HashMap<String, RenderOps>,
    ) {
        assert_eq2!(editor_buffer.render_cache, cache.clone());
    }
}
