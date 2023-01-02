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

use std::{borrow::Cow, fmt::Debug};

use int_enum::IntEnum;
use r3bl_rs_utils_core::*;

use crate::*;

#[derive(Debug)]
pub enum DialogEngineApplyResponse {
    UpdateEditorBuffer(EditorBuffer),
    DialogChoice(DialogChoice),
    Noop,
}

/// Things you can do with a [DialogEngine].
impl DialogEngine {
    pub async fn render_engine<S, A>(
        args: DialogEngineArgs<'_, S, A>,
    ) -> CommonResult<RenderPipeline>
    where
        S: HasDialogBuffers + Default + Clone + PartialEq + Debug + Sync + Send,
        A: Default + Clone + Sync + Send,
    {
        let current_box: PartialFlexBox = {
            match &args.dialog_engine.maybe_flex_box {
                // No need to calculate new flex box if:
                // 1) there's an existing one & 2) the window size hasn't changed.
                Some((saved_size, saved_box)) if saved_size == args.window_size => *saved_box,
                // Otherwise, calculate a new flex box & save it.
                _ => {
                    let it = internal_impl::make_flex_box_for_dialog(
                        &args.self_id,
                        &args.dialog_engine.dialog_options,
                        args.window_size,
                        args.dialog_engine.maybe_surface_bounds,
                    )?;
                    args.dialog_engine
                        .maybe_flex_box
                        .replace((*args.window_size, it));
                    it
                }
            }
        };

        let (origin_pos, bounds_size) = current_box.get_style_adjusted_position_and_size();

        let pipeline = {
            let mut it = render_pipeline!();

            it.push(
                ZOrder::Glass,
                internal_impl::render_border(&origin_pos, &bounds_size, args.dialog_engine),
            );

            it.push(
                ZOrder::Glass,
                internal_impl::render_title(
                    &origin_pos,
                    &bounds_size,
                    &args.dialog_buffer.title,
                    args.dialog_engine,
                ),
            );

            // Call render_results_panel() if mode is autocomplete.
            if matches!(
                args.dialog_engine.dialog_options.mode,
                DialogEngineMode::ModalAutocomplete
            ) {
                let results_panel_ops = internal_impl::render_results_panel(
                    &origin_pos,
                    &bounds_size,
                    args.dialog_engine,
                    args.self_id,
                    args.state,
                )?;
                if !results_panel_ops.is_empty() {
                    it.push(ZOrder::Glass, results_panel_ops);
                }
            }

            it += internal_impl::render_editor(&origin_pos, &bounds_size, args).await?;

            it
        };

        Ok(pipeline)
    }

    /// Event based interface for the editor. This executes the [InputEvent] and returns one of the
    /// following:
    /// - [DialogEngineApplyResponse::DialogChoice] => <kbd>Enter</kbd> or <kbd>Esc</kbd> was
    ///   pressed.
    /// - [DialogEngineApplyResponse::UpdateEditorBuffer] => the editor buffer was updated.
    /// - [DialogEngineApplyResponse::Noop] => otherwise.
    pub async fn apply_event<S, A>(
        args: DialogEngineArgs<'_, S, A>,
        input_event: &InputEvent,
    ) -> CommonResult<DialogEngineApplyResponse>
    where
        S: HasDialogBuffers + Default + Clone + PartialEq + Debug + Sync + Send,
        A: Default + Clone + Sync + Send,
    {
        let DialogEngineArgs {
            self_id,
            component_registry,
            shared_store,
            shared_global_data,
            state,
            dialog_buffer,
            dialog_engine,
            ..
        } = args;

        // Was a dialog choice made?
        if let Some(choice) = internal_impl::try_handle_dialog_choice(input_event, dialog_buffer) {
            dialog_engine.reset();
            return Ok(DialogEngineApplyResponse::DialogChoice(choice));
        }

        // Otherwise, pass the event to the editor engine.
        let editor_engine_args = EditorEngineArgs {
            component_registry,
            shared_global_data,
            self_id,
            editor_buffer: &dialog_buffer.editor_buffer,
            editor_engine: &mut dialog_engine.editor_engine,
            shared_store,
            state,
        };

        // If the editor engine applied the event, return the new editor buffer.
        if let EditorEngineApplyResponse::Applied(new_editor_buffer) =
            EditorEngine::apply_event(editor_engine_args, input_event).await?
        {
            return Ok(DialogEngineApplyResponse::UpdateEditorBuffer(
                new_editor_buffer,
            ));
        }

        // Otherwise, return noop.
        Ok(DialogEngineApplyResponse::Noop)
    }
}

