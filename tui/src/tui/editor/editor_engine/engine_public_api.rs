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

//! Functions that implement the public (re-exported in `mod.rs`) event based API of the
//! editor engine. See [`mod@super::engine_internal_api`] for the internal and functional
//! API.
use syntect::easy::HighlightLines;

use crate::{caret_scr_adj, caret_scroll_index,
            clipboard_support::ClipboardService,
            col, convert_syntect_to_styled_text, fg_green, get_selection_style, glyphs,
            height, inline_string, new_style,
            render_cache::{RenderCache, UseRenderCache},
            render_ops, render_pipeline, render_tui_styled_texts_into, row,
            terminal_lib_backends::KeyPress,
            throws, throws_with_return, try_get_syntax_ref, try_parse_and_highlight,
            tui_color, usize, ColWidth, CommonResult, EditMode, EditorBuffer,
            EditorEngine, EditorEvent, FlexBox, GCString, GCStringExt, HasFocus,
            InputEvent, Key, PrettyPrintDebug, RenderArgs, RenderOp, RenderOps,
            RenderPipeline, RowHeight, RowIndex, ScrollOffsetColLocationInRange,
            SegString, SelectionRange, Size, SpecialKey, StyleUSSpanLines,
            SyntaxHighlightMode, ZOrder, DEBUG_TUI_COPY_PASTE, DEBUG_TUI_MOD,
            DEBUG_TUI_SYN_HI, DEFAULT_CURSOR_CHAR};

fn triggers_undo_redo(editor_event: &EditorEvent) -> bool {
    matches!(
        editor_event,
        EditorEvent::InsertChar(_)
            | EditorEvent::InsertString(_)
            | EditorEvent::InsertNewLine
            | EditorEvent::Delete
            | EditorEvent::Backspace
            | EditorEvent::Copy
            | EditorEvent::Paste
            | EditorEvent::Cut
    )
}

fn input_event_matches_navigation_keys(input_event: InputEvent) -> bool {
    input_event.matches_any_of_these_keypresses(&[
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
    ])
}

/// Event based interface for the editor. This converts the [`InputEvent`] into an
/// [`EditorEvent`] and then executes it. Returns a new [`EditorBuffer`] if the operation
/// was applied otherwise returns [None].
pub fn apply_event(
    buffer: &mut EditorBuffer,
    engine: &mut EditorEngine,
    input_event: InputEvent,
    clipboard: &mut impl ClipboardService,
) -> CommonResult<EditorEngineApplyEventResult> {
    let editor_config = &engine.config_options;

    // If in ReadOnly mode, filter out all input events that are not navigation keys, by
    // doing early return. It is not possible to modify the buffer in ReadOnly mode.
    if let EditMode::ReadOnly = editor_config.edit_mode
        && !input_event_matches_navigation_keys(input_event.clone()) {
            return Ok(EditorEngineApplyEventResult::NotApplied);
        }

    if let Ok(editor_event) = EditorEvent::try_from(input_event) {
        // The following events trigger undo / redo. Add the initial state to the history
        // if it is empty. This seeds the history buffer with its first entry.
        if triggers_undo_redo(&editor_event) & buffer.history.is_empty() {
            engine.clear_ast_cache();
            buffer.add();
        }

        // Actually apply the editor event, which might produce a new buffer.
        EditorEvent::apply_editor_event(engine, buffer, editor_event.clone(), clipboard);

        // The following events trigger undo / redo. Now that the event has been applied,
        // add the new state to the history. So that the user will be able to get back to
        // this state if they want to (after making a change in the future).
        if triggers_undo_redo(&editor_event) {
            engine.clear_ast_cache();
            buffer.add();
        }

        Ok(EditorEngineApplyEventResult::Applied)
    } else {
        Ok(EditorEngineApplyEventResult::NotApplied)
    }
}

