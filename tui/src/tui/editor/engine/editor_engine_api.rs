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

        let mut num_rows_rendered = ch!(0);

        // Paint each line in the buffer (skipping the scroll_offset.row).
        // https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.skip
        for (row_index, line) in editor_buffer
            .get_lines()
            .iter()
            .skip(ch!(@to_usize editor_buffer.get_scroll_offset().row_index))
            .enumerate()
        {
            num_rows_rendered += 1;

            // Clip the content to max rows.
            if ch!(row_index) > max_display_row_count {
                break;
            }

            render_ops.push(RenderOp::MoveCursorPositionRelTo(
                editor_engine.current_box.style_adjusted_origin_pos,
                position! { col_index: 0 , row_index: ch!(@to_usize row_index) },
            ));

            // Try and load syntax highlighting for the current line.
            let maybe_my_syntax = {
                if !syntax_highlight_enabled {
                    None
                } else {
                    let syntax_set = &editor_engine.syntax_set;
                    let file_extension = editor_buffer.get_file_extension();
                    let it = syntax_set.find_syntax_by_extension(file_extension);
                    it
                }
            };

            match (syntax_highlight_enabled, maybe_my_syntax) {
                (true, Some(my_syntax)) => {
                    // Load the syntax highlighting theme & create a highlighter.
                    let mut my_highlight_lines =
                        HighlightLines::new(my_syntax, &editor_engine.theme);

                    // BM: ▌SYNTECT▐ highlight a single line
                    if let Ok(syntect_highlighted_line) =
                        my_highlight_lines.highlight_line(&line.string, &editor_engine.syntax_set)
                    {
                        EditorEngine::render_line_with_syntax_highlight(
                            syntect_highlighted_line,
                            editor_buffer,
                            max_display_col_count,
                            render_ops,
                            editor_engine,
                        );
                    } else {
                        EditorEngine::render_line_no_syntax_highlight(
                            line,
                            editor_buffer,
                            max_display_col_count,
                            render_ops,
                            editor_engine,
                        );
                    }
                }
                _ => {
                    EditorEngine::render_line_no_syntax_highlight(
                        line,
                        editor_buffer,
                        max_display_col_count,
                        render_ops,
                        editor_engine,
                    );
                }
            }

            render_ops.push(RenderOp::ResetColor);
        }
    }

    fn render_line_with_syntax_highlight(
        syntect_highlighted_line: Vec<(syntect::highlighting::Style, &str)>,
        editor_buffer: &&EditorBuffer,
        max_display_col_count: ChUnit,
        render_ops: &mut RenderOps,
        _editor_engine: &&mut EditorEngine,
    ) {
        let scroll_offset_col = editor_buffer.get_scroll_offset().col_index;

        // BM: ▌RENDER▐ life of styled texts is short
        let list: List<(Style, UnicodeString)> =
            styled_text_conversion::from_syntect_to_tui(syntect_highlighted_line);
        let styled_texts: StyledTexts = list.clip(scroll_offset_col, max_display_col_count);
        styled_texts.render_into(render_ops);
    }

    fn render_line_no_syntax_highlight(
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
