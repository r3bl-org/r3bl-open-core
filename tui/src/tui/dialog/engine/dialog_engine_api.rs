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

    if let Some(choice) = try_handle_dialog_choice(input_event, dialog_buffer) {
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

  /// Return the [FlexBox] for the dialog to be rendered in.
  ///
  /// ```ignore
  /// FlexBox {
  ///   id: 0,
  ///   style_adjusted_origin_pos: [col:0, row:0],
  ///   style_adjusted_bounds_size: [width:0, height:0],
  ///   maybe_computed_style: None,
  ///   /* dir: Horizontal, */
  ///   /* origin_pos: [col:0, row:0], */
  ///   /* bounds_size: [width:0, height:0], */
  ///   /* requested_size_percent: [width:0%, height:0%], */
  ///   /* insertion_pos_for_next_box: None, */
  /// }
  /// window_size: [width:159, height:13]
  /// ```
  pub fn flex_box_from(id: FlexBoxIdType, window_size: &Size) -> CommonResult<FlexBox> {
    // TODO: impl this
    throws_with_return!({
      EditorEngineFlexBox {
        id,
        // TODO: style_adjusted_origin_pos: Position,
        // TODO: style_adjusted_bounds_size: Size,
        // TODO: maybe_computed_style: Option<Style>,
        ..Default::default()
      }
      .into()
    })
  }

  pub async fn render_engine<S, A>(
    args: DialogEngineArgs<'_, S, A>,
    current_box: &FlexBox,
  ) -> CommonResult<RenderPipeline>
  where
    S: Default + Clone + PartialEq + Debug + Sync + Send,
    A: Default + Clone + Sync + Send,
  {
    let DialogEngineArgs { window_size, .. } = args;

    // TODO: remove debug
    log_no_err!(DEBUG, "📦 current_box: {:?}", current_box);
    log_no_err!(DEBUG, "📦 window_size: {:?}", window_size);

    // TODO: impl render
    /*
    EditorEngineRenderApi::render_engine(render_args, current_box).await;

    render_args:
    - self_id: dialog_engine_args.self_id (modal dialog's id)

    current_box:
    - .style_adjusted_bounds_size
    - .style_adjusted_origin_pos
    - .get_computed_style()
    */

    Ok(RenderPipeline::default())
  }
}

fn try_handle_dialog_choice(input_event: &InputEvent, dialog_buffer: &DialogBuffer) -> Option<DialogChoice> {
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
