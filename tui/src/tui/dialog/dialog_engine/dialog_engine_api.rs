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
use std::{borrow::Cow, fmt::Debug};

use crate::{ch,
            col,
            editor_engine::engine_public_api,
            glyphs::SPACER_GLYPH as SPACER,
            height,
            inline_string,
            pc,
            render_ops,
            render_pipeline,
            render_tui_styled_texts_into,
            row,
            terminal_lib_backends::KeyPress,
            throws_with_return,
            tui_style_attrib,
            u16,
            usize,
            width,
            BorderGlyphCharacter,
            ColorWheel,
            CommonError,
            CommonErrorType,
            CommonResult,
            DialogBuffer,
            DialogChoice,
            DialogEngine,
            DialogEngineArgs,
            DialogEngineConfigOptions,
            DialogEngineMode,
            DialogEvent,
            EditorEngineApplyEventResult,
            EventPropagation,
            FlexBox,
            FlexBoxId,
            GCStringExt as _,
            GlobalData,
            GradientGenerationPolicy,
            HasDialogBuffers,
            InlineString,
            InputEvent,
            Key,
            MinSize,
            PartialFlexBox,
            Pos,
            RenderOp,
            RenderOps,
            RenderPipeline,
            Size,
            SpecialKey,
            SurfaceBounds,
            SystemClipboard,
            TextColorizationPolicy,
            TuiStyle,
            ZOrder};

#[derive(Debug)]
pub enum DialogEngineApplyResponse {
    UpdateEditorBuffer,
    DialogChoice(DialogChoice),
    SelectScrollResultsPanel,
    Noop,
}

/// Things you can do with a [DialogEngine].
pub struct DialogEngineApi;

impl DialogEngineApi {
    pub fn render_engine<S, AS>(
        args: DialogEngineArgs<'_, S, AS>,
    ) -> CommonResult<RenderPipeline>
    where
        S: Debug + Default + Clone + Sync + Send + HasDialogBuffers,
        AS: Debug + Default + Clone + Sync + Send,
    {
        // Unpack local scope data.
        let DialogEngineArgs {
            self_id,
            global_data,
            engine: dialog_engine,
            has_focus,
        } = args;

        // Unpack global data.
        let GlobalData { state, .. } = global_data;

        let mode = dialog_engine.dialog_options.mode;
        let overlay_flex_box: PartialFlexBox = {
            let window_size = global_data.window_size;
            match &dialog_engine.maybe_flex_box {
                // No need to calculate new flex box if:
                // 1) there's an existing one & 2) the window size hasn't changed.
                Some((saved_size, saved_mode, saved_box))
                    if *saved_size == window_size && saved_mode == &mode =>
                {
                    *saved_box
                }
                // Otherwise, calculate a new flex box & save it.
                _ => {
                    let it = internal_impl::make_flex_box_for_dialog(
                        self_id,
                        dialog_engine.dialog_options,
                        window_size,
                        dialog_engine.maybe_surface_bounds,
                    )?;

                    dialog_engine
                        .maybe_flex_box
                        .replace((window_size, mode, it));

                    it
                }
            }
        };

        let (origin_pos, bounds_size) = overlay_flex_box.get_style_adjusted_pos_and_dim();

        let pipeline = {
            let mut it = render_pipeline!();

            it.push(
                ZOrder::Glass,
                internal_impl::render_border(origin_pos, bounds_size, dialog_engine),
            );

            // Paint title.
            let title = if let Some(dialog_buffer) = state.get_mut_dialog_buffer(self_id)
            {
                &dialog_buffer.title
            } else {
                "N/A"
            };
            it.push(
                ZOrder::Glass,
                internal_impl::render_title(
                    origin_pos,
                    bounds_size,
                    title,
                    dialog_engine,
                ),
            );

            // Call render_results_panel() if mode is autocomplete.
            if matches!(
                dialog_engine.dialog_options.mode,
                DialogEngineMode::ModalAutocomplete
            ) {
                let results_panel_ops = internal_impl::render_results_panel(
                    origin_pos,
                    bounds_size,
                    dialog_engine,
                    self_id,
                    state,
                )?;
                if !results_panel_ops.is_empty() {
                    it.push(ZOrder::Glass, results_panel_ops);
                }
            }

            it += internal_impl::render_editor(
                origin_pos,
                bounds_size,
                DialogEngineArgs {
                    self_id,
                    global_data,
                    engine: dialog_engine,
                    has_focus,
                },
            )?;

            it
        };

        Ok(pipeline)
    }

