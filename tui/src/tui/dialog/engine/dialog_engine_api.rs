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

// ┏━━━━━━━━━━━━━━━━━━┓
// ┃ DialogEngine API ┃
// ┛                  ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// Things you can do w/ a dialog engine.
pub struct DialogEngineApi;

#[derive(Debug)]
pub enum DialogEngineApplyResponse {
  UpdateEditorBuffer(EditorBuffer),
  DialogChoice(DialogChoice),
  Noop,
}

impl DialogEngineApi {
  /// Return the [FlexBox] for the dialog to be rendered in.
  /// - In non-modal contexts, this is determined by the layout engine.
  /// - In the modal case, things are different because the dialog escapes the boundaries of the
  ///   layout engine and really just paints itself on top of everything. It can reach any corner
  ///   of the screen.
  /// - However, it is still constrained by the bounds of the [Surface] itself and does not take
  ///   into account the full window size (in case these are different).
  pub fn make_flex_box_for_dialog(
    dialog_id: FlexBoxId,
    surface: &Surface,
    window_size: &Size,
  ) -> CommonResult<FlexBox> {
    internal_impl::make_flex_box_for_dialog(dialog_id, surface, window_size)
  }

  /// See [make_flex_box](DialogEngineApi::make_flex_box_for_dialog) which actually generates the
  /// [FlexBox] that is passed to this function.
  pub async fn render_engine<S, A>(
    args: DialogEngineArgs<'_, S, A>,
    current_box: &FlexBox,
  ) -> CommonResult<RenderPipeline>
  where
    S: Default + Clone + PartialEq + Debug + Sync + Send,
    A: Default + Clone + Sync + Send,
  {
    let (origin_pos, bounds_size) = {
      let dialog_flex_box: EditorEngineFlexBox = current_box.into();
      (
        dialog_flex_box.style_adjusted_origin_pos,
        dialog_flex_box.style_adjusted_bounds_size,
      )
    };

    let pipeline = {
      let mut it = render_pipeline!();

      it.push(
        ZOrder::Glass,
        internal_impl::render_border(&origin_pos, &bounds_size, args.dialog_engine),
      );

      it.push(
        ZOrder::Glass,
        internal_impl::render_title(&origin_pos, &bounds_size, &args.dialog_buffer.title, args.dialog_engine),
      );

      it += internal_impl::render_editor(&origin_pos, &bounds_size, args).await?;

      it
    };

    Ok(pipeline)
  }

  /// Event based interface for the editor. This executes the [InputEvent].
  /// 1. Returns [Some(DialogResponse)] if <kbd>Enter</kbd> or <kbd>Esc</kbd> was pressed.
  /// 2. Otherwise returns [None].
  pub async fn apply_event<S, A>(
    args: DialogEngineArgs<'_, S, A>,
    input_event: &InputEvent,
  ) -> CommonResult<DialogEngineApplyResponse>
  where
    S: Default + Clone + PartialEq + Debug + Sync + Send,
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

    if let Some(choice) = internal_impl::try_handle_dialog_choice(input_event, dialog_buffer) {
      return Ok(DialogEngineApplyResponse::DialogChoice(choice));
    }

    let editor_engine_args = EditorEngineArgs {
      component_registry,
      shared_global_data,
      self_id,
      editor_buffer: &dialog_buffer.editor_buffer,
      editor_engine: &mut dialog_engine.editor_engine,
      shared_store,
      state,
    };

    if let EditorEngineApplyResponse::Applied(new_editor_buffer) =
      EditorEngineRenderApi::apply_event(editor_engine_args, input_event).await?
    {
      return Ok(DialogEngineApplyResponse::UpdateEditorBuffer(new_editor_buffer));
    }

    Ok(DialogEngineApplyResponse::Noop)
  }
}

mod internal_impl {
  use super::*;

