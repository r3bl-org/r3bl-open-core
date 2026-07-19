// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Functions that implement the public (re-exported in `mod.rs`) event based API of the
//! editor engine. See [`mod@super::engine_internal_api`] for the internal and functional
//! API.
use crate::{ClipboardService, CommonResult, DEBUG_TUI_COPY_PASTE, DEBUG_TUI_MOD,
            DEBUG_TUI_SYN_HI, DEFAULT_CURSOR_CHAR, EditMode, EditorBuffer, EditorEngine,
            EditorEvent, FlexBox, GapBufferLine, HasFocus, InlineString, InputEvent,
            Key, KeyPress, NarrowingCastToU16, PrettyPrintDebug, RangeBoundsResult,
            RangeConstructExt, RenderArgs, RenderOpCommon, RenderOpIR, RenderOpIRVec,
            RenderPipeline, SpecialKey, StyleUSSpanLines, SyntaxHighlightMode,
            SyntaxHighlightPipeline, VPHeight, VPRow, VPSize, VPWidth, Viewport,
            ViewportBoundsCheck, ZOrder, c_width, convert_syntect_to_styled_text,
            core::CanvasCameraExt,
            fg_green, get_selection_style, glyphs, inline_string, new_style, ok,
            render_cache::{RenderCache, UseRenderCache},
            render_pipeline, render_tui_styled_texts_into, throws, try_get_syntax_ref,
            try_parse_and_highlight, tui_color, vp_col, vp_height, vp_pos, vp_row};
use syntect::easy::HighlightLines;

/// Checks if we should stop rendering at this row index.
///
/// Uses [`ViewportBoundsCheck`] because viewport rendering fills screen space
/// and needs to restrict rendering to positions `[0, length)` exclusive.
fn should_stop_rendering(row_index: VPRow, max_display_row_count: VPHeight) -> bool {
    row_index.check_viewport_bounds(vp_row(0), max_display_row_count)
        == RangeBoundsResult::Overflowed
}

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