#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, IntEnum)]
pub enum DisplayConstants {
    ColPercent = 90,
    /// border-top, title, input, border-bottom.
    SimpleModalRowCount = 4,
    EmptyLine = 1,
    DefaultResultsPanelRowCount = 5,
}

mod internal_impl {
    use super::*;

    /// Return the [FlexBox] for the dialog to be rendered in.
    ///
    /// - In non-modal contexts (which this is not), this is determined by the layout engine.
    /// - In the modal case (which this is), things are different because the dialog escapes the
    ///   boundaries of the layout engine and really just paints itself on top of everything. It can
    ///   reach any corner of the screen.
    ///   - In autocomplete mode it sizes itself differently than in normal mode.
    /// - However, it is still constrained by the bounds of the [Surface] itself and does not take
    ///   into account the full window size (in case these are different). This only applies if a
    ///   [Surface] is passed in as an argument.
    ///
    /// ```text
    /// EditorEngineFlexBox {
    ///   id: ..,
    ///   style_adjusted_origin_pos: ..,
    ///   style_adjusted_bounds_size: ..,
    ///   maybe_computed_style: None,
    /// }
    /// ```
    pub fn make_flex_box_for_dialog(
        dialog_id: &FlexBoxId,
        dialog_options: &DialogEngineConfigOptions,
        window_size: &Size,
        maybe_surface_bounds: Option<SurfaceBounds>,
    ) -> CommonResult<PartialFlexBox> {
        let surface_size = if let Some(surface_bounds) = maybe_surface_bounds {
            surface_bounds.box_size
        } else {
            *window_size
        };

        let surface_origin_pos = if let Some(surface_bounds) = maybe_surface_bounds {
            surface_bounds.origin_pos
        } else {
            position!(col_index: 0, row_index: 0)
        };

        // Check to ensure that the dialog box has enough space to be displayed.
        if window_size.col_count < ch!(MinSize::Col.int_value())
            || window_size.row_count < ch!(MinSize::Row.int_value())
        {
            return CommonError::new(
                CommonErrorType::DisplaySizeTooSmall,
                &format!(
                    "Window size is too small. Min size is {} cols x {} rows",
                    MinSize::Col.int_value(),
                    MinSize::Row.int_value()
                ),
            );
        }

        let (origin_pos, bounds_size) = match dialog_options.mode {
            DialogEngineMode::ModalSimple => {
                let simple_dialog_size = {
                    // Calc dialog bounds size based on window size.
                    let col_count = {
                        let percent = percent!(DisplayConstants::ColPercent.int_value())?;
                        percent.calc_percentage(surface_size.col_count)
                    };
                    let row_count = ch!(DisplayConstants::SimpleModalRowCount.int_value());
                    let size = size! { col_count: col_count, row_count: row_count };
                    assert!(size.row_count < ch!(MinSize::Row.int_value()));
                    size
                };

                let origin_pos = {
                    // Calc origin position based on window size & dialog size.
                    let origin_col = surface_size.col_count / 2 - simple_dialog_size.col_count / 2;
                    let origin_row = surface_size.row_count / 2 - simple_dialog_size.row_count / 2;
                    let mut it = position!(col_index: origin_col, row_index: origin_row);
                    it += surface_origin_pos;
                    it
                };

                (origin_pos, simple_dialog_size)
            }
            DialogEngineMode::ModalAutocomplete => {
                let autocomplete_dialog_size = {
                    // Calc dialog bounds size based on window size.
                    let row_count = ch!(DisplayConstants::SimpleModalRowCount.int_value())
                        + ch!(DisplayConstants::EmptyLine.int_value())
                        + dialog_options.result_panel_row_count;
                    let col_count = {
                        let percent = percent!(DisplayConstants::ColPercent.int_value())?;
                        percent.calc_percentage(surface_size.col_count)
                    };
                    let size = size!(col_count: col_count, row_count: row_count);
                    assert!(size.row_count < ch!(MinSize::Row.int_value()));
                    size
                };

                let origin_pos = {
                    // Calc origin position based on window size & dialog size.
                    let origin_col =
                        surface_size.col_count / 2 - autocomplete_dialog_size.col_count / 2;
                    let origin_row =
                        surface_size.row_count / 2 - autocomplete_dialog_size.row_count / 2;
                    let mut it = position!(col_index: origin_col, row_index: origin_row);
                    it += surface_origin_pos;
                    it
                };

                (origin_pos, autocomplete_dialog_size)
            }
        };

        throws_with_return!({
            PartialFlexBox {
                id: *dialog_id,
                style_adjusted_origin_pos: origin_pos,
                style_adjusted_bounds_size: bounds_size,
                maybe_computed_style: None,
            }
        })
    }

