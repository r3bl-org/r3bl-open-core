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

use std::fmt::Debug;

use r3bl_rs_utils_core::*;
use r3bl_rs_utils_macro::style;
use syntect::easy::HighlightLines;

use super::*;
use crate::*;

impl EditorEngine {
    /// Event based interface for the editor. This converts the [InputEvent] into an [EditorEvent]
    /// and then executes it. Returns a new [EditorBuffer] if the operation was applied otherwise
    /// returns [None].
    pub async fn apply_event<S, A>(
        args: EditorEngineArgs<'_, S, A>,
        input_event: &InputEvent,
    ) -> CommonResult<EditorEngineApplyResponse<EditorBuffer>>
    where
        S: Debug + Default + Clone + PartialEq + Sync + Send,
        A: Debug + Default + Clone + Sync + Send,
    {
        let EditorEngineArgs {
            editor_buffer,
            component_registry,
            shared_global_data,
            self_id,
            editor_engine,
            ..
        } = args;

        if let Ok(editor_event) = EditorEvent::try_from(input_event) {
            let mut new_editor_buffer = editor_buffer.clone();
            EditorEvent::apply_editor_event(
                editor_engine,
                &mut new_editor_buffer,
                editor_event,
                shared_global_data,
                component_registry,
                self_id,
            );
            Ok(EditorEngineApplyResponse::Applied(new_editor_buffer))
        } else {
            Ok(EditorEngineApplyResponse::NotApplied)
        }
    }

    pub async fn render_engine<S, A>(
        args: EditorEngineArgs<'_, S, A>,
        current_box: &FlexBox,
    ) -> CommonResult<RenderPipeline>
    where
        S: Debug + Default + Clone + PartialEq + Sync + Send,
        A: Debug + Default + Clone + Sync + Send,
    {
        throws_with_return!({
            let EditorEngineArgs {
                editor_buffer,
                component_registry,
                editor_engine,
                ..
            } = args;

            editor_engine.current_box = current_box.into();

            // Create reusable args for render functions.
            let render_args = RenderArgs {
                editor_buffer,
                component_registry,
                editor_engine,
            };

            if editor_buffer.is_empty() {
                EditorEngine::render_empty_state(&render_args)
            } else {
                let mut render_ops = render_ops!();
                EditorEngine::render_content(&render_args, &mut render_ops);
                EditorEngine::render_caret(&render_args, &mut render_ops);
                let mut render_pipeline = render_pipeline!();
                render_pipeline.push(ZOrder::Normal, render_ops);
                render_pipeline
            }
        })
    }

    fn render_content<S, A>(render_args: &RenderArgs<'_, S, A>, render_ops: &mut RenderOps)
    where
        S: Debug + Default + Clone + PartialEq + Sync + Send,
        A: Debug + Default + Clone + Sync + Send,
    {
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
            SyntaxHighlightConfig::Enable(_)
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

        // Render using syntect first.
        syntect_path::render_content(
            editor_buffer,
            max_display_row_count,
            render_ops,
            editor_engine,
            max_display_col_count,
        );

        // Any overrides can be applied here.
        r3bl_path::render_content(
            editor_buffer,
            max_display_row_count,
            render_ops,
            editor_engine,
            max_display_col_count,
        );
    }

    /// Implement caret painting using two different strategies represented by [CaretPaintStyle].
    fn render_caret<S, A>(render_args: &RenderArgs<'_, S, A>, render_ops: &mut RenderOps)
    where
        S: Debug + Default + Clone + PartialEq + Sync + Send,
        A: Debug + Default + Clone + Sync + Send,
    {
        let RenderArgs {
            component_registry,
            editor_buffer,
            editor_engine,
            ..
        } = render_args;
        if component_registry
            .has_focus
            .does_id_have_focus(editor_engine.current_box.id)
        {
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
                style! { attrib: [reverse] }.into(),
            ));
            render_ops.push(RenderOp::MoveCursorPositionRelTo(
                editor_engine.current_box.style_adjusted_origin_pos,
                editor_buffer.get_caret(CaretKind::Raw),
            ));
            render_ops.push(RenderOp::ResetColor);
        }
    }