fn input_event_matches_navigation_keys(input_event: &InputEvent) -> bool {
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
///
/// # Errors
///
/// Returns an error if the event processing fails.
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
        && !input_event_matches_navigation_keys(&input_event)
    {
        return Ok(EditorEngineApplyEventResult::NotApplied);
    }

    if let Ok(editor_event) = EditorEvent::try_from(input_event) {
        // The following events trigger undo / redo. Add the initial state to the history
        // if it is empty. This seeds the history buffer with its first entry.
        if triggers_undo_redo(&editor_event) & buffer.get_history().is_empty() {
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

/// # Errors
///
/// Returns an error if the rendering operation fails.
pub fn render_engine(
    engine: &mut EditorEngine,
    buffer: &mut EditorBuffer,
    current_box: FlexBox,
    has_focus: &mut HasFocus,
    window_size: VPSize,
    pipeline: &mut RenderPipeline,
) -> CommonResult {
    engine.current_box = current_box.into();

    if buffer.is_empty() {
        render_empty_state(RenderArgs::new(engine, buffer, has_focus), pipeline);
    } else {
        let mut render_ops = RenderOpIRVec::new();

        RenderCache::render_content(
            buffer,
            engine,
            window_size,
            has_focus,
            &mut render_ops,
            UseRenderCache::Yes,
        );

        render_selection(RenderArgs::new(engine, buffer, has_focus), &mut render_ops);
        render_caret(RenderArgs::new(engine, buffer, has_focus), &mut render_ops);

        pipeline.push(ZOrder::Normal, render_ops);
    }

    ok!()
}

pub fn render_content(render_args: RenderArgs<'_>, render_ops: &mut RenderOpIRVec) {
    let RenderArgs {
        buffer: editor_buffer,
        engine: editor_engine,
        ..
    } = render_args;
    let VPSize {
        col_width: max_display_col_count,
        row_height: max_display_row_count,
    } = editor_engine.current_box.style_adjusted_bounds.bounds_size;

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

    let syn_hi_pipeline = editor_buffer.get_syntax_highlight_pipeline();

    DEBUG_TUI_MOD.then(|| {
        // % is Display, ? is Debug.
        tracing::info!(
            message = %inline_string!(
                "EditorEngineApi -> render_content() {ch}",
                ch = glyphs::RENDER_GLYPH
            ),
            pipeline = ?syn_hi_pipeline,
            syn_hi_mode = ?editor_engine.config_options.syntax_highlight,
            maybe_file_ext = ?editor_buffer.get_maybe_file_extension()
        );
    });

    match syn_hi_pipeline {
        SyntaxHighlightPipeline::R3BLMarkdown => {
            syn_hi_r3bl_path::render_content(
                editor_buffer,
                max_display_row_count,
                render_ops,
                editor_engine,
                max_display_col_count,
            );
        }
        SyntaxHighlightPipeline::Syntect(file_ext) => {
            syn_hi_syntect_path::render_content(
                editor_buffer,
                file_ext,
                max_display_row_count,
                render_ops,
                editor_engine,
                max_display_col_count,
            );
        }
        SyntaxHighlightPipeline::PlainText => {
            no_syn_hi_path::render_content(
                editor_buffer,
                max_display_row_count,
                render_ops,
                editor_engine,
                max_display_col_count,
            );
        }
    }
}

// XMARK: Render selection.

pub fn render_selection(render_args: RenderArgs<'_>, mut render_ops: &mut RenderOpIRVec) {
    let RenderArgs {
        buffer: editor_buffer,
        engine: editor_engine,
        ..
    } = render_args;

    let style_adjusted_bounds = editor_engine.current_box.style_adjusted_bounds;
    let vp_origin = editor_buffer.get_vp_origin();
    let viewport = Viewport::new(vp_origin, style_adjusted_bounds.bounds_size);
    let lines = editor_buffer.get_lines();

    for selection_in_a_line in editor_buffer.get_selection_container().iter() {
        let row_index = selection_in_a_line.row;

        if let Some(line_with_info) = lines.get_line(row_index) {
            // Take the vp_origin into account when "slicing" the selection.
            let selected_str = selection_in_a_line
                .clip_left_to_vp_origin(vp_origin, row_index)
                .clip_to_range_str(line_with_info);

            if selected_str.is_empty() {
                continue;
            }

            DEBUG_TUI_COPY_PASTE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug! {
                    message = "🍉🍉🍉 selection_str_slice",
                    selected_str = %fg_green(&inline_string!("{}", selected_str)),
                    selection_line = ?selection_in_a_line,
                    vp_origin = ?vp_origin,
                };
            });

            let position = {
                // Convert canvas coordinates to viewport coordinates.
                let start_row = row_index;
                let start_col = selection_in_a_line.get_start();
                viewport.to_vp(start_col + start_row)
            };

            render_ops += RenderOpCommon::MoveCursorPositionRelTo(
                style_adjusted_bounds.origin_pos,
                position,
            );
            render_ops += RenderOpCommon::ApplyColors(Some(get_selection_style()));
            render_ops += RenderOpIR::PaintTextWithAttributes(selected_str.into(), None);
            render_ops += RenderOpCommon::ResetColor;
        }
    }
}

pub fn render_caret(render_args: RenderArgs<'_>, mut render_ops: &mut RenderOpIRVec) {
    let RenderArgs {
        buffer,
        engine,
        has_focus,
    } = render_args;

    if has_focus.does_id_have_focus(engine.current_box.id) {
        let style_adjusted_bounds = engine.current_box.style_adjusted_bounds;
        let bounds_size = style_adjusted_bounds.bounds_size;

        let is_col_visible = buffer.get_c_caret().col_index.check_viewport_bounds(
            buffer.get_vp_origin().col_index,
            bounds_size.col_width,
        ) == RangeBoundsResult::Within;

        let is_row_visible = buffer.get_c_caret().row_index.check_viewport_bounds(
            buffer.get_vp_origin().row_index,
            bounds_size.row_height,
        ) == RangeBoundsResult::Within;

        if is_col_visible && is_row_visible {
            let str_at_caret = match buffer.get_str_at_caret() {
                Some(str_slice) => InlineString::from(str_slice),
                None => DEFAULT_CURSOR_CHAR.into(),
            };

            render_ops += RenderOpCommon::MoveCursorPositionRelTo(
                style_adjusted_bounds.origin_pos,
                *buffer.get_vp_caret(),
            );
            render_ops += RenderOpIR::PaintTextWithAttributes(
                str_at_caret,
                Some(new_style!(reverse)),
            );
            render_ops += RenderOpCommon::MoveCursorPositionRelTo(
                style_adjusted_bounds.origin_pos,
                *buffer.get_vp_caret(),
            );
            render_ops += RenderOpCommon::ResetColor;
        }
    }
}

pub fn render_empty_state(render_args: RenderArgs<'_>, pipeline: &mut RenderPipeline) {
    let RenderArgs {
        engine: editor_engine,
        has_focus,
        ..
    } = render_args;

    // Only when the editor has focus.
    if has_focus.does_id_have_focus(editor_engine.current_box.id) {
        let style_adjusted_bounds = editor_engine.current_box.style_adjusted_bounds;
        let bounds_size = style_adjusted_bounds.bounds_size;

        // Paint line 1.
        render_pipeline! {
            @push_into pipeline
            at ZOrder::Normal
            =>
            RenderOpIR::Common(RenderOpCommon::MoveCursorPositionRelTo(
                style_adjusted_bounds.origin_pos,
                vp_pos(0, 0)
            )),
            RenderOpIR::Common(RenderOpCommon::ApplyColors(
                Some(new_style!(dim color_fg: {tui_color!(green)}))
            )),
            RenderOpIR::PaintTextWithAttributes("📝 Please start typing your MD content.".into(), None),
            RenderOpIR::Common(RenderOpCommon::ResetColor)
        };

        // Paint line 2.
        let mut content_cursor_pos = vp_pos(0, 0);
        content_cursor_pos.add_row_with_bounds(vp_height(1), bounds_size.row_height);
        render_pipeline! {
          @push_into pipeline
          at ZOrder::Normal
          =>
            RenderOpIR::Common(RenderOpCommon::MoveCursorPositionRelTo(
                style_adjusted_bounds.origin_pos,
                content_cursor_pos,
            )),
            RenderOpIR::Common(RenderOpCommon::ApplyColors(
                Some(new_style!(dim color_fg: {tui_color!(dark_gray)}))
            )),
            RenderOpIR::PaintTextWithAttributes("🧭 Ctrl+S: Save your work. Ctrl+Q: Exit the app.".into(), None),
            RenderOpIR::Common(RenderOpCommon::ResetColor)
        };
    }
}

#[derive(Debug)]
pub enum EditorEngineApplyEventResult {
    Applied,
    NotApplied,
}

mod syn_hi_r3bl_path {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Try to convert [`ZeroCopyGapBuffer`] to [`MdDocument`]:
    /// - Step 1: Get the lines from the buffer using [`editor_buffer.get_lines()`].
    /// - Step 2: Convert the lines into a [List] of [`StyleUSSpanLine`] using
    ///   [`try_parse_and_highlight()`]. If this fails then take the path of no syntax
    ///   highlighting else take the path of syntax highlighting.
    ///
    /// [`editor_buffer.get_lines()`]: EditorBuffer::get_lines()
    /// [`MDDocument`]: crate::markdown_parser::MDDocument
    /// [`ZeroCopyGapBuffer`]: ZeroCopyGapBuffer
    pub fn render_content(
        editor_buffer: &EditorBuffer,
        max_display_row_count: VPHeight,
        render_ops: &mut RenderOpIRVec,
        editor_engine: &mut EditorEngine,
        max_display_col_count: VPWidth,
    ) {
        // Try to parse the ZeroCopyGapBuffer into an MDDocument & render it.
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
    ///   editor_buffer.get_vp_origin().row_index)` to: `ch(@to_usize
    ///   max_display_row_count)`
    /// - Step 2: For each, call `StyleUSSpanLine::clip()` which returns a `StyledTexts`
    /// - Step 3: Render the `StyledTexts` into `render_ops`
    fn try_render_content(
        editor_buffer: &EditorBuffer,
        max_display_row_count: VPHeight,
        mut render_ops: &mut RenderOpIRVec,
        editor_engine: &mut EditorEngine,
        max_display_col_count: VPWidth,
    ) -> CommonResult {
        throws!({
            // Save some values that are needed later. But are copied here to avoid.
            // multiple borrows.
            let box_pos = editor_engine.current_box.style_adjusted_bounds.origin_pos;
            let vp_origin = editor_buffer.get_vp_origin();

            // Fill engine ast cache if empty.
            if editor_engine.ast_cache_is_empty() {
                // PERF: This function call is very expensive.
                let ast_cache: StyleUSSpanLines = try_parse_and_highlight(
                    editor_buffer.get_lines(),
                    editor_engine.current_box.get_computed_style(),
                    Some((editor_engine.syntax_set, editor_engine.theme)),
                )?;
                editor_engine.set_ast_cache(ast_cache);
            }

            // Reuse the ast cache from engine.
            debug_assert!(!editor_engine.ast_cache_is_empty());
            #[allow(
                clippy::unwrap_used,
                reason = "Cache presence verified by debug_assert"
            )]
            let lines: &StyleUSSpanLines =
                editor_engine.get_ast_cache().expect("conversion error");

            DEBUG_TUI_SYN_HI.then(|| {
                // % is Display, ? is Debug.
                //
                // # Implementation Note: Intentional Use of Raw `usize`
                //
                // Uses `.as_usize()` for debug display formatting in tracing statement.
                // Type-safe `Length` values need conversion to `usize` for string interpolation.
                tracing::debug!(
                    message = %inline_string!(
                        "🎯🎯🎯 editor_buffer.lines({a}) vs md_document.lines.len({b})",
                        a = editor_buffer.get_lines().get_c_len().as_usize(),
                        b = lines.len(),
                    ),
                    buffer_as_string = %editor_buffer.get_as_string_with_comma_instead_of_newlines(),
                    md_document_lines_debug = %lines.pretty_print_debug()
                );
            });

            for (row_index, line) in lines
                .inner
                .iter()
                .skip(vp_origin.row_index.as_usize())
                .enumerate()
            {
                let row_index = vp_row(row_index.as_u16_narrowing());

                // Clip the content to max rows.
                if should_stop_rendering(row_index, max_display_row_count) {
                    break;
                }

                // Render each line.
                render_ops +=
                    RenderOpIR::Common(RenderOpCommon::MoveCursorPositionRelTo(
                        box_pos,
                        vp_col(0) + row_index,
                    ));
                let styled_texts = line.clip(vp_origin, max_display_col_count);
                render_tui_styled_texts_into(&styled_texts, render_ops);
                render_ops += RenderOpCommon::ResetColor;
            }
        });
    }
}