    pub async fn render_editor<S, A>(
        origin_pos: &Position,
        bounds_size: &Size,
        args: DialogEngineArgs<'_, S, A>,
    ) -> CommonResult<RenderPipeline>
    where
        S: Default + Clone + PartialEq + Debug + Sync + Send,
        A: Default + Clone + Sync + Send,
    {
        let maybe_style = args.dialog_engine.dialog_options.maybe_style_editor;

        let flex_box: FlexBox = PartialFlexBox {
            id: args.self_id,
            style_adjusted_origin_pos: position! {col_index: origin_pos.col_index + 1, row_index: origin_pos.row_index + 2},
            style_adjusted_bounds_size: size! {col_count: bounds_size.col_count - 2, row_count: 1},
            maybe_computed_style: maybe_style,
        }
        .into();

        let editor_engine_args = EditorEngineArgs {
            component_registry: args.component_registry,
            shared_global_data: args.shared_global_data,
            self_id: args.self_id,
            editor_buffer: &args.dialog_buffer.editor_buffer,
            editor_engine: &mut args.dialog_engine.editor_engine,
            shared_store: args.shared_store,
            state: args.state,
        };

        let mut pipeline = EditorEngine::render_engine(editor_engine_args, &flex_box).await?;
        pipeline.hoist(ZOrder::Normal, ZOrder::Glass);

        Ok(pipeline)
    }

    pub fn render_results_panel<S>(
        origin_pos: &Position,
        bounds_size: &Size,
        dialog_engine: &mut DialogEngine,
        self_id: FlexBoxId,
        state: &S,
    ) -> CommonResult<RenderOps>
    where
        S: HasDialogBuffers + Default + Clone + PartialEq + Debug + Sync + Send,
    {
        let mut it = render_ops!();

        if let Some(dialog_buffer) = state.get_dialog_buffer(self_id) {
            if let Some(results) = dialog_buffer.maybe_results.as_ref() {
                if !results.is_empty() {
                    paint_results(&mut it, origin_pos, bounds_size, results, dialog_engine);
                };
            }
        };

        return Ok(it);

        pub fn paint_results(
            ops: &mut RenderOps,
            origin_pos: &Position,
            bounds_size: &Size,
            results: &[String],
            dialog_engine: &mut DialogEngine,
        ) {
            // TODO: draw results panel from state.dialog_buffer.results (Vec<String>)
            let max_row_count = dialog_engine.dialog_options.result_panel_row_count;
            let col_start_index = ch!(1);
            let row_start_index = ch!(DisplayConstants::SimpleModalRowCount.int_value())
                + ch!(DisplayConstants::EmptyLine.int_value());

            for (row_index, item) in results.iter().enumerate() {
                let row_index = ch!(row_index);
                let abs_insertion_pos = position!(
                    col_index: col_start_index,
                    row_index: row_start_index + ch!(row_index)
                );

                // TODO: check for text clipping using bounds_size

                // TODO: does this work?
                if row_index >= max_row_count {
                    break;
                }

                ops.push(RenderOp::ResetColor);
                ops.push(RenderOp::MoveCursorPositionRelTo(
                    *origin_pos,
                    abs_insertion_pos,
                ));

                // If the row_index == dialog_engine.selected_row_index (not in state) ie, it is
                // selected, then change the attribute to underline.
                if dialog_engine.selected_row_index == row_index {
                    ops.push(RenderOp::ApplyColors(
                        match dialog_engine.dialog_options.maybe_style_results_panel {
                            Some(style) => {
                                let mut new_style = style;
                                new_style.underline = true;
                                Some(new_style)
                            }
                            _ => None,
                        },
                    ));
                }
                // Regular row, not selected.
                else {
                    ops.push(RenderOp::ApplyColors(
                        dialog_engine.dialog_options.maybe_style_results_panel,
                    ));
                }

                ops.push(RenderOp::PaintTextWithAttributes(
                    item.into(),
                    dialog_engine.dialog_options.maybe_style_results_panel,
                ));
            }
        }
    }

