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
  pub fn flex_box_from(id: FlexBoxIdType, window_size: &Size) -> CommonResult<FlexBox> {
    internal_impl::flex_box_from(id, window_size)
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
      shared_tw_data,
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
      shared_tw_data,
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

  pub async fn render_engine<S, A>(
    args: DialogEngineArgs<'_, S, A>,
    current_box: &FlexBox,
  ) -> CommonResult<RenderPipeline>
  where
    S: Default + Clone + PartialEq + Debug + Sync + Send,
    A: Default + Clone + Sync + Send,
  {
    let (origin_pos, bounds_size, maybe_style) = {
      let dialog_flex_box: EditorEngineFlexBox = current_box.into();
      (
        dialog_flex_box.style_adjusted_origin_pos,
        dialog_flex_box.style_adjusted_bounds_size,
        dialog_flex_box.maybe_computed_style,
      )
    };

    let mut pipeline = render_pipeline!(@new_empty);

    pipeline += internal_impl::clear_dialog_box(&origin_pos, &bounds_size, &maybe_style);
    pipeline += internal_impl::add_title(&origin_pos, &bounds_size, &maybe_style, &args.dialog_buffer.title);
    pipeline += internal_impl::render_editor(&origin_pos, &bounds_size, &maybe_style, args).await?;

    Ok(pipeline)
  }
}

mod internal_impl {
  use super::*;

  /// Return the [FlexBox] for the dialog to be rendered in.
  ///
  /// ```ignore
  /// FlexBox {
  ///   id: 0,
  ///   style_adjusted_origin_pos: [col:0, row:0],
  ///   style_adjusted_bounds_size: [width:0, height:0],
  ///   maybe_computed_style: None,
  ///   ..
  /// }
  /// window_size: [width:0, height:0]
  /// ```
  pub fn flex_box_from(id: FlexBoxIdType, window_size: &Size) -> CommonResult<FlexBox> {
    if window_size.cols < ch!(MinSize::Col.int_value()) || window_size.rows < ch!(MinSize::Row.int_value()) {
      return CommonError::new(
        CommonErrorType::General,
        &format!(
          "Window size is too small. Min size is {} cols x {} rows",
          MinSize::Col.int_value(),
          MinSize::Row.int_value()
        ),
      );
    }

    let dialog_size = {
      // Calc dialog bounds size based on window size.
      let size = size! { cols: window_size.cols * 90/100, rows: 4 };
      assert!(size.rows < ch!(MinSize::Row.int_value()));
      size
    };

    let origin_pos = {
      // Calc origin pos based on window size & dialog size.
      let origin_col = window_size.cols / 2 - dialog_size.cols / 2;
      let origin_row = window_size.rows / 2 - dialog_size.rows / 2;
      position!(col: origin_col, row: origin_row)
    };

    throws_with_return!({
      EditorEngineFlexBox {
        id,
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
    maybe_style: &Option<Style>,
    args: DialogEngineArgs<'_, S, A>,
  ) -> CommonResult<RenderPipeline>
  where
    S: Default + Clone + PartialEq + Debug + Sync + Send,
    A: Default + Clone + Sync + Send,
  {
    let flex_box: FlexBox = EditorEngineFlexBox {
      id: args.self_id,
      style_adjusted_origin_pos: position! {col: origin_pos.col + 1, row: origin_pos.row + 2},
      style_adjusted_bounds_size: size! {cols: bounds_size.cols - 2, rows: 1},
      maybe_computed_style: maybe_style.clone(),
    }
    .into();

    let editor_engine_args = EditorEngineArgs {
      component_registry: args.component_registry,
      shared_tw_data: args.shared_tw_data,
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

  pub fn add_title(
    origin_pos: &Position,
    bounds_size: &Size,
    maybe_style: &Option<Style>,
    title: &str,
  ) -> RenderPipeline {
    let mut pipeline = render_pipeline!(@new_empty);

    let row_pos = position!(col: origin_pos.col + 1, row: origin_pos.row + 1);
    let unicode_string = UnicodeString::from(title);
    let text_content = unicode_string.truncate_to_fit_size(size! {
      cols: bounds_size.cols - 2, rows: bounds_size.rows
    });

    render_pipeline!(@push_into pipeline at ZOrder::Glass =>
      RenderOp::ResetColor,
      RenderOp::MoveCursorPositionAbs(row_pos),
      RenderOp::PrintTextWithAttributes(text_content.into(), maybe_style.clone())
    );

    pipeline
  }

  pub fn clear_dialog_box(origin_pos: &Position, bounds_size: &Size, maybe_style: &Option<Style>) -> RenderPipeline {
    let mut pipeline = render_pipeline!(@new_empty);

    let inner_spaces = " ".repeat(ch!(@to_usize bounds_size.cols - 2));

    for row_idx in 0..*bounds_size.rows {
      let row_pos = position!(col: origin_pos.col, row: origin_pos.row + row_idx);

      let is_first_line = row_idx == 0;
      let is_last_line = row_idx == (*bounds_size.rows - 1);

      render_pipeline!(@push_into pipeline at ZOrder::Glass =>
        RenderOp::ResetColor,
        RenderOp::MoveCursorPositionAbs(row_pos)
      );

      match (is_first_line, is_last_line) {
        // First line.
        (true, false) => {
          let text_content = format!(
            "{}{}{}",
            BorderGlyphCharacter::TopLeft.as_ref(),
            BorderGlyphCharacter::Horizontal
              .as_ref()
              .repeat(ch!(@to_usize bounds_size.cols - 2)),
            BorderGlyphCharacter::TopRight.as_ref()
          );
          render_pipeline!(@push_into pipeline at ZOrder::Glass =>
            RenderOp::PrintTextWithAttributes(text_content, maybe_style.clone())
          );
        }
        // Last line.
        (false, true) => {
          let text_content = format!(
            "{}{}{}",
            BorderGlyphCharacter::BottomLeft.as_ref(),
            BorderGlyphCharacter::Horizontal
              .as_ref()
              .repeat(ch!(@to_usize bounds_size.cols - 2)),
            BorderGlyphCharacter::BottomRight.as_ref(),
          );
          render_pipeline!(@push_into pipeline at ZOrder::Glass =>
            RenderOp::PrintTextWithAttributes(text_content, maybe_style.clone())
          );
        }
        // Middle line.
        (false, false) => {
          let text_content = format!(
            "{}{}{}",
            BorderGlyphCharacter::Vertical.as_ref(),
            inner_spaces,
            BorderGlyphCharacter::Vertical.as_ref()
          );
          render_pipeline!(@push_into pipeline at ZOrder::Glass =>
            RenderOp::PrintTextWithAttributes(text_content, maybe_style.clone())
          );
        }
        _ => {}
      };
    }

    pipeline
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