mod syn_hi_syntect_path {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub fn render_content(
        editor_buffer: &EditorBuffer,
        file_ext: &str,
        max_display_row_count: VPHeight,
        render_ops: &mut RenderOpIRVec,
        editor_engine: &mut EditorEngine,
        max_display_col_count: VPWidth,
    ) {
        let lines = editor_buffer.get_lines();
        let vp_origin = editor_buffer.get_vp_origin();

        // Paint each line in the buffer (skipping the vp_origin.row_index).
        // https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.skip
        for (row_index, line_with_info) in lines
            .iter_lines()
            .skip(vp_origin.row_index.as_usize())
            .enumerate()
        {
            let row_index = vp_row(row_index.as_u16_narrowing());

            // Clip the content to max rows.
            if should_stop_rendering(row_index, max_display_row_count) {
                break;
            }

            render_single_line(
                render_ops,
                row_index,
                editor_engine,
                editor_buffer,
                file_ext,
                line_with_info,
                max_display_col_count,
            );
        }
    }

    fn render_single_line(
        mut render_ops: &mut RenderOpIRVec,
        row_index: VPRow,
        editor_engine: &mut EditorEngine,
        editor_buffer: &EditorBuffer,
        file_ext: &str,
        line: GapBufferLine<'_>,
        max_display_col_count: VPWidth,
    ) {
        render_ops += RenderOpCommon::MoveCursorPositionRelTo(
            editor_engine.current_box.style_adjusted_bounds.origin_pos,
            vp_col(0) + row_index,
        );

        let line_content = line.content();
        let it = try_get_syntect_highlighted_line(editor_engine, file_ext, line_content);

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

    #[allow(clippy::needless_pass_by_value)]
    fn render_line_with_syntect(
        syntect_highlighted_line: Vec<(syntect::highlighting::Style, &str)>,
        editor_buffer: &EditorBuffer,
        max_display_col_count: VPWidth,
        mut render_ops: &mut RenderOpIRVec,
    ) {
        let vp_origin = editor_buffer.get_vp_origin();
        let line =
            convert_syntect_to_styled_text::convert_highlighted_line_from_syntect_to_tui(
                &syntect_highlighted_line,
            );
        let styled_texts = line.clip(vp_origin, max_display_col_count);
        render_tui_styled_texts_into(&styled_texts, render_ops);
        render_ops += RenderOpCommon::ResetColor;
    }

    /// Try and load syntax highlighting for the current line. It might seem lossy to
    /// create a new [`HighlightLines`] for each line, but if this struct is re-used then
    /// it will not be able to highlight the lines correctly in the editor component.
    /// This struct is mutated when it is used to highlight a line, so it must be
    /// re-created for each line.
    fn try_get_syntect_highlighted_line<'a>(
        editor_engine: &'a mut EditorEngine,
        file_ext: &str,
        line: &'a str,
    ) -> Option<Vec<(syntect::highlighting::Style, &'a str)>> {
        let syntax_ref = try_get_syntax_ref(editor_engine.syntax_set, file_ext)?;
        let theme = &editor_engine.theme;
        let mut highlighter = HighlightLines::new(syntax_ref, theme);
        highlighter
            .highlight_line(line, editor_engine.syntax_set)
            .ok()
    }
}