    pub fn render_title(
        origin_pos: &Position,
        bounds_size: &Size,
        title: &str,
        dialog_engine: &mut DialogEngine,
    ) -> RenderOps {
        let mut ops = render_ops!();

        let row_pos =
            position!(col_index: origin_pos.col_index + 1, row_index: origin_pos.row_index + 1);
        let unicode_string = UnicodeString::from(title);
        let mut text_content = Cow::Borrowed(unicode_string.truncate_to_fit_size(size! {
          col_count: bounds_size.col_count - 2, row_count: bounds_size.row_count
        }));

        // Apply lolcat override (if enabled) to the fg_color of text_content.
        apply_lolcat_from_style(
            &dialog_engine.dialog_options.maybe_style_title,
            &mut dialog_engine.lolcat,
            &mut text_content,
        );

        ops.push(RenderOp::ResetColor);
        ops.push(RenderOp::MoveCursorPositionAbs(row_pos));
        ops.push(RenderOp::ApplyColors(
            dialog_engine.dialog_options.maybe_style_title,
        ));
        ops.push(RenderOp::PaintTextWithAttributes(
            text_content.into(),
            dialog_engine.dialog_options.maybe_style_title,
        ));

        ops
    }

    pub fn render_border(
        origin_pos: &Position,
        bounds_size: &Size,
        dialog_engine: &mut DialogEngine,
    ) -> RenderOps {
        let mut ops = render_ops!();

        let inner_spaces = SPACER.repeat(ch!(@to_usize bounds_size.col_count - 2));

        let maybe_style = dialog_engine.dialog_options.maybe_style_border;

        for row_idx in 0..*bounds_size.row_count {
            let row_pos = position!(col_index: origin_pos.col_index, row_index: origin_pos.row_index + row_idx);

            let is_first_line = row_idx == 0;
            let is_last_line = row_idx == (*bounds_size.row_count - 1);

            ops.push(RenderOp::ResetColor);
            ops.push(RenderOp::MoveCursorPositionAbs(row_pos));
            ops.push(RenderOp::ApplyColors(maybe_style));

            match (is_first_line, is_last_line) {
                // First line.
                (true, false) => {
                    let mut text_content = Cow::Owned(format!(
                        "{}{}{}",
                        BorderGlyphCharacter::TopLeft.as_ref(),
                        BorderGlyphCharacter::Horizontal
                            .as_ref()
                            .repeat(ch!(@to_usize bounds_size.col_count - 2)),
                        BorderGlyphCharacter::TopRight.as_ref()
                    ));
                    // Apply lolcat override (if enabled) to the fg_color of text_content.
                    apply_lolcat_from_style(
                        &maybe_style,
                        &mut dialog_engine.lolcat,
                        &mut text_content,
                    );

                    ops.push(RenderOp::PaintTextWithAttributes(
                        text_content.into(),
                        maybe_style,
                    ));
                }
                // Last line.
                (false, true) => {
                    let mut text_content = Cow::Owned(format!(
                        "{}{}{}",
                        BorderGlyphCharacter::BottomLeft.as_ref(),
                        BorderGlyphCharacter::Horizontal
                            .as_ref()
                            .repeat(ch!(@to_usize bounds_size.col_count - 2)),
                        BorderGlyphCharacter::BottomRight.as_ref(),
                    ));
                    // Apply lolcat override (if enabled) to the fg_color of text_content.
                    apply_lolcat_from_style(
                        &maybe_style,
                        &mut dialog_engine.lolcat,
                        &mut text_content,
                    );
                    ops.push(RenderOp::PaintTextWithAttributes(
                        text_content.into(),
                        maybe_style,
                    ));
                }
                // Middle line.
                (false, false) => {
                    let mut text_content = Cow::Owned(format!(
                        "{}{}{}",
                        BorderGlyphCharacter::Vertical.as_ref(),
                        inner_spaces,
                        BorderGlyphCharacter::Vertical.as_ref()
                    ));
                    // Apply lolcat override (if enabled) to the fg_color of text_content.
                    apply_lolcat_from_style(
                        &maybe_style,
                        &mut dialog_engine.lolcat,
                        &mut text_content,
                    );
                    ops.push(RenderOp::PaintTextWithAttributes(
                        text_content.into(),
                        maybe_style,
                    ));
                }
                _ => {}
            };
        }

        ops
    }

