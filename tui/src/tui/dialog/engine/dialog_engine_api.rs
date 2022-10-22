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
  Noop,
  DialogChoice(DialogChoice),
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
    if let Some(choice) = try_handle_dialog_choice(input_event, args) {
      return Ok(DialogEngineApplyResponse::DialogChoice(choice));
    }

    // TODO: handle passing thru input_event to the editor.

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
    // TODO: impl render
    Ok(RenderPipeline::default())
  }
}

fn try_handle_dialog_choice<S, A>(input_event: &InputEvent, args: DialogEngineArgs<S, A>) -> Option<DialogChoice>
where
  S: Default + Clone + PartialEq + Debug + Sync + Send,
  A: Default + Clone + Sync + Send,
{
  if let Some(dialog_event) = DialogEvent::try_from(input_event, None) {
    match dialog_event {
      // Handle Enter.
      DialogEvent::EnterPressed => {
        let DialogEngineArgs { dialog_buffer, .. } = args;
        // Get the EditorBuffer content.
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