mod no_syn_hi_path {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub fn render_content(
        editor_buffer: &EditorBuffer,
        max_display_row_count: VPHeight,
        render_ops: &mut RenderOpIRVec,
        editor_engine: &mut EditorEngine,
        max_display_col_count: VPWidth,
    ) {
        let lines = editor_buffer.get_lines();
        let vp_origin = editor_buffer.get_vp_origin();

        // Paint each line in the buffer (skipping the vp_origin.row_index).
        // https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.skip
        for (row_index, line_with_info) in lines
            .iter_lines()
            .skip(vp_origin.row_index.as_usize())
            .enumerate()
        {
            let row_index = vp_row(row_index.as_u16_narrowing());

            // Clip the content to max rows.
            if should_stop_rendering(row_index, max_display_row_count) {
                break;
            }

            render_single_line(
                render_ops,
                row_index,
                editor_engine,
                editor_buffer,
                line_with_info,
                max_display_col_count,
            );
        }
    }

    fn render_single_line(
        mut render_ops: &mut RenderOpIRVec,
        row_index: VPRow,
        editor_engine: &mut EditorEngine,
        editor_buffer: &EditorBuffer,
        line: GapBufferLine<'_>,
        max_display_col_count: VPWidth,
    ) {
        render_ops += RenderOpCommon::MoveCursorPositionRelTo(
            editor_engine.current_box.style_adjusted_bounds.origin_pos,
            vp_col(0) + row_index,
        );

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
        line: GapBufferLine<'_>,
        editor_buffer: &EditorBuffer,
        max_display_col_count: VPWidth,
        mut render_ops: &mut RenderOpIRVec,
        editor_engine: &mut EditorEngine,
    ) {
        let vp_origin_col = editor_buffer.get_vp_origin().col_index;

        // Clip the content [vp_origin.col_index .. max cols].
        // Use the pre-computed segment data from GapBufferLine for efficient clipping.
        let col_range =
            (vp_origin_col, c_width(max_display_col_count)).to_exclusive_range();
        let line_trunc = line.info().clip_to_range(line.content(), col_range);

        render_ops +=
            RenderOpCommon::ApplyColors(editor_engine.current_box.get_computed_style());

        render_ops += RenderOpIR::PaintTextWithAttributes(
            line_trunc.into(),
            editor_engine.current_box.get_computed_style(),
        );

        render_ops += RenderOpCommon::ResetColor;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CaretDirection, EditorEngineConfig, FlexBoxId, ModifierKeysMask,
                RenderList, VPBoundingBox, VPPos, c_col,
                clipboard_test_fixtures::TestClipboard, key_press, vp_row, vp_width};