    pub fn render_empty_state<S, A>(render_args: &RenderArgs<'_, S, A>) -> RenderPipeline
    where
        S: Debug + Default + Clone + PartialEq + Sync + Send,
        A: Debug + Default + Clone + Sync + Send,
    {
        let RenderArgs {
            component_registry,
            editor_engine,
            ..
        } = render_args;
        let mut pipeline = render_pipeline!();
        let mut content_cursor_pos = position! { col_index: 0 , row_index: 0 };

        // Paint the text.
        render_pipeline! {
          @push_into pipeline
          at ZOrder::Normal
          =>
            RenderOp::MoveCursorPositionRelTo(
              editor_engine.current_box.style_adjusted_origin_pos, position! { col_index: 0 , row_index: 0 }),
            RenderOp::ApplyColors(style! {
              color_fg: TuiColor::Basic(ANSIBasicColor::Red)
            }.into()),
            RenderOp::PaintTextWithAttributes("No content added".into(), None),
            RenderOp::ResetColor
        };

        // Paint the emoji.
        if component_registry
            .has_focus
            .does_id_have_focus(editor_engine.current_box.id)
        {
            render_pipeline! {
              @push_into pipeline
              at ZOrder::Normal
              =>
                RenderOp::MoveCursorPositionRelTo(
                  editor_engine.current_box.style_adjusted_origin_pos,
                  content_cursor_pos.add_row_with_bounds(
                    ch!(1), editor_engine.current_box.style_adjusted_bounds_size.row_count)),
                RenderOp::PaintTextWithAttributes("👀".into(), None),
                RenderOp::ResetColor
            };
        }

        pipeline
    }
}

pub enum EditorEngineApplyResponse<T>
where
    T: Debug,
{
    Applied(T),
    NotApplied,
}

mod r3bl_path {
    use super::*;

    /// Try convert [Vec] of [US] to [MdDocument]:
    /// - Step 1: Get the lines from the buffer using
    ///           [editor_buffer.get_lines()](EditorBuffer::get_lines()).
    /// - Step 2: Convert the lines into a [List] of [StyleUSSpanLine] using
    ///           [try_parse_and_highlight()]. If this fails then take the path of no syntax
    ///           highlighting else take the path of syntax highlighting.
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
    /// - Step 1: Iterate the `List<StyleUSSpanLine>`
    ///           from: `ch!(@to_usize editor_buffer.get_scroll_offset().row_index)`
    ///           to: `ch!(@to_usize max_display_row_count)`
    /// - Step 2: For each, call `StyleUSSpanLine::clip()` which returns a `StyledTexts`
    /// - Step 3: Render the `StyledTexts` into `render_ops`
    fn try_render_content(
        editor_buffer: &&EditorBuffer,
        max_display_row_count: ChUnit,
        render_ops: &mut RenderOps,
        editor_engine: &&mut EditorEngine,
        max_display_col_count: ChUnit,
    ) -> CommonResult<()> {
        let lines = try_parse_and_highlight(
            editor_buffer.get_lines(),
            &editor_engine.current_box.get_computed_style(),
        )?;

        call_if_true!(DEBUG_TUI_SYN_HI, {
            log_debug(format!(
                "\n🎯🎯🎯\neditor_buffer.lines.len(): {} vs md_document.lines.len(): {}\n{}\n{}🎯🎯🎯",
                ansi_term::Color::Cyan.paint(editor_buffer.get_lines().len().to_string()),
                ansi_term::Color::Yellow.paint(lines.len().to_string()),
                ansi_term::Color::Cyan.paint(editor_buffer.get_as_string()),
                ansi_term::Color::Yellow.paint(lines.pretty_print()),
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

        Ok(())
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
        let styled_texts: StyledTexts = line.clip(scroll_offset_col, max_display_col_count);
        styled_texts.render_into(render_ops);
        render_ops.push(RenderOp::ResetColor);
    }
}

mod syntect_path {
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

        let it = try_get_syntect_highlighted_line(editor_engine, editor_buffer, &line.string);

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
            syntect_to_styled_text_conversion::from_syntect_to_tui(syntect_highlighted_line);
        let styled_texts: StyledTexts = list.clip(scroll_offset_col, max_display_col_count);
        styled_texts.render_into(render_ops);
        render_ops.push(RenderOp::ResetColor);
    }

    fn try_get_syntax_ref_from<'a>(
        editor_engine: &'a &mut EditorEngine,
        editor_buffer: &'a &EditorBuffer,
    ) -> Option<&'a syntect::parsing::SyntaxReference> {
        let syntax_set = &editor_engine.syntax_set;
        let file_extension = editor_buffer.get_file_extension();
        syntax_set.find_syntax_by_extension(file_extension)
    }

    /// Try and load syntax highlighting for the current line. It might seem lossy to create a new
    /// [HighlightLines] for each line, but if this struct is re-used then it will not be able to
    /// highlight the lines correctly in the editor component. This struct is mutated when it is
    /// used to highlight a line, so it must be re-created for each line.
    fn try_get_syntect_highlighted_line<'a>(
        editor_engine: &'a &mut EditorEngine,
        editor_buffer: &&EditorBuffer,
        line: &'a str,
    ) -> Option<Vec<(syntect::highlighting::Style, &'a str)>> {
        let syntax_ref = try_get_syntax_ref_from(editor_engine, editor_buffer)?;
        let mut highlighter = HighlightLines::new(syntax_ref, &editor_engine.theme);
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
        let truncated_line = line.clip(scroll_offset_col_index, max_display_col_count);

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
