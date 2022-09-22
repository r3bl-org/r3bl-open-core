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

use async_trait::async_trait;
use r3bl_rs_utils::*;

use crate::{ex_editor::*, *};

#[derive(Debug, Clone, Default)]
pub struct EditorComponent {
  pub editor_engine: EditorEngine,
  pub id: String,
}

impl EditorComponent {
  pub fn new(id: &str) -> Self {
    Self {
      editor_engine: EditorEngine::default(),
      id: id.to_string(),
    }
  }
}

#[async_trait]
impl Component<State, Action> for EditorComponent {
  async fn handle_event(
    &mut self, input_event: &InputEvent, state: &State, shared_store: &SharedStore<State, Action>,
    shared_tw_data: &SharedTWData,
  ) -> CommonResult<EventPropagation> {
    throws_with_return!({
      // Try to apply the `input_event` to `editor_engine` to decide whether to fire action.
      match self
        .editor_engine
        .apply(&state.editor_buffer, input_event, shared_tw_data, &self.id)
        .await?
      {
        Some(editor_buffer) => {
          let action = Action::UpdateEditorBuffer(editor_buffer);
          dispatch_editor_action!(@update_editor_buffer => shared_store, action);
          EventPropagation::Consumed
        }
        None => {
          // Optional: handle any `input_event` not consumed by `editor_engine`.
          EventPropagation::Propagate
        }
      }
    });
  }

  async fn render(
    &mut self, has_focus: &HasFocus, current_box: &FlexBox, state: &State,
    _: &SharedStore<State, Action>, shared_tw_data: &SharedTWData,
  ) -> CommonResult<RenderPipeline> {
    self
      .editor_engine
      .render(
        &state.editor_buffer,
        has_focus,
        current_box,
        shared_tw_data,
        &self.id,
      )
      .await
  }
}