    #[test]
    fn test_undo_redo_clears_ast_cache() {
        let mut engine = EditorEngine::default();
        let mut buffer = EditorBuffer::default();
        let mut clipboard = TestClipboard::default();

        // Add some content to create initial state.
        buffer.init_with(["Hello World"]);

        // Populate the AST cache.
        let test_ast: StyleUSSpanLines = RenderList::new();
        engine.set_ast_cache(test_ast);
        assert!(!engine.ast_cache_is_empty());

        // Apply undo event.
        let undo_event = InputEvent::Keyboard(
            key_press! { @char ModifierKeysMask::new().with_ctrl(), 'z' },
        );

        let result = apply_event(&mut buffer, &mut engine, undo_event, &mut clipboard)
            .expect("conversion error");
        assert!(matches!(result, EditorEngineApplyEventResult::Applied));

        // Verify AST cache was cleared (after our fix)
        assert!(engine.ast_cache_is_empty());

        // Set cache again and test redo.
        let test_ast2: StyleUSSpanLines = RenderList::new();
        engine.set_ast_cache(test_ast2);
        assert!(!engine.ast_cache_is_empty());

        // Apply redo event.
        let redo_event = InputEvent::Keyboard(
            key_press! { @char ModifierKeysMask::new().with_ctrl(), 'y' },
        );

        let result = apply_event(&mut buffer, &mut engine, redo_event, &mut clipboard)
            .expect("conversion error");
        assert!(matches!(result, EditorEngineApplyEventResult::Applied));

        // Verify AST cache was cleared.
        assert!(engine.ast_cache_is_empty());
    }