    pub fn try_handle_dialog_choice(
        input_event: &InputEvent,
        dialog_buffer: &DialogBuffer,
    ) -> Option<DialogChoice> {
        match DialogEvent::from(input_event) {
            // Handle Enter.
            DialogEvent::EnterPressed => {
                let text = dialog_buffer.editor_buffer.get_as_string();
                return Some(DialogChoice::Yes(text));
            }

            // Handle Esc.
            DialogEvent::EscPressed => {
                return Some(DialogChoice::No);
            }
            _ => {}
        }
        None
    }
}

#[cfg(test)]
mod test_dialog_engine_api_render_engine {
    use r3bl_rs_utils_core::*;

    use super::*;
    use crate::test_dialog::mock_real_objects_for_dialog;

    #[tokio::test]
    async fn render_engine() {
        let self_id: FlexBoxId = 0;
        let window_size = &size!( col_count: 70, row_count: 15 );
        let dialog_buffer = &mut DialogBuffer::new_empty();
        let dialog_engine = &mut mock_real_objects_for_dialog::make_dialog_engine();
        let shared_store = &mock_real_objects_for_dialog::create_store();
        let shared_global_data =
            &test_editor::mock_real_objects_for_editor::make_shared_global_data(
                (*window_size).into(),
            );
        let component_registry =
            &mut test_editor::mock_real_objects_for_editor::make_component_registry();
        let state = &shared_store.read().await.state.clone();
        let args = DialogEngineArgs {
            shared_global_data,
            shared_store,
            state,
            component_registry,
            window_size,
            self_id,
            dialog_buffer,
            dialog_engine,
        };

        let pipeline = dbg!(DialogEngine::render_engine(args).await.unwrap());
        assert_eq2!(pipeline.len(), 1);
        let render_ops = pipeline.get(&ZOrder::Glass).unwrap();
        assert!(!render_ops.is_empty());
    }
}

#[cfg(test)]
mod test_dialog_api_make_flex_box_for_dialog {
    use std::error::Error;

    use r3bl_rs_utils_core::*;

    use crate::{dialog_engine_api::internal_impl, *};

    /// More info on `is` and downcasting:
    /// - https://stackoverflow.com/questions/71409337/rust-how-to-match-against-any
    /// - https://ysantos.com/blog/downcast-rust
    #[test]
    fn make_flex_box_for_dialog_simple_display_size_too_small() {
        let surface = Surface::default();
        let window_size = Size::default();
        let dialog_id: FlexBoxId = 0;

        // The window size is too small and will result in this error.
        // Err(
        //   CommonError {
        //       err_type: DisplaySizeTooSmall,
        //       err_msg: Some(
        //           "Window size is too small. Min size is 65 cols x 10 rows",
        //       ),
        //   },
        let result_flex_box = dbg!(internal_impl::make_flex_box_for_dialog(
            &dialog_id,
            &DialogEngineConfigOptions {
                mode: DialogEngineMode::ModalSimple,
                ..Default::default()
            },
            &window_size,
            Some(SurfaceBounds::from(&surface)),
        ));

        // Assert that a general `CommonError` is returned.
        let my_err: Box<dyn Error + Send + Sync> = result_flex_box.err().unwrap();
        assert_eq2!(my_err.is::<CommonError>(), true);

        // Assert that this specific error is returned.
        let result = matches!(
            my_err.downcast_ref::<CommonError>(),
            Some(CommonError {
                err_type: CommonErrorType::DisplaySizeTooSmall,
                err_msg: _,
            })
        );

        assert_eq2!(result, true);
    }