    /// Event based interface for the editor. This executes the [InputEvent] and returns
    /// one of the following:
    /// - [DialogEngineApplyResponse::DialogChoice] => <kbd>Enter</kbd> or <kbd>Esc</kbd>
    ///   was pressed.
    /// - [DialogEngineApplyResponse::UpdateEditorBuffer] => the editor buffer was
    ///   updated.
    /// - [DialogEngineApplyResponse::Noop] => otherwise.
    pub fn apply_event<S, AS>(
        mut_state: &mut S,
        self_id: FlexBoxId,
        dialog_engine: &mut DialogEngine,
        input_event: InputEvent,
    ) -> CommonResult<DialogEngineApplyResponse>
    where
        S: Debug + Default + Clone + Sync + Send + HasDialogBuffers,
        AS: Debug + Default + Clone + Sync + Send,
    {
        // Was a dialog choice made?
        if let Some(choice) = internal_impl::try_handle_dialog_choice(
            input_event,
            mut_state.get_mut_dialog_buffer(self_id),
            dialog_engine,
        ) {
            dialog_engine.reset();
            return Ok(DialogEngineApplyResponse::DialogChoice(choice));
        }

        // Was up / down pressed to select autocomplete results & vert scroll the results
        // panel?
        if let EventPropagation::ConsumedRender = internal_impl::try_handle_up_down(
            input_event,
            mut_state.get_mut_dialog_buffer(self_id),
            dialog_engine,
        ) {
            return Ok(DialogEngineApplyResponse::SelectScrollResultsPanel);
        }

        // Otherwise, pass the event to the editor engine.

        // It is safe to unwrap the dialog buffer here (since it will have Some value).
        let dialog_buffer = {
            let it = mut_state.get_mut_dialog_buffer(self_id);
            match it {
                Some(it) => it,
                None => return Ok(DialogEngineApplyResponse::Noop),
            }
        };

        let result = engine_public_api::apply_event(
            &mut dialog_buffer.editor_buffer,
            &mut dialog_engine.editor_engine,
            input_event,
            &mut SystemClipboard,
        )?;

        match result {
            // If the editor engine applied the event, return the new editor buffer.
            EditorEngineApplyEventResult::Applied => {
                Ok(DialogEngineApplyResponse::UpdateEditorBuffer)
            }
            _ =>
            // Otherwise, return noop.
            {
                Ok(DialogEngineApplyResponse::Noop)
            }
        }
    }
}

#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DisplayConstants {
    DialogComponentBorderWidthPercent = 90,
    /// border-top, title, input, border-bottom.
    SimpleModalRowCount = 4,
    EmptyLine = 1,
    DefaultResultsPanelRowCount = 5,
}

mod internal_impl {
    use super::*;