    #[test]
    fn test_content_modifying_events_clear_ast_cache() {
        let mut engine = EditorEngine::default();
        let mut buffer = EditorBuffer::default();
        let mut clipboard = TestClipboard::default();

        // Test InsertChar event.
        {
            let test_ast: StyleUSSpanLines = RenderList::new();
            engine.set_ast_cache(test_ast);
            assert!(!engine.ast_cache_is_empty());

            let insert_event = InputEvent::Keyboard(key_press! { @char 'a' });

            let result =
                apply_event(&mut buffer, &mut engine, insert_event, &mut clipboard)
                    .expect("conversion error");
            assert!(matches!(result, EditorEngineApplyEventResult::Applied));
            assert!(engine.ast_cache_is_empty());
        }

        // Test Delete event.
        {
            let test_ast: StyleUSSpanLines = RenderList::new();
            engine.set_ast_cache(test_ast);
            assert!(!engine.ast_cache_is_empty());

            let delete_event =
                InputEvent::Keyboard(key_press! { @special SpecialKey::Delete });

            let result =
                apply_event(&mut buffer, &mut engine, delete_event, &mut clipboard)
                    .expect("conversion error");
            assert!(matches!(result, EditorEngineApplyEventResult::Applied));
            assert!(engine.ast_cache_is_empty());
        }

        // Test Backspace event.
        {
            buffer.init_with(["test"]);
            {
                let buffer_mut = buffer.get_mut(engine.viewport());
                buffer_mut.inner.c_caret.col_index = c_col(4); // Position at end
            }

            let test_ast: StyleUSSpanLines = RenderList::new();
            engine.set_ast_cache(test_ast);
            assert!(!engine.ast_cache_is_empty());

            let backspace_event =
                InputEvent::Keyboard(key_press! { @special SpecialKey::Backspace });

            let result =
                apply_event(&mut buffer, &mut engine, backspace_event, &mut clipboard)
                    .expect("conversion error");
            assert!(matches!(result, EditorEngineApplyEventResult::Applied));
            assert!(engine.ast_cache_is_empty());
        }
    }

    #[test]
    fn test_navigation_events_do_not_clear_ast_cache() {
        let mut engine = EditorEngine::default();
        let mut buffer = EditorBuffer::default();
        let mut clipboard = TestClipboard::default();

        buffer.init_with(["Hello", "World"]);

        // Set AST cache
        let test_ast: StyleUSSpanLines = RenderList::new();
        engine.set_ast_cache(test_ast);
        assert!(!engine.ast_cache_is_empty());

        // Test arrow key navigation.
        let nav_events = vec![
            InputEvent::Keyboard(key_press! { @special SpecialKey::Up }),
            InputEvent::Keyboard(key_press! { @special SpecialKey::Down }),
            InputEvent::Keyboard(key_press! { @special SpecialKey::Left }),
            InputEvent::Keyboard(key_press! { @special SpecialKey::Right }),
        ];

        for event in nav_events {
            let result = apply_event(&mut buffer, &mut engine, event, &mut clipboard)
                .expect("conversion error");
            assert!(matches!(result, EditorEngineApplyEventResult::Applied));
            // Navigation should NOT clear the AST cache.
            assert!(!engine.ast_cache_is_empty());
        }
    }