    /// More info on `is` and downcasting:
    /// - https://stackoverflow.com/questions/71409337/rust-how-to-match-against-any
    /// - https://ysantos.com/blog/downcast-rust
    #[test]
    fn make_flex_box_for_dialog_autocomplete_display_size_too_small() {
        let surface = Surface::default();
        let window_size = Size::default();
        let dialog_id: FlexBoxId = 0;

        // The window size is too small and will result in this error.
        // Err(
        //   CommonError {
        //       err_type: DisplaySizeTooSmall,
        //       err_msg: Some(
        //           "Window size is too small. Min size is 65 cols x 10 rows",
        //       ),
        //   },
        let result_flex_box = dbg!(internal_impl::make_flex_box_for_dialog(
            &dialog_id,
            &DialogEngineConfigOptions {
                mode: DialogEngineMode::ModalAutocomplete,
                ..Default::default()
            },
            &window_size,
            Some(SurfaceBounds::from(&surface)),
        ));

        // Assert that a general `CommonError` is returned.
        let my_err: Box<dyn Error + Send + Sync> = result_flex_box.err().unwrap();
        assert_eq2!(my_err.is::<CommonError>(), true);

        // Assert that this specific error is returned.
        let result = matches!(
            my_err.downcast_ref::<CommonError>(),
            Some(CommonError {
                err_type: CommonErrorType::DisplaySizeTooSmall,
                err_msg: _,
            })
        );

        assert_eq2!(result, true);
    }

    #[test]
    fn make_flex_box_for_dialog_simple() {
        // 1. The surface and window_size are not the same width and height.
        // 2. The surface is also not starting from the top left corner of the window.
        let surface = Surface {
            origin_pos: position! { col_index: 2, row_index: 2 },
            box_size: size!( col_count: 65, row_count: 10 ),
            ..Default::default()
        };
        let window_size = size!( col_count: 70, row_count: 15 );
        let self_id: FlexBoxId = 0;

        // The dialog box should be centered inside the surface.
        let result_flex_box = dbg!(internal_impl::make_flex_box_for_dialog(
            &self_id,
            &DialogEngineConfigOptions {
                mode: DialogEngineMode::ModalSimple,
                ..Default::default()
            },
            &window_size,
            Some(SurfaceBounds::from(&surface)),
        ));

        assert_eq2!(result_flex_box.is_ok(), true);

        let flex_box = result_flex_box.unwrap();
        assert_eq2!(flex_box.id, self_id);
        assert_eq2!(
            flex_box.style_adjusted_bounds_size,
            size!( col_count: 58, row_count: 4 )
        );
        assert_eq2!(
            flex_box.style_adjusted_origin_pos,
            position!( col_index: 5, row_index: 5 )
        );
    }

    #[test]

    fn make_flex_box_for_dialog_autocomplete() {
        // 1. The surface and window_size are not the same width and height.
        // 2. The surface is also not starting from the top left corner of the window.
        let surface = Surface {
            origin_pos: position! { col_index: 2, row_index: 2 },
            box_size: size!( col_count: 65, row_count: 10 ),
            ..Default::default()
        };
        let window_size = size!( col_count: 70, row_count: 15 );
        let self_id: FlexBoxId = 0;

        // The dialog box should be centered inside the surface.
        let result_flex_box = dbg!(internal_impl::make_flex_box_for_dialog(
            &self_id,
            &DialogEngineConfigOptions {
                mode: DialogEngineMode::ModalAutocomplete,
                ..Default::default()
            },
            &window_size,
            Some(SurfaceBounds::from(&surface)),
        ));

        assert_eq2!(result_flex_box.is_ok(), true);

        let flex_box = result_flex_box.unwrap();
        assert_eq2!(flex_box.id, self_id);
        assert_eq2!(
            flex_box.style_adjusted_bounds_size,
            size!( col_count: 58, row_count: 10 )
        );
        assert_eq2!(
            flex_box.style_adjusted_origin_pos,
            position!( col_index: 5, row_index: 2 )
        );
    }
}