    /// Return the [FlexBox] for the dialog to be rendered in.
    ///
    /// - In non-modal contexts (which this is not), this is determined by the layout
    ///   engine.
    /// - In the modal case (which this is), things are different because the dialog
    ///   escapes the boundaries of the layout engine and really just paints itself on top
    ///   of everything. It can reach any corner of the screen.
    ///   - In autocomplete mode it sizes itself differently than in normal mode.
    /// - However, it is still constrained by the bounds of the [Surface] itself and does
    ///   not take into account the full window size (in case these are different). This
    ///   only applies if a [Surface] is passed in as an argument.
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
        dialog_id: FlexBoxId,
        dialog_options: DialogEngineConfigOptions,
        window_size: Size,
        maybe_surface_bounds: Option<SurfaceBounds>,
    ) -> CommonResult<PartialFlexBox> {
        let surface_size = if let Some(surface_bounds) = maybe_surface_bounds {
            surface_bounds.box_size
        } else {
            window_size
        };

        let surface_origin_pos = if let Some(surface_bounds) = maybe_surface_bounds {
            surface_bounds.origin_pos
        } else {
            col(0) + row(0)
        };

        // Check to ensure that the dialog box has enough space to be displayed.
        if window_size.col_width < width(MinSize::Col as u8)
            || window_size.row_height < height(MinSize::Row as u8)
        {
            return CommonError::new_error_result(
                CommonErrorType::DisplaySizeTooSmall,
                &format!(
                    "Window size is too small. Min size is {} cols x {} rows",
                    MinSize::Col as u8,
                    MinSize::Row as u8
                ),
            );
        }

        let (origin_pos, bounds_size) = match dialog_options.mode {
            DialogEngineMode::ModalSimple => {
                let simple_dialog_size = {
                    // Calc dialog bounds size based on window size.
                    let col_count = {
                        let percent =
                            pc!(DisplayConstants::DialogComponentBorderWidthPercent
                                as u16)?;
                        width(percent.apply_to(*surface_size.col_width))
                    };
                    let row_count = height(DisplayConstants::SimpleModalRowCount as u16);
                    let size = col_count + row_count;
                    debug_assert!(size.row_height < height(MinSize::Row as u8));
                    size
                };

                let origin_pos = {
                    // Calc origin position based on window size & dialog size.
                    let origin_col = col(*surface_size.col_width / ch(2)
                        - *simple_dialog_size.col_width / ch(2));
                    let origin_row = row(*surface_size.row_height / ch(2)
                        - *simple_dialog_size.row_height / ch(2));
                    let mut it = origin_col + origin_row;
                    it += surface_origin_pos;
                    it
                };

                (origin_pos, simple_dialog_size)
            }
            DialogEngineMode::ModalAutocomplete => {
                let autocomplete_dialog_size = {
                    // Calc dialog bounds size based on window size.
                    let row_count = height(DisplayConstants::SimpleModalRowCount as u16)
                        + height(DisplayConstants::EmptyLine as u16)
                        + dialog_options.result_panel_display_row_count;
                    let col_count = {
                        let percent =
                            pc!(DisplayConstants::DialogComponentBorderWidthPercent
                                as u16)?;
                        width(percent.apply_to(*surface_size.col_width))
                    };
                    let size = col_count + row_count;
                    debug_assert!(size.row_height < height(MinSize::Row as u8));
                    size
                };

                let origin_pos = {
                    // Calc origin position based on window size & dialog size.
                    let origin_col = col(*surface_size.col_width / ch(2)
                        - *autocomplete_dialog_size.col_width / ch(2));
                    let origin_row = row(*surface_size.row_height / ch(2)
                        - *autocomplete_dialog_size.row_height / ch(2));
                    let mut it = origin_col + origin_row;
                    it += surface_origin_pos;
                    it
                };

                (origin_pos, autocomplete_dialog_size)
            }
        };

        throws_with_return!({
            PartialFlexBox {
                id: dialog_id,
                style_adjusted_origin_pos: origin_pos,
                style_adjusted_bounds_size: bounds_size,
                maybe_computed_style: None,
            }
        })
    }

    pub fn render_editor<S, AS>(
        origin_pos: Pos,
        bounds_size: Size,
        args: DialogEngineArgs<'_, S, AS>,
    ) -> CommonResult<RenderPipeline>
    where
        S: Debug + Default + Clone + Sync + Send + HasDialogBuffers,
        AS: Debug + Default + Clone + Sync + Send,
    {
        let DialogEngineArgs {
            self_id,
            global_data,
            engine: dialog_engine,
            has_focus,
        } = args;

        let GlobalData { state, .. } = global_data;

        let maybe_style = dialog_engine.dialog_options.maybe_style_editor;

        let flex_box: FlexBox = {
            let origin_pos_width = origin_pos.col_index + width(1);
            let origin_pos_height = origin_pos.row_index + height(2);
            let origin_pos = origin_pos_width + origin_pos_height;

            let bounds_size_width = {
                let it = bounds_size.col_width - width(2);
                width(*it)
            };

            let bounds_size_height = height(1);
            let bounds_size = bounds_size_width + bounds_size_height;

            PartialFlexBox {
                id: self_id,
                style_adjusted_origin_pos: origin_pos,
                style_adjusted_bounds_size: bounds_size,
                maybe_computed_style: maybe_style,
            }
            .into()
        };

        let dialog_buffer = {
            let it = state.get_mut_dialog_buffer(self_id);
            match it {
                Some(it) => it,
                None => {
                    return CommonError::new_error_result(
                        CommonErrorType::NotFound,
                        &format!(
                            "Dialog buffer does not exist for component id:{self_id:?}"
                        ),
                    )
                }
            }
        };

        let mut pipeline = engine_public_api::render_engine(
            &mut dialog_engine.editor_engine,
            &mut dialog_buffer.editor_buffer,
            flex_box,
            has_focus,
            global_data.window_size,
        )?;

        pipeline.hoist(ZOrder::Normal, ZOrder::Glass);

        // Paint hint.
        if dialog_buffer.editor_buffer.is_empty()
            || dialog_buffer
                .editor_buffer
                .get_as_string_with_comma_instead_of_newlines()
                == ""
        {
            let mut ops = render_ops!();
            let msg = "Press <Esc> to close, or <Enter> to accept".to_string();

            ops.push(RenderOp::ResetColor);
            ops.push(RenderOp::MoveCursorPositionAbs(
                flex_box.style_adjusted_origin_pos,
            ));

            ops.push(RenderOp::PaintTextWithAttributes(
                msg.into(),
                Some(if let Some(style) = maybe_style {
                    TuiStyle {
                        dim: Some(tui_style_attrib::Dim),
                        ..style
                    }
                } else {
                    TuiStyle {
                        dim: Some(tui_style_attrib::Dim),
                        ..Default::default()
                    }
                }),
            ));

            pipeline.push(ZOrder::Glass, ops);
        };

        Ok(pipeline)
    }

    pub fn render_results_panel<S>(
        origin_pos: Pos,
        bounds_size: Size,
        dialog_engine: &DialogEngine,
        self_id: FlexBoxId,
        state: &mut S,
    ) -> CommonResult<RenderOps>
    where
        S: Default + Clone + Debug + Sync + Send + HasDialogBuffers,
    {
        let mut it = render_ops!();

        if let Some(dialog_buffer) = state.get_mut_dialog_buffer(self_id) {
            if let Some(results) = dialog_buffer.maybe_results.as_ref() {
                if !results.is_empty() {
                    paint_results(
                        &mut it,
                        origin_pos,
                        bounds_size,
                        results,
                        dialog_engine,
                    );
                }
            }
        };

        return Ok(it);

        pub fn paint_results(
            ops: &mut RenderOps,
            origin_pos: Pos,
            bounds_size: Size,
            results: &[InlineString],
            dialog_engine: &DialogEngine,
        ) {
            let col_start_index = col(1);
            let row_start_index =
                row(DisplayConstants::SimpleModalRowCount as u16) - row(1);

            let mut rel_insertion_pos = col_start_index + row_start_index;

            let scroll_offset_row_index = dialog_engine.scroll_offset_row_index;
            let selected_row_index = dialog_engine.selected_row_index;

            // Print results panel.
            for (row_index, item) in results.iter().enumerate() {
                let row_index = row(row_index);

                // Skip rows that are above the scroll offset.
                if row_index < scroll_offset_row_index {
                    continue;
                }

                rel_insertion_pos.add_row(height(1));

                let text = item.as_str();
                let text_gcs = text.grapheme_string();
                let text_display_width = text_gcs.display_width;

                let max_display_col_count = bounds_size.col_width - width(2);

                let clipped_text = if text_display_width > max_display_col_count {
                    let snip_len = width(2); /* `..` */
                    let postfix_len = width(5); /* last 5 characters */

                    let lhs_start_index = col(0);
                    let lhs_end_width = max_display_col_count - postfix_len - snip_len;
                    let lhs_str = text_gcs.clip(lhs_start_index, lhs_end_width);

                    // This is calculated relative to the end of the string (not the
                    // start!). So it's backwards.
                    let rhs_start_index = (text_display_width - postfix_len)
                        .convert_to_col_index()
                        + col(1) /* skip one segment right */;
                    let rhs_str = text_gcs.clip(rhs_start_index, text_display_width);

                    Cow::Owned(inline_string!("{lhs_str}..{rhs_str}"))
                } else {
                    Cow::Borrowed(item)
                };

                let max_display_row_index = {
                    let viewport_height =
                        dialog_engine.dialog_options.result_panel_display_row_count;
                    scroll_offset_row_index + viewport_height
                };

                if row_index >= max_display_row_index {
                    break;
                }

                ops.push(RenderOp::ResetColor);
                ops.push(RenderOp::MoveCursorPositionRelTo(
                    origin_pos,
                    rel_insertion_pos,
                ));

                // Set style to underline if selected row & paint.
                match selected_row_index.eq(&row_index) {
                    // This is the selected row.
                    true => {
                        let my_selected_style = match dialog_engine
                            .dialog_options
                            .maybe_style_results_panel
                        {
                            // Update existing style.
                            Some(style) => TuiStyle {
                                underline: Some(tui_style_attrib::Underline),
                                ..style
                            },
                            // No existing style, so create a new style w/ only underline.
                            _ => TuiStyle {
                                underline: Some(tui_style_attrib::Underline),
                                ..Default::default()
                            },
                        }
                        .into();
                        // Paint the text for the row.
                        ops.push(RenderOp::ApplyColors(my_selected_style));
                        ops.push(RenderOp::PaintTextWithAttributes(
                            clipped_text.into_owned(),
                            my_selected_style,
                        ));
                    }
                    // Regular row, not selected.
                    false => {
                        // Paint the text for the row.
                        ops.push(RenderOp::ApplyColors(
                            dialog_engine.dialog_options.maybe_style_results_panel,
                        ));
                        ops.push(RenderOp::PaintTextWithAttributes(
                            clipped_text.into_owned(),
                            dialog_engine.dialog_options.maybe_style_results_panel,
                        ));
                    }
                }
            }
        }
    }

    pub fn render_title(
        origin_pos: Pos,
        bounds_size: Size,
        title: &str,
        dialog_engine: &mut DialogEngine,
    ) -> RenderOps {
        let mut ops = render_ops!();

        let row_pos = {
            let col_index = origin_pos.col_index + col(1);
            let row_index = origin_pos.row_index + row(1);
            col_index + row_index
        };

        let title_gcs = title.grapheme_string();
        let title_content_clipped =
            title_gcs.trunc_end_to_fit(bounds_size.col_width - width(2));

        ops.push(RenderOp::ResetColor);
        ops.push(RenderOp::MoveCursorPositionAbs(row_pos));
        ops.push(RenderOp::ApplyColors(
            dialog_engine.dialog_options.maybe_style_title,
        ));

        // Apply lolcat override (if enabled) to the fg_color of text_content.
        lolcat_from_style(
            &mut ops,
            &mut dialog_engine.color_wheel,
            &dialog_engine.dialog_options.maybe_style_title,
            title_content_clipped,
        );

        ops
    }

    /// Only Colorizes text in-place if [Style]'s `lolcat` field is true. Otherwise leaves
    /// `text` alone.
    fn lolcat_from_style(
        ops: &mut RenderOps,
        color_wheel: &mut ColorWheel,
        maybe_style: &Option<TuiStyle>,
        text: &str,
    ) {
        // If lolcat is enabled, then colorize the text.
        if let Some(style) = maybe_style {
            if style.lolcat.is_some() {
                let text_gcs = text.grapheme_string();
                let texts = color_wheel.colorize_into_styled_texts(
                    &text_gcs,
                    GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
                    TextColorizationPolicy::ColorEachCharacter(*maybe_style),
                );
                render_tui_styled_texts_into(&texts, ops);
                return;
            }
        }

        // Otherwise, just paint the text as-is.
        ops.push(RenderOp::PaintTextWithAttributes(text.into(), *maybe_style));
    }

    pub fn render_border(
        origin_pos: Pos,
        bounds_size: Size,
        dialog_engine: &mut DialogEngine,
    ) -> RenderOps {
        let mut ops = render_ops!();

        let inner_spaces = SPACER.repeat({
            let it = bounds_size.col_width - width(2);
            usize(*it)
        });

        let maybe_style = dialog_engine.dialog_options.maybe_style_border;

        let max_row_idx = u16(*bounds_size.row_height);

        for row_idx in 0..max_row_idx {
            let row_pos = {
                let col_index = origin_pos.col_index;
                let row_index = origin_pos.row_index + row(row_idx);
                col_index + row_index
            };

            enum IsFirstLine {
                Yes,
                No,
            }

            enum IsLastLine {
                Yes,
                No,
            }

            let is_first_line = if row_idx == 0 {
                IsFirstLine::Yes
            } else {
                IsFirstLine::No
            };

            let is_last_line = if row_idx == u16(*bounds_size.row_height - ch(1)) {
                IsLastLine::Yes
            } else {
                IsLastLine::No
            };

            ops.push(RenderOp::ResetColor);
            ops.push(RenderOp::MoveCursorPositionAbs(row_pos));
            ops.push(RenderOp::ApplyColors(maybe_style));

            match (is_first_line, is_last_line) {
                // First line.
                (IsFirstLine::Yes, IsLastLine::No) => {
                    let text_content = format!(
                        "{}{}{}",
                        BorderGlyphCharacter::TopLeft.as_ref(),
                        BorderGlyphCharacter::Horizontal
                            .as_ref()
                            .repeat(usize(*bounds_size.col_width - ch(2))),
                        BorderGlyphCharacter::TopRight.as_ref()
                    );

                    // Apply lolcat override (if enabled) to the fg_color of text_content.
                    lolcat_from_style(
                        &mut ops,
                        &mut dialog_engine.color_wheel,
                        &maybe_style,
                        &text_content,
                    );
                }

                // Middle line.
                (IsFirstLine::No, IsLastLine::No) => {
                    let text_content = format!(
                        "{}{}{}",
                        BorderGlyphCharacter::Vertical.as_ref(),
                        inner_spaces,
                        BorderGlyphCharacter::Vertical.as_ref()
                    );
                    // Apply lolcat override (if enabled) to the fg_color of text_content.
                    lolcat_from_style(
                        &mut ops,
                        &mut dialog_engine.color_wheel,
                        &maybe_style,
                        &text_content,
                    );
                }

                // Last line.
                (IsFirstLine::No, IsLastLine::Yes) => {
                    // Paint bottom border.
                    let text_content = format!(
                        "{}{}{}",
                        BorderGlyphCharacter::BottomLeft.as_ref(),
                        BorderGlyphCharacter::Horizontal
                            .as_ref()
                            .repeat(usize(*bounds_size.col_width - ch(2))),
                        BorderGlyphCharacter::BottomRight.as_ref(),
                    );
                    // Apply lolcat override (if enabled) to the fg_color of text_content.
                    lolcat_from_style(
                        &mut ops,
                        &mut dialog_engine.color_wheel,
                        &maybe_style,
                        &text_content,
                    );
                }
                _ => {}
            };

            // Paint separator for results panel if in autocomplete mode.
            match dialog_engine.dialog_options.mode {
                DialogEngineMode::ModalSimple => {}
                DialogEngineMode::ModalAutocomplete => {
                    let inner_line = BorderGlyphCharacter::Horizontal
                        .as_ref()
                        .repeat(usize(*bounds_size.col_width - ch(2)))
                        .to_string();

                    let text_content = format!(
                        "{}{}{}",
                        BorderGlyphCharacter::LineUpDownRight.as_ref(),
                        inner_line,
                        BorderGlyphCharacter::LineUpDownLeft.as_ref()
                    );

                    let col_start_index = col(0);
                    let row_start_index =
                        row(DisplayConstants::SimpleModalRowCount as u16 - 1);
                    let rel_insertion_pos = col_start_index + row_start_index;

                    ops.push(RenderOp::ResetColor);
                    ops.push(RenderOp::MoveCursorPositionRelTo(
                        origin_pos,
                        rel_insertion_pos,
                    ));

                    // Apply lolcat override (if enabled) to the fg_color of text_content.
                    lolcat_from_style(
                        &mut ops,
                        &mut dialog_engine.color_wheel,
                        &maybe_style,
                        &text_content,
                    );
                }
            }
        }

        ops
    }

    pub fn try_handle_dialog_choice(
        input_event: InputEvent,
        maybe_dialog_buffer: Option<&mut DialogBuffer>,
        dialog_engine: &mut DialogEngine,
    ) -> Option<DialogChoice> {
        // It is safe to unwrap the dialog buffer here (since it will have Some value).
        let dialog_buffer = { maybe_dialog_buffer? };

        match DialogEvent::from(input_event) {
            // Handle Enter.
            DialogEvent::EnterPressed => match dialog_engine.dialog_options.mode {
                DialogEngineMode::ModalSimple => {
                    let text = dialog_buffer
                        .editor_buffer
                        .get_as_string_with_comma_instead_of_newlines();
                    return Some(DialogChoice::Yes(text));
                }

                DialogEngineMode::ModalAutocomplete => {
                    let selected_index = usize(*dialog_engine.selected_row_index);
                    if let Some(results) = &dialog_buffer.maybe_results {
                        if let Some(selected_result) = results.get(selected_index) {
                            return Some(DialogChoice::Yes(selected_result.clone()));
                        }
                    }
                    return Some(DialogChoice::No);
                }
            },

            // Handle Esc.
            DialogEvent::EscPressed => {
                return Some(DialogChoice::No);
            }
            _ => {}
        }
        None
    }

    pub fn try_handle_up_down(
        input_event: InputEvent,
        maybe_dialog_buffer: Option<&mut DialogBuffer>,
        dialog_engine: &mut DialogEngine,
    ) -> EventPropagation {
        // It is safe to unwrap the dialog buffer here (since it will have Some value).
        let dialog_buffer = {
            if let Some(it) = maybe_dialog_buffer {
                it
            } else {
                return EventPropagation::Propagate;
            }
        };

        // Handle up arrow?
        if input_event.matches(&[InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Up),
        })]) {
            if *dialog_engine.selected_row_index > ch(0) {
                *dialog_engine.selected_row_index -= 1;
            }

            if dialog_engine.selected_row_index < dialog_engine.scroll_offset_row_index {
                *dialog_engine.scroll_offset_row_index -= 1;
            }

            return EventPropagation::ConsumedRender;
        }

        // Handle down arrow?
        if input_event.matches(&[InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Down),
        })]) {
            let max_abs_row_index = dialog_buffer.get_results_count() - ch(1);

            let results_panel_viewport_height_row_count =
                dialog_engine.dialog_options.result_panel_display_row_count;

            if *dialog_engine.selected_row_index < max_abs_row_index {
                *dialog_engine.selected_row_index += 1;
            }

            if dialog_engine.selected_row_index
                >= dialog_engine.scroll_offset_row_index
                    + results_panel_viewport_height_row_count
            {
                *dialog_engine.scroll_offset_row_index += 1;
            }

            return EventPropagation::ConsumedRender;
        }

        EventPropagation::Propagate
    }
}