    #[test]
    fn test_readonly_mode_filters_non_navigation_events() {
        let mut engine = EditorEngine::new(EditorEngineConfig {
            edit_mode: EditMode::ReadOnly,
            ..Default::default()
        });
        let mut buffer = EditorBuffer::default();
        let mut clipboard = TestClipboard::default();

        // Try to insert a character in readonly mode.
        let insert_event = InputEvent::Keyboard(key_press! { @char 'a' });

        let result = apply_event(&mut buffer, &mut engine, insert_event, &mut clipboard)
            .expect("conversion error");
        assert!(matches!(result, EditorEngineApplyEventResult::NotApplied));

        // Navigation should still work.
        let nav_event = InputEvent::Keyboard(key_press! { @special SpecialKey::Right });

        let result = apply_event(&mut buffer, &mut engine, nav_event, &mut clipboard)
            .expect("conversion error");
        assert!(matches!(result, EditorEngineApplyEventResult::Applied));
    }

    #[test]
    fn test_triggers_undo_redo_function() {
        // Events that should trigger undo/redo
        assert!(triggers_undo_redo(&EditorEvent::InsertChar('a')));
        assert!(triggers_undo_redo(&EditorEvent::InsertString(
            "test".to_string()
        )));
        assert!(triggers_undo_redo(&EditorEvent::InsertNewLine));
        assert!(triggers_undo_redo(&EditorEvent::Delete));
        assert!(triggers_undo_redo(&EditorEvent::Backspace));
        assert!(triggers_undo_redo(&EditorEvent::Copy));
        assert!(triggers_undo_redo(&EditorEvent::Paste));
        assert!(triggers_undo_redo(&EditorEvent::Cut));

        // Events that should NOT trigger undo/redo
        assert!(!triggers_undo_redo(&EditorEvent::Undo));
        assert!(!triggers_undo_redo(&EditorEvent::Redo));
        assert!(!triggers_undo_redo(&EditorEvent::MoveCaret(
            CaretDirection::Up
        )));
        assert!(!triggers_undo_redo(&EditorEvent::Home));
        assert!(!triggers_undo_redo(&EditorEvent::End));
    }

    #[test]
    fn test_renders_all_viewport_rows_inclusive() {
        // Setup: Create buffer with 30 lines of content
        let mut buffer = EditorBuffer::default();
        let lines: Vec<String> = (0..30).map(|i| format!("Line {i}")).collect();
        buffer.init_with(&lines);

        // Setup: Create engine with viewport height of 20 (should render rows 0-20 = 21
        // rows)
        let mut engine = EditorEngine::default();
        let test_vp_height = vp_height(20);
        let test_vp_width = vp_width(80);

        // Create FlexBox with the test viewport size
        let current_box = FlexBox {
            id: FlexBoxId::from(1),
            style_adjusted_bounds: VPBoundingBox {
                origin_pos: VPPos::default(),
                bounds_size: VPSize {
                    row_height: test_vp_height,
                    col_width: test_vp_width,
                },
            },
            ..Default::default()
        };

        // Setup: Create focus manager and give focus to the editor
        let mut has_focus = HasFocus::default();
        has_focus.set_id(FlexBoxId::from(1));

        // Create window size (larger than viewport to ensure no constraints)
        let window_size = VPSize {
            row_height: vp_height(50),
            col_width: vp_width(100),
        };

        // Execute: Render the editor
        let mut pipeline = RenderPipeline::default();
        render_engine(
            &mut engine,
            &mut buffer,
            current_box,
            &mut has_focus,
            window_size,
            &mut pipeline,
        )
        .expect("render_engine should succeed");

        // Verify: Check that row 20 is NOT rendered (the 21st c_row, 0-indexed)
        // because viewport bounds are [0, 20) exclusive.
        let has_row_20 = pipeline
            .pipeline_map
            .iter()
            .flat_map(|render_ops| render_ops.list.iter())
            .any(|op| {
                matches!(op, RenderOpIR::Common(RenderOpCommon::MoveCursorPositionRelTo(_, pos))
                    if pos.row_index == vp_row(20))
            });

        assert!(
            !has_row_20,
            "Should NOT render row 20 when viewport height is 20. \
             This verifies viewport-style bounds checking (index < length) \
             is used instead of cursor-style bounds checking (index <= length)."
        );

        // Additional verification: Ensure row 19 IS rendered (last visible row)
        let has_row_19 = pipeline
            .pipeline_map
            .iter()
            .flat_map(|render_ops| render_ops.list.iter())
            .any(|op| {
                matches!(op, RenderOpIR::Common(RenderOpCommon::MoveCursorPositionRelTo(_, pos))
                    if pos.row_index == vp_row(19))
            });

        assert!(
            has_row_19,
            "Should render row 19 when viewport height is 20."
        );
    }
}