pub fn render_engine(
    engine: &mut EditorEngine,
    buffer: &mut EditorBuffer,
    current_box: FlexBox,
    has_focus: &mut HasFocus,
    window_size: Size,
) -> CommonResult<RenderPipeline> {
    throws_with_return!({
        engine.current_box = current_box.into();

        if buffer.is_empty() {
            render_empty_state(RenderArgs {
                engine,
                buffer,
                has_focus,
            })
        } else {
            let mut render_ops = render_ops!();

            RenderCache::render_content(
                buffer,
                engine,
                window_size,
                has_focus,
                &mut render_ops,
                UseRenderCache::Yes,
            );

            render_selection(
                RenderArgs {
                    engine,
                    buffer,
                    has_focus,
                },
                &mut render_ops,
            );
            render_caret(
                RenderArgs {
                    engine,
                    buffer,
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

pub fn render_content(render_args: RenderArgs<'_>, render_ops: &mut RenderOps) {
    let RenderArgs {
        buffer: editor_buffer,
        engine: editor_engine,
        ..
    } = render_args;
    let Size {
        col_width: max_display_col_count,
        row_height: max_display_row_count,
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

    // XMARK: Render using syntect first, then custom MD parser.

    DEBUG_TUI_MOD.then(|| {
        // % is Display, ? is Debug.
        tracing::info!(
            message = %inline_string!(
                "EditorEngineApi -> render_content() {ch}",
                ch = glyphs::RENDER_GLYPH
            ),
            is_default_file_ext = %editor_buffer.is_file_extension_default(),
            syn_hi_mode = ?editor_engine.config_options.syntax_highlight,
            maybe_file_ext = ?editor_buffer.get_maybe_file_extension()
        );
    });

    if editor_buffer.is_file_extension_default() {
        syn_hi_r3bl_path::render_content(
            editor_buffer,
            max_display_row_count,
            render_ops,
            editor_engine,
            max_display_col_count,
        );
    } else {
        syn_hi_syntect_path::render_content(
            editor_buffer,
            max_display_row_count,
            render_ops,
            editor_engine,
            max_display_col_count,
        );
    }
}

// XMARK: Render selection
pub fn render_selection(render_args: RenderArgs<'_>, render_ops: &mut RenderOps) {
    let RenderArgs {
        buffer: editor_buffer,
        engine: editor_engine,
        ..
    } = render_args;

    for (row_index, sel_range) in editor_buffer.get_selection_list().iter() {
        let row_index = *row_index;
        let lines = editor_buffer.get_lines();

        let scroll_offset = editor_buffer.get_scr_ofs();

        if let Some(line_gcs) = lines.get(usize(row_index)) {
            // Take the scroll_offset into account when "slicing" the selection.
            let selection_holder = match sel_range.locate_scroll_offset_col(scroll_offset)
            {
                ScrollOffsetColLocationInRange::Underflow => {
                    (*sel_range).clip_to_range(line_gcs)
                }
                ScrollOffsetColLocationInRange::Overflow => {
                    let start = caret_scr_adj(scroll_offset.col_index + row_index);
                    let end = caret_scr_adj(sel_range.end() + row_index);
                    let scr_ofs_clipped_sel_range: SelectionRange = (start, end).into();
                    scr_ofs_clipped_sel_range.clip_to_range(line_gcs)
                }
            };

            if selection_holder.is_empty() {
                continue;
            }

            DEBUG_TUI_COPY_PASTE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug! {
                    message = "🍉🍉🍉 selection_str_slice",
                    selection = %fg_green(&inline_string!("{}", selection_holder)),
                    range = ?sel_range,
                    scroll_offset = ?scroll_offset,
                };
            });

            let position = {
                // Convert scroll adjusted to raw.
                let raw_row_index = {
                    let row_scroll_offset = scroll_offset.row_index;
                    row_index - row_scroll_offset
                };

                // Convert scroll adjusted to raw.
                let raw_col_index = {
                    let col_scroll_offset = scroll_offset.col_index;
                    sel_range.start() - col_scroll_offset
                };

                raw_col_index + raw_row_index
            };

            render_ops.push(RenderOp::MoveCursorPositionRelTo(
                editor_engine.current_box.style_adjusted_origin_pos,
                position,
            ));

            render_ops.push(RenderOp::ApplyColors(Some(get_selection_style())));

            render_ops.push(RenderOp::PaintTextWithAttributes(
                selection_holder.into(),
                None,
            ));

            render_ops.push(RenderOp::ResetColor);
        }
    }
}

pub fn render_caret(render_args: RenderArgs<'_>, render_ops: &mut RenderOps) {
    let RenderArgs {
        buffer,
        engine,
        has_focus,
    } = render_args;

    if has_focus.does_id_have_focus(engine.current_box.id) {
        let str_at_caret = match buffer.string_at_caret() {
            Some(SegString {
                string: seg_text, ..
            }) => seg_text,
            None => DEFAULT_CURSOR_CHAR.grapheme_string(),
        };

        render_ops.push(RenderOp::MoveCursorPositionRelTo(
            engine.current_box.style_adjusted_origin_pos,
            *buffer.get_caret_raw(),
        ));
        render_ops.push(RenderOp::PaintTextWithAttributes(
            str_at_caret.string,
            Some(new_style!(reverse)),
        ));
        render_ops.push(RenderOp::MoveCursorPositionRelTo(
            engine.current_box.style_adjusted_origin_pos,
            *buffer.get_caret_raw(),
        ));
        render_ops.push(RenderOp::ResetColor);
    }
}

#[must_use]
pub fn render_empty_state(render_args: RenderArgs<'_>) -> RenderPipeline {
    let RenderArgs {
        has_focus,
        engine: editor_engine,
        ..
    } = render_args;
    let mut pipeline = render_pipeline!();

    // Only when the editor has focus.
    if has_focus.does_id_have_focus(editor_engine.current_box.id) {
        // Paint line 1.
        render_pipeline! {
            @push_into pipeline
            at ZOrder::Normal
            =>
            RenderOp::MoveCursorPositionRelTo(
                editor_engine.current_box.style_adjusted_origin_pos,
                col(0) + row(0)
            ),
            RenderOp::ApplyColors(
                Some(new_style!(dim color_fg: {tui_color!(green)}))
            ),
            RenderOp::PaintTextWithAttributes("📝 Please start typing your MD content.".into(), None),
            RenderOp::ResetColor
        };

        // Paint line 2.
        let mut content_cursor_pos = col(0) + row(0);
        content_cursor_pos.add_row_with_bounds(
            height(1),
            editor_engine
                .current_box
                .style_adjusted_bounds_size
                .row_height,
        );
        render_pipeline! {
          @push_into pipeline
          at ZOrder::Normal
          =>
            RenderOp::MoveCursorPositionRelTo(
                editor_engine.current_box.style_adjusted_origin_pos,
                content_cursor_pos,
            ),
            RenderOp::ApplyColors(
                Some(new_style!(dim color_fg: {tui_color!(dark_gray)}))
            ),
            RenderOp::PaintTextWithAttributes("🧭 Ctrl+S: Save your work. Ctrl+Q: Exit the app.".into(), None),
            RenderOp::ResetColor
        };
    }

    pipeline
}

#[derive(Debug)]
pub enum EditorEngineApplyEventResult {
    Applied,
    NotApplied,
}

mod syn_hi_r3bl_path {
    use super::{caret_scroll_index, col, inline_string, render_tui_styled_texts_into,
                row, throws, try_parse_and_highlight, usize, ColWidth, CommonResult,
                EditorBuffer, EditorEngine, PrettyPrintDebug, RenderOp, RenderOps,
                RowHeight, StyleUSSpanLines, DEBUG_TUI_SYN_HI};

    /// Try convert [Vec] of [US] to [`MdDocument`]:
    /// - Step 1: Get the lines from the buffer using
    ///   [`editor_buffer.get_lines()`](EditorBuffer::get_lines()).
    /// - Step 2: Convert the lines into a [List] of [`StyleUSSpanLine`] using
    ///   [`try_parse_and_highlight()`]. If this fails then take the path of no syntax
    ///   highlighting else take the path of syntax highlighting.
    pub fn render_content(
        editor_buffer: &EditorBuffer,
        max_display_row_count: RowHeight,
        render_ops: &mut RenderOps,
        editor_engine: &mut EditorEngine,
        max_display_col_count: ColWidth,
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
    /// - Step 1: Iterate the `List<StyleUSSpanLine>` from: `ch(@to_usize
    ///   editor_buffer.get_scroll_offset().row_index)` to: `ch(@to_usize
    ///   max_display_row_count)`
    /// - Step 2: For each, call `StyleUSSpanLine::clip()` which returns a `StyledTexts`
    /// - Step 3: Render the `StyledTexts` into `render_ops`
    fn try_render_content(
        editor_buffer: &EditorBuffer,
        max_display_row_count: RowHeight,
        render_ops: &mut RenderOps,
        editor_engine: &mut EditorEngine,
        max_display_col_count: ColWidth,
    ) -> CommonResult<()> {
        throws!({
            // Save some values that are needed later. But are copied here to avoid
            // multiple borrows.
            let box_pos = editor_engine.current_box.style_adjusted_origin_pos;
            let scr_ofs = editor_buffer.get_scr_ofs();

            // Fill engine ast cache if empty.
            if editor_engine.ast_cache_is_empty() {
                // PERF: This function call is very expensive.
                let ast_cache: StyleUSSpanLines = try_parse_and_highlight(
                    editor_buffer.get_lines(),
                    editor_engine.current_box.get_computed_style(),
                    Some((editor_engine.syntax_set, editor_engine.theme)),
                    &mut editor_engine.parser_byte_cache,
                )?;
                editor_engine.set_ast_cache(ast_cache);
            }

            // Reuse the ast cache from engine.
            debug_assert!(!editor_engine.ast_cache_is_empty());
            let lines: &StyleUSSpanLines = editor_engine.get_ast_cache().unwrap();

            DEBUG_TUI_SYN_HI.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = %inline_string!(
                        "🎯🎯🎯 editor_buffer.lines({a}) vs md_document.lines.len({b})",
                        a = editor_buffer.get_lines().len(),
                        b = lines.len(),
                    ),
                    buffer_as_string = %editor_buffer.get_as_string_with_comma_instead_of_newlines(),
                    md_document_lines_debug = %lines.pretty_print_debug()
                );
            });

            for (row_index, line) in lines
                .iter()
                .skip(usize(editor_buffer.get_scr_ofs().row_index))
                .enumerate()
            {
                let row_index = row(row_index);

                // Clip the content to max rows.
                if row_index
                    > caret_scroll_index::row_index_for_height(max_display_row_count)
                {
                    break;
                }

                // Render each line.
                render_ops.push(RenderOp::MoveCursorPositionRelTo(
                    box_pos,
                    col(0) + row_index,
                ));
                let styled_texts = line.clip(scr_ofs, max_display_col_count);
                render_tui_styled_texts_into(&styled_texts, render_ops);
                render_ops.push(RenderOp::ResetColor);
            }
        });
    }
}

mod syn_hi_syntect_path {
    use super::{caret_scroll_index, col, convert_syntect_to_styled_text, no_syn_hi_path,
                render_tui_styled_texts_into, row, try_get_syntax_ref, usize, ColWidth,
                EditorBuffer, EditorEngine, GCString, HighlightLines, RenderOp,
                RenderOps, RowHeight, RowIndex};

    pub fn render_content(
        editor_buffer: &EditorBuffer,
        max_display_row_count: RowHeight,
        render_ops: &mut RenderOps,
        editor_engine: &mut EditorEngine,
        max_display_col_count: ColWidth,
    ) {
        // Paint each line in the buffer (skipping the scroll_offset.row_index).
        // https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.skip
        for (row_index, line) in editor_buffer
            .get_lines()
            .iter()
            .skip(usize(editor_buffer.get_scr_ofs().row_index))
            .enumerate()
        {
            let row_index = row(row_index);

            // Clip the content to max rows.
            if row_index > caret_scroll_index::row_index_for_height(max_display_row_count)
            {
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
        row_index: RowIndex,
        editor_engine: &mut EditorEngine,
        editor_buffer: &EditorBuffer,
        line: &GCString,
        max_display_col_count: ColWidth,
    ) {
        render_ops.push(RenderOp::MoveCursorPositionRelTo(
            editor_engine.current_box.style_adjusted_origin_pos,
            col(0) + row_index,
        ));

        let it = try_get_syntect_highlighted_line(editor_engine, editor_buffer, line);

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
        editor_buffer: &EditorBuffer,
        max_display_col_count: ColWidth,
        render_ops: &mut RenderOps,
    ) {
        let scr_ofs = editor_buffer.get_scr_ofs();
        let line =
            convert_syntect_to_styled_text::convert_highlighted_line_from_syntect_to_tui(
                syntect_highlighted_line,
            );
        let styled_texts = line.clip(scr_ofs, max_display_col_count);
        render_tui_styled_texts_into(&styled_texts, render_ops);
        render_ops.push(RenderOp::ResetColor);
    }

    /// Try and load syntax highlighting for the current line. It might seem lossy to
    /// create a new [`HighlightLines`] for each line, but if this struct is re-used then
    /// it will not be able to highlight the lines correctly in the editor component.
    /// This struct is mutated when it is used to highlight a line, so it must be
    /// re-created for each line.
    fn try_get_syntect_highlighted_line<'a>(
        editor_engine: &'a mut EditorEngine,
        editor_buffer: &EditorBuffer,
        line: &'a GCString,
    ) -> Option<Vec<(syntect::highlighting::Style, &'a str)>> {
        let file_ext = editor_buffer.get_maybe_file_extension()?;
        let syntax_ref = try_get_syntax_ref(editor_engine.syntax_set, file_ext)?;
        let theme = &editor_engine.theme;
        let mut highlighter = HighlightLines::new(syntax_ref, theme);
        highlighter
            .highlight_line(&line.string, editor_engine.syntax_set)
            .ok()
    }
}

mod no_syn_hi_path {
    use super::{caret_scroll_index, col, no_syn_hi_path, row, usize, ColWidth,
                EditorBuffer, EditorEngine, GCString, RenderOp, RenderOps, RowHeight,
                RowIndex};

    pub fn render_content(
        editor_buffer: &EditorBuffer,
        max_display_row_count: RowHeight,
        render_ops: &mut RenderOps,
        editor_engine: &mut EditorEngine,
        max_display_col_count: ColWidth,
    ) {
        // Paint each line in the buffer (skipping the scroll_offset.row_index).
        // https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.skip
        for (row_index, line) in editor_buffer
            .get_lines()
            .iter()
            .skip(usize(editor_buffer.get_scr_ofs().row_index))
            .enumerate()
        {
            let row_index = row(row_index);

            // Clip the content to max rows.
            if row_index > caret_scroll_index::row_index_for_height(max_display_row_count)
            {
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
        row_index: RowIndex,
        editor_engine: &mut EditorEngine,
        editor_buffer: &EditorBuffer,
        line: &GCString,
        max_display_col_count: ColWidth,
    ) {
        render_ops.push(RenderOp::MoveCursorPositionRelTo(
            editor_engine.current_box.style_adjusted_origin_pos,
            col(0) + row_index,
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
        line_gcs: &GCString,
        editor_buffer: &EditorBuffer,
        max_display_col_count: ColWidth,
        render_ops: &mut RenderOps,
        editor_engine: &mut EditorEngine,
    ) {
        let scroll_offset_col_index = editor_buffer.get_scr_ofs().col_index;

        // Clip the content [scroll_offset.col_index .. max cols].
        let line_trunc = line_gcs.clip(scroll_offset_col_index, max_display_col_count);

        render_ops.push(RenderOp::ApplyColors(
            editor_engine.current_box.get_computed_style(),
        ));

        render_ops.push(RenderOp::PaintTextWithAttributes(
            line_trunc.into(),
            editor_engine.current_box.get_computed_style(),
        ));

        render_ops.push(RenderOp::ResetColor);
    }
}