#[cfg(test)]
mod test_dialog_engine_api_render_engine {
    use super::*;
    use crate::{assert_eq2,
                test_dialog::mock_real_objects_for_dialog::{self, make_global_data},
                HasFocus};

    #[test]
    fn render_engine_with_no_dialog_buffer_in_state() {
        let self_id: FlexBoxId = FlexBoxId::from(0);
        let window_size = width(70) + height(15);
        let dialog_engine = &mut mock_real_objects_for_dialog::make_dialog_engine();
        let global_data = &mut {
            let (mut it, _) = make_global_data(Some(window_size));
            it.state.dialog_buffers.clear();
            it
        };
        let has_focus = &mut HasFocus::default();
        let args = DialogEngineArgs {
            self_id,
            global_data,
            engine: dialog_engine,
            has_focus,
        };
        assert_eq2!(DialogEngineApi::render_engine(args).is_err(), true);
    }

    #[test]
    fn render_engine_with_dialog_buffer_in_state() {
        let self_id: FlexBoxId = FlexBoxId::from(0);
        let window_size = width(70) + height(15);
        let dialog_engine = &mut mock_real_objects_for_dialog::make_dialog_engine();
        let global_data = &mut {
            let (it, _) = make_global_data(Some(window_size));
            it
        };
        let has_focus = &mut HasFocus::default();
        let args = DialogEngineArgs {
            self_id,
            global_data,
            engine: dialog_engine,
            has_focus,
        };
        let pipeline = dbg!(DialogEngineApi::render_engine(args).unwrap());
        assert_eq2!(pipeline.len(), 1);
        let render_ops = pipeline.get(&ZOrder::Glass).unwrap();
        assert!(!render_ops.is_empty());
    }
}

