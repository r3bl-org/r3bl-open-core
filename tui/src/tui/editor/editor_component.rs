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

use std::{borrow::Cow,
          fmt::{Debug, Display},
          sync::Arc};

use async_trait::async_trait;
use r3bl_redux::*;
use r3bl_rs_utils_core::*;
use tokio::sync::RwLock;

use crate::*;

/// This is a shim which allows the reusable [EditorEngine] to be used in the context of [Component]
/// and [Store]. The main methods here simply pass thru all their arguments to the [EditorEngine].
#[derive(Clone, Default)]
pub struct EditorComponent<S, A>
where
  S: Default + Display + Clone + PartialEq + Debug + Sync + Send,
  A: Default + Display + Clone + Sync + Send,
{
  pub engine: EditorEngine,
  pub id: String,
  pub on_editor_buffer_change_handler: Option<OnEditorBufferChangeFn<S, A>>,
}

pub type OnEditorBufferChangeFn<S, A> = fn(&SharedStore<S, A>, String, EditorBuffer);

impl<S, A> EditorComponent<S, A>
where
  S: Default + Display + Clone + PartialEq + Debug + Sync + Send,
  A: Default + Display + Clone + Sync + Send,
{
  /// The on_change_handler is a lambda that is called if the editor buffer changes. Typically this
  /// results in a Redux action being created and then dispatched to the given store.
  pub fn new(
    id: &str,
    config_options: EditorEngineConfigOptions,
    on_buffer_change: OnEditorBufferChangeFn<S, A>,
  ) -> Self {
    Self {
      engine: EditorEngine::new(config_options),
      id: id.to_string(),
      on_editor_buffer_change_handler: Some(on_buffer_change),
    }
  }

  pub fn new_shared(
    id: &str,
    config_options: EditorEngineConfigOptions,
    on_buffer_change: OnEditorBufferChangeFn<S, A>,
  ) -> Arc<RwLock<Self>> {
    Arc::new(RwLock::new(EditorComponent::new(id, config_options, on_buffer_change)))
  }
}

/// This marker trait is meant to be implemented by whatever state struct is being used to store the
/// editor buffer for this re-usable editor component. It is used in the `where` clause of the
/// [EditorComponent] to ensure that the generic type `S` implements this trait, guaranteeing that
/// it holds an [EditorBuffer].
pub trait HasEditorBuffers {
  fn get_editor_buffer(&self, id: &str) -> Option<&EditorBuffer>;
}

#[async_trait]
impl<S, A> Component<S, A> for EditorComponent<S, A>
where
  S: HasEditorBuffers + Default + Display + Clone + PartialEq + Debug + Sync + Send,
  A: Default + Display + Clone + Sync + Send,
{
  fn get_id(&self) -> &str { &self.id }

  // ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
  // │ handle_event │
  // ╯              ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
  /// This shim simply calls [EditorEngine::apply](EditorEngine::apply) w/ all the necessary
  /// arguments:
  /// - Global scope: [SharedStore], [SharedTWData].
  /// - App scope: `S`, [ComponentRegistry<S, A>].
  /// - User input (from [main_event_loop]): [InputEvent].
  async fn handle_event(
    &mut self,
    args: ComponentScopeArgs<'_, S, A>,
    input_event: &InputEvent,
  ) -> CommonResult<EventPropagation> {
    throws_with_return!({
      let ComponentScopeArgs {
        shared_tw_data,
        shared_store,
        state,
        component_registry,
      } = args;

      let my_buffer: Cow<EditorBuffer> = {
        if let Some(buffer) = state.get_editor_buffer(self.get_id()) {
          Cow::Borrowed(buffer)
        } else {
          Cow::Owned(EditorBuffer::default())
        }
      };

      // Try to apply the `input_event` to `editor_engine` to decide whether to fire action.
      let engine_args = EditorEngineArgs {
        state,
        buffer: &my_buffer,
        component_registry,
        shared_tw_data,
        shared_store,
        self_id: &self.id,
      };

      match self.engine.apply(engine_args, input_event).await? {
        EngineResponse::Applied(buffer) => {
          if let Some(on_change_handler) = self.on_editor_buffer_change_handler {
            let my_id = self.get_id().to_string();
            on_change_handler(shared_store, my_id, buffer);
          }
          EventPropagation::Consumed
        }
        EngineResponse::NotApplied => {
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
  /// - Global scope: [SharedStore], [SharedTWData].
  /// - App scope: `S`, [ComponentRegistry<S, A>].
  /// - User input (from [main_event_loop]): [InputEvent].
  async fn render(
    &mut self,
    args: ComponentScopeArgs<'_, S, A>,
    current_box: &FlexBox,
  ) -> CommonResult<RenderPipeline> {
    let ComponentScopeArgs {
      state,
      shared_store,
      shared_tw_data,
      component_registry,
    } = args;

    let my_buffer: Cow<EditorBuffer> = {
      if let Some(buffer) = state.get_editor_buffer(self.get_id()) {
        Cow::Borrowed(buffer)
      } else {
        Cow::Owned(EditorBuffer::default())
      }
    };

    let render_args = EditorEngineArgs {
      state,
      buffer: &my_buffer,
      component_registry,
      shared_tw_data,
      shared_store,
      self_id: &self.id,
    };

    self.engine.render(render_args, current_box).await
  }
}
