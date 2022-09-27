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

use async_trait::async_trait;
use r3bl_rs_utils::*;

use crate::{ex_editor::*, *};

/// This is a shim which allows the reusable [EditorEngine] to be used in the context of [Component]
/// and [Store]. The main methods here simply pass thru all their arguments to the [EditorEngine].
#[derive(Debug, Clone, Default)]
pub struct EditorComponent {
  pub engine: EditorRenderEngine,
  pub id: String,
}

impl EditorComponent {
  pub fn new(id: &str) -> Self {
    Self {
      engine: EditorRenderEngine::default(),
      id: id.to_string(),
    }
  }
}

#[async_trait]
impl Component<State, Action> for EditorComponent {
  // ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
  // │ handle_event │
  // ╯              ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
  /// This shim simply calls [EditorEngine::apply](EditorEngine::apply) w/ all the necessary
  /// arguments:
  /// - Global scope: [SharedStore], [SharedTwData].
  /// - App scope: [State], [ComponentRegistry<State, Action>].
  /// - User input (from [main_event_loop]): [InputEvent].
  async fn handle_event(
    &mut self,
    args: ComponentScopeArgs<'_, State, Action>,
    input_event: &InputEvent,
  ) -> CommonResult<EventPropagation> {
    throws_with_return!({
      let ComponentScopeArgs {
        shared_tw_data,
        shared_store,
        state,
        component_registry,
      } = args;

      // Try to apply the `input_event` to `editor_engine` to decide whether to fire action.
      match self
        .engine
        .apply(
          EditorEngineArgs {
            state,
            buffer: &state.buffer,
            component_registry,
            shared_tw_data,
            shared_store,
            self_id: &self.id,
          },
          input_event,
        )
        .await?
      {
        Some(buffer) => {
          let action = Action::UpdateEditorBuffer(buffer);
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

  // ╭┄┄┄┄┄┄┄┄╮
  // │ render │
  // ╯        ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
  /// This shim simply calls [EditorEngine::apply](EditorEngine::render) w/ all the necessary
  /// arguments:
  /// - Global scope: [SharedStore], [SharedTwData].
  /// - App scope: [State], [ComponentRegistry<State, Action>].
  /// - User input (from [main_event_loop]): [InputEvent].
  async fn render(
    &mut self,
    args: ComponentScopeArgs<'_, State, Action>,
    current_box: &FlexBox,
  ) -> CommonResult<RenderPipeline> {
    let ComponentScopeArgs {
      state,
      shared_store,
      shared_tw_data,
      component_registry,
    } = args;

    self
      .engine
      .render(
        EditorEngineArgs {
          state,
          buffer: &state.buffer,
          component_registry,
          shared_tw_data,
          shared_store,
          self_id: &self.id,
        },
        current_box,
      )
      .await
  }
}

#[macro_export]
macro_rules! dispatch_editor_action {
  (
    @update_editor_buffer =>
    $arg_shared_store: ident,
    $arg_action:       expr
  ) => {{
    let mut _event_consumed = false;
    let action_clone_for_debug = $arg_action.clone();
    spawn_and_consume_event!(_event_consumed, $arg_shared_store, $arg_action);
    dispatch_editor_action!(@debug => action_clone_for_debug);
    _event_consumed
  }};
  (
    @debug => $arg_action: expr
  ) => {
    use $crate::DEBUG;
    call_if_true!(
      DEBUG,
      log_no_err!(
        INFO,
        "⛵ EditorComponent::handle_event -> dispatch_spawn: {}",
        $arg_action
      )
    );
  };
}