#[cfg(test)]
mod test_dialog_engine_api_apply_event {
    use r3bl_rs_utils_core::*;

    use super::*;
    use crate::test_dialog::mock_real_objects_for_dialog;

    #[tokio::test]
    async fn apply_event_esc() {
        let self_id: FlexBoxId = 0;
        let window_size = &size!( col_count: 70, row_count: 15 );
        let dialog_buffer = &mut DialogBuffer::new_empty();
        let dialog_engine = &mut mock_real_objects_for_dialog::make_dialog_engine();
        let shared_store = &mock_real_objects_for_dialog::create_store();
        let shared_global_data =
            &test_editor::mock_real_objects_for_editor::make_shared_global_data(
                (*window_size).into(),
            );
        let component_registry =
            &mut test_editor::mock_real_objects_for_editor::make_component_registry();
        let state = &shared_store.read().await.state.clone();
        let args = DialogEngineArgs {
            shared_global_data,
            shared_store,
            state,
            component_registry,
            window_size,
            self_id,
            dialog_buffer,
            dialog_engine,
        };

        let input_event = InputEvent::Keyboard(keypress!(@special SpecialKey::Esc));
        let response = dbg!(DialogEngine::apply_event(args, &input_event).await.unwrap());
        assert!(matches!(
            response,
            DialogEngineApplyResponse::DialogChoice(DialogChoice::No)
        ));
    }

    #[tokio::test]
    async fn apply_event_enter() {
        let self_id: FlexBoxId = 0;
        let window_size = &size!( col_count: 70, row_count: 15 );
        let dialog_buffer = &mut DialogBuffer::new_empty();
        let dialog_engine = &mut mock_real_objects_for_dialog::make_dialog_engine();
        let shared_store = &mock_real_objects_for_dialog::create_store();
        let shared_global_data =
            &test_editor::mock_real_objects_for_editor::make_shared_global_data(
                (*window_size).into(),
            );
        let component_registry =
            &mut test_editor::mock_real_objects_for_editor::make_component_registry();
        let state = &shared_store.read().await.state.clone();
        let args = DialogEngineArgs {
            shared_global_data,
            shared_store,
            state,
            component_registry,
            window_size,
            self_id,
            dialog_buffer,
            dialog_engine,
        };

        let input_event = InputEvent::Keyboard(keypress!(@special SpecialKey::Enter));
        let response = dbg!(DialogEngine::apply_event(args, &input_event).await.unwrap());
        if let DialogEngineApplyResponse::DialogChoice(DialogChoice::Yes(value)) = &response {
            assert_eq2!(value, "");
        }
        assert!(matches!(
            response,
            DialogEngineApplyResponse::DialogChoice(DialogChoice::Yes(_))
        ));
    }

    #[tokio::test]
    async fn apply_event_other_key() {
        let self_id: FlexBoxId = 0;
        let window_size = &size!( col_count: 70, row_count: 15 );
        let dialog_buffer = &mut DialogBuffer::new_empty();
        let dialog_engine = &mut mock_real_objects_for_dialog::make_dialog_engine();
        let shared_store = &mock_real_objects_for_dialog::create_store();
        let shared_global_data =
            &test_editor::mock_real_objects_for_editor::make_shared_global_data(
                (*window_size).into(),
            );
        let component_registry =
            &mut test_editor::mock_real_objects_for_editor::make_component_registry();
        let state = &shared_store.read().await.state.clone();
        let args = DialogEngineArgs {
            shared_global_data,
            shared_store,
            state,
            component_registry,
            window_size,
            self_id,
            dialog_buffer,
            dialog_engine,
        };

        let input_event = InputEvent::Keyboard(keypress!(@char 'a'));
        let response = dbg!(DialogEngine::apply_event(args, &input_event).await.unwrap());
        if let DialogEngineApplyResponse::UpdateEditorBuffer(editor_buffer) = &response {
            assert_eq2!(editor_buffer.get_as_string(), "a");
        }
    }
}