  /// Return the [FlexBox] for the dialog to be rendered in.
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
    surface: &Surface,
    window_size: &Size,
  ) -> CommonResult<FlexBox> {
    let surface_size = surface.box_size;
    let surface_origin_pos = surface.origin_pos;

    // Check to ensure that the dialog box has enough space to be displayed.
    if window_size.col_count < ch!(MinSize::Col.int_value()) || window_size.row_count < ch!(MinSize::Row.int_value()) {
      return CommonError::new(
        CommonErrorType::DisplaySizeTooSmall,
        &format!(
          "Window size is too small. Min size is {} cols x {} rows",
          MinSize::Col.int_value(),
          MinSize::Row.int_value()
        ),
      );
    }

    let dialog_size = {
      // Calc dialog bounds size based on window size.
      let size = size! { col_count: surface_size.col_count * 90/100, row_count: 4 };
      assert!(size.row_count < ch!(MinSize::Row.int_value()));
      size
    };

    let mut origin_pos = {
      // Calc origin pos based on window size & dialog size.
      let origin_col = surface_size.col_count / 2 - dialog_size.col_count / 2;
      let origin_row = surface_size.row_count / 2 - dialog_size.row_count / 2;
      position!(col_index: origin_col, row_index: origin_row)
    };
    origin_pos += surface_origin_pos;

    throws_with_return!({
      EditorEngineFlexBox {
        id: dialog_id,
        style_adjusted_origin_pos: origin_pos,
        style_adjusted_bounds_size: dialog_size,
        maybe_computed_style: None,
      }
      .into()
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
    let maybe_style = args.dialog_engine.maybe_style_editor.clone();

    let flex_box: FlexBox = EditorEngineFlexBox {
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

    let mut pipeline = EditorEngineRenderApi::render_engine(editor_engine_args, &flex_box).await?;
    pipeline.hoist(ZOrder::Normal, ZOrder::Glass);

    Ok(pipeline)
  }

  pub fn render_title(
    origin_pos: &Position,
    bounds_size: &Size,
    title: &str,
    dialog_engine: &mut DialogEngine,
  ) -> RenderOps {
    let mut ops = render_ops!();

    let row_pos = position!(col_index: origin_pos.col_index + 1, row_index: origin_pos.row_index + 1);
    let unicode_string = UnicodeString::from(title);
    let mut text_content = Cow::Borrowed(unicode_string.truncate_to_fit_size(size! {
      col_count: bounds_size.col_count - 2, row_count: bounds_size.row_count
    }));

    // Apply lolcat override (if enabled) to the fg_color of text_content.
    apply_lolcat_from_style(
      &dialog_engine.maybe_style_title,
      &mut dialog_engine.lolcat,
      &mut text_content,
    );

    ops.push(RenderOp::ResetColor);
    ops.push(RenderOp::MoveCursorPositionAbs(row_pos));
    ops.push(RenderOp::ApplyColors(dialog_engine.maybe_style_title.clone()));
    ops.push(RenderOp::PrintTextWithAttributes(
      text_content.into(),
      dialog_engine.maybe_style_title.clone(),
    ));

    ops
  }

  pub fn render_border(origin_pos: &Position, bounds_size: &Size, dialog_engine: &mut DialogEngine) -> RenderOps {
    let mut ops = render_ops!();

    let inner_spaces = SPACER.repeat(ch!(@to_usize bounds_size.col_count - 2));

    let maybe_style = dialog_engine.maybe_style_border.clone();

    for row_idx in 0..*bounds_size.row_count {
      let row_pos = position!(col_index: origin_pos.col_index, row_index: origin_pos.row_index + row_idx);

      let is_first_line = row_idx == 0;
      let is_last_line = row_idx == (*bounds_size.row_count - 1);

      ops.push(RenderOp::ResetColor);
      ops.push(RenderOp::MoveCursorPositionAbs(row_pos));
      ops.push(RenderOp::ApplyColors(maybe_style.clone()));

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
          apply_lolcat_from_style(&maybe_style, &mut dialog_engine.lolcat, &mut text_content);

          ops.push(RenderOp::PrintTextWithAttributes(
            text_content.into(),
            maybe_style.clone(),
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
          apply_lolcat_from_style(&maybe_style, &mut dialog_engine.lolcat, &mut text_content);
          ops.push(RenderOp::PrintTextWithAttributes(
            text_content.into(),
            maybe_style.clone(),
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
          apply_lolcat_from_style(&maybe_style, &mut dialog_engine.lolcat, &mut text_content);
          ops.push(RenderOp::PrintTextWithAttributes(
            text_content.into(),
            maybe_style.clone(),
          ));
        }
        _ => {}
      };
    }

    ops
  }

  pub fn try_handle_dialog_choice(input_event: &InputEvent, dialog_buffer: &DialogBuffer) -> Option<DialogChoice> {
    if let Some(dialog_event) = DialogEvent::try_from(input_event, None) {
      match dialog_event {
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
    }
    None
  }
}