#[cfg(test)]
mod test_dialog_api_make_flex_box_for_dialog {
    use super::*;
    use crate::{assert_eq2, dialog_engine_api::internal_impl, Surface};

    /// More info on `is` and downcasting:
    /// - https://stackoverflow.com/questions/71409337/rust-how-to-match-against-any
    /// - https://ysantos.com/blog/downcast-rust
    #[test]
    fn make_flex_box_for_dialog_simple_display_size_too_small() {
        let surface = Surface::default();
        let window_size = Size::default();
        let dialog_id: FlexBoxId = FlexBoxId::from(0);

        // The window size is too small and will result in this error.
        // Err(
        //   CommonError {
        //       err_type: DisplaySizeTooSmall,
        //       err_msg: Some(
        //           "Window size is too small. Min size is 65 cols x 10 rows",
        //       ),
        //   },
        let result_flex_box = dbg!(internal_impl::make_flex_box_for_dialog(
            dialog_id,
            DialogEngineConfigOptions {
                mode: DialogEngineMode::ModalSimple,
                ..Default::default()
            },
            window_size,
            Some(SurfaceBounds::from(&surface)),
        ));

        // Assert that a general `CommonError` is returned.
        let my_err = result_flex_box.err().unwrap();
        // More info on downcast_ref::<T>(): https://gemini.google.com/app/fd537ea573f1d1fb
        assert_eq2!(my_err.is::<CommonError>(), true);

        // Assert that this specific error is returned.
        let result = matches!(
            my_err.downcast_ref::<CommonError>(),
            Some(CommonError {
                error_type: CommonErrorType::DisplaySizeTooSmall,
                error_message: _,
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
        let dialog_id: FlexBoxId = FlexBoxId::from(0);

        // The window size is too small and will result in this error.
        // Err(
        //   CommonError {
        //       err_type: DisplaySizeTooSmall,
        //       err_msg: Some(
        //           "Window size is too small. Min size is 65 cols x 10 rows",
        //       ),
        //   },
        let result_flex_box = dbg!(internal_impl::make_flex_box_for_dialog(
            dialog_id,
            DialogEngineConfigOptions {
                mode: DialogEngineMode::ModalAutocomplete,
                ..Default::default()
            },
            window_size,
            Some(SurfaceBounds::from(&surface)),
        ));

        // Assert that a general `CommonError` is returned.
        let my_err = result_flex_box.err().unwrap();
        // More info on downcast_ref::<T>(): https://gemini.google.com/app/fd537ea573f1d1fb
        assert_eq2!(my_err.is::<CommonError>(), true);

        // Assert that this specific error is returned.
        let result = matches!(
            my_err.downcast_ref::<CommonError>(),
            Some(CommonError {
                error_type: CommonErrorType::DisplaySizeTooSmall,
                error_message: _,
            })
        );

        assert_eq2!(result, true);
    }

    #[test]
    fn make_flex_box_for_dialog_simple() {
        // 1. The surface and window_size are not the same width and height.
        // 2. The surface is also not starting from the top left corner of the window.
        let surface = Surface {
            origin_pos: col(2) + row(2),
            box_size: width(65) + height(10),
            ..Default::default()
        };
        let window_size = width(70) + height(15);
        let self_id: FlexBoxId = FlexBoxId::from(0);

        // The dialog box should be centered inside the surface.
        let result_flex_box = dbg!(internal_impl::make_flex_box_for_dialog(
            self_id,
            DialogEngineConfigOptions {
                mode: DialogEngineMode::ModalSimple,
                ..Default::default()
            },
            window_size,
            Some(SurfaceBounds::from(&surface)),
        ));

        assert_eq2!(result_flex_box.is_ok(), true);

        let flex_box = result_flex_box.unwrap();
        assert_eq2!(flex_box.id, self_id);
        assert_eq2!(flex_box.style_adjusted_bounds_size, width(58) + height(4));
        assert_eq2!(flex_box.style_adjusted_origin_pos, col(5) + row(5));
    }

    #[test]
    fn make_flex_box_for_dialog_autocomplete() {
        // 1. The surface and window_size are not the same width and height.
        // 2. The surface is also not starting from the top left corner of the window.
        let surface = Surface {
            origin_pos: col(2) + row(2),
            box_size: width(65) + height(10),
            ..Default::default()
        };
        let window_size = width(70) + height(15);
        let self_id: FlexBoxId = FlexBoxId::from(0);

        // The dialog box should be centered inside the surface.
        let result_flex_box = dbg!(internal_impl::make_flex_box_for_dialog(
            self_id,
            DialogEngineConfigOptions {
                mode: DialogEngineMode::ModalAutocomplete,
                ..Default::default()
            },
            window_size,
            Some(SurfaceBounds::from(&surface)),
        ));

        assert_eq2!(result_flex_box.is_ok(), true);

        let flex_box = result_flex_box.unwrap();
        assert_eq2!(flex_box.id, self_id);
        assert_eq2!(flex_box.style_adjusted_bounds_size, width(58) + height(10));
        assert_eq2!(flex_box.style_adjusted_origin_pos, col(5) + row(2));
    }
}

#[cfg(test)]
mod test_dialog_engine_api_apply_event {
    use super::*;
    use crate::{assert_eq2, key_press, test_dialog::mock_real_objects_for_dialog};

    #[test]
    fn apply_event_esc() {
        let self_id: FlexBoxId = FlexBoxId::from(0);
        let dialog_engine = &mut mock_real_objects_for_dialog::make_dialog_engine();
        let state = &mut mock_real_objects_for_dialog::create_state();
        let input_event = InputEvent::Keyboard(key_press!(@special SpecialKey::Esc));
        let response = dbg!(DialogEngineApi::apply_event::<_, ()>(
            state,
            self_id,
            dialog_engine,
            input_event,
        )
        .unwrap());
        assert!(matches!(
            response,
            DialogEngineApplyResponse::DialogChoice(DialogChoice::No)
        ));
    }

    #[test]
    fn apply_event_enter() {
        let self_id: FlexBoxId = FlexBoxId::from(0);
        let dialog_engine = &mut mock_real_objects_for_dialog::make_dialog_engine();
        let state = &mut mock_real_objects_for_dialog::create_state();
        let input_event = InputEvent::Keyboard(key_press!(@special SpecialKey::Enter));
        let response = dbg!(DialogEngineApi::apply_event::<
            mock_real_objects_for_dialog::State,
            (),
        >(state, self_id, dialog_engine, input_event)
        .unwrap());
        if let DialogEngineApplyResponse::DialogChoice(DialogChoice::Yes(value)) =
            &response
        {
            assert_eq2!(value, "");
        }
        assert!(matches!(
            response,
            DialogEngineApplyResponse::DialogChoice(DialogChoice::Yes(_))
        ));
    }

    #[test]
    fn apply_event_other_key() {
        let self_id: FlexBoxId = FlexBoxId::from(0);
        let dialog_engine = &mut mock_real_objects_for_dialog::make_dialog_engine();
        let state = &mut mock_real_objects_for_dialog::create_state();
        let input_event = InputEvent::Keyboard(key_press!(@char 'a'));
        let response = dbg!(DialogEngineApi::apply_event::<
            mock_real_objects_for_dialog::State,
            (),
        >(state, self_id, dialog_engine, input_event)
        .unwrap());
        if let DialogEngineApplyResponse::UpdateEditorBuffer = &response {
            let editor_content = state
                .get_mut_dialog_buffer(self_id)
                .unwrap()
                .editor_buffer
                .get_as_string_with_comma_instead_of_newlines();
            assert_eq2!(editor_content, "a");
        }
    }
}
