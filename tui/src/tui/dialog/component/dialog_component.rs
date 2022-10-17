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

use std::{fmt::{Debug, Display},
          sync::Arc};

use async_trait::async_trait;
use r3bl_redux::*;
use r3bl_rs_utils_core::*;
use tokio::sync::RwLock;

use crate::*;

// ┏━━━━━━━━━━━━━━━━━━━━━━━━━┓
// ┃ Dialog Component struct ┃
// ┛                         ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// This is a shim which allows the reusable [DialogEngine] to be used in the context of [Component]
/// and [Store]. The main methods here simply pass thru all their arguments to the
/// [DialogEngineApi].
#[derive(Clone, Default)]
pub struct DialogComponent<S, A>
where
  S: Default + Display + Clone + PartialEq + Debug + Sync + Send,
  A: Default + Display + Clone + Sync + Send,
{
  pub id: String,
  pub prev_focus_id: String,
  pub engine: DialogEngine,
  pub on_dialog_press_handler: Option<OnDialogPressFn<S, A>>,
}

pub mod impl_component {
  use super::*;

  #[async_trait]
  impl<S, A> Component<S, A> for DialogComponent<S, A>
  where
    S: HasDialogBuffer + Default + Display + Clone + PartialEq + Debug + Sync + Send,
    A: Default + Display + Clone + Sync + Send,
  {
    fn get_id(&self) -> &str { &self.id }

    // ┏━━━━━━━━━━━━━━┓
    // ┃ handle_event ┃
    // ┛              ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    /// This shim simply calls [apply_event](DialogEngineApi::apply_event) w/ all the necessary
    /// arguments:
    /// - Global scope: [SharedStore], [SharedTWData].
    /// - App scope: `S`, [ComponentRegistry<S, A>].
    /// - User input (from [main_event_loop]): [InputEvent].
    ///
    /// Usually a component must have focus in order for the [App] to
    /// [route_event_to_focused_component!] in the first place.
    async fn handle_event(
      &mut self,
      args: ComponentScopeArgs<'_, S, A>,
      input_event: &InputEvent,
    ) -> CommonResult<EventPropagation> {
      // TODO: connect to DialogEngineApi::apply_event
      Ok(EventPropagation::Propagate)
    }

    // ┏━━━━━━━━┓
    // ┃ render ┃
    // ┛        ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    /// This shim simply calls [render](DialogEngineApi::render_engine) w/ all the necessary
    /// arguments:
    /// - Global scope: [SharedStore], [SharedTWData].
    /// - App scope: `S`, [ComponentRegistry<S, A>].
    /// - User input (from [main_event_loop]): [InputEvent].
    async fn render(
      &mut self,
      args: ComponentScopeArgs<'_, S, A>,
      current_box: &FlexBox,
    ) -> CommonResult<RenderPipeline> {
      // TODO: connect to DialogEngineApi::render_engine
      Ok(RenderPipeline::default())
    }
  }
}

pub mod handle_focus_change {
  use super::*;

  // TODO: save prev_focus_id (called by App)
  // TODO: restore prev_focus_id (called by DialogComponent)
}

pub mod constructor {
  use super::*;

  impl<S, A> DialogComponent<S, A>
  where
    S: Default + Display + Clone + PartialEq + Debug + Sync + Send,
    A: Default + Display + Clone + Sync + Send,
  {
    /// The on_dialog_press_handler is a lambda that is called if the user presses enter or escape.
    /// Typically this results in a Redux action being created and then dispatched to the given
    /// store.
    pub fn new(id: &str, on_dialog_press_handler: OnDialogPressFn<S, A>) -> Self {
      Self {
        engine: DialogEngine::default(),
        prev_focus_id: String::default(),
        id: id.to_string(),
        on_dialog_press_handler: Some(on_dialog_press_handler),
      }
    }

    pub fn new_shared(id: &str, on_dialog_press_handler: OnDialogPressFn<S, A>) -> Arc<RwLock<Self>> {
      Arc::new(RwLock::new(DialogComponent::new(id, on_dialog_press_handler)))
    }
  }
}

pub mod misc {
  use super::*;

  /// This marker trait is meant to be implemented by whatever state struct is being used to store the
  /// dialog buffer for this re-usable editor component. It is used in the `where` clause of the
  /// [DialogComponent] to ensure that the generic type `S` implements this trait, guaranteeing that
  /// it holds a single [DialogBuffer].
  pub trait HasDialogBuffer {
    fn get_dialog_buffer(&self) -> Option<&DialogBuffer>;
  }

  #[derive(Debug)]
  pub enum DialogResponse {
    Yes(String),
    No,
  }

  pub type OnDialogPressFn<S, A> = fn(
    String, /* my_id */
    DialogResponse,
    String, /* prev_focus_id */
    &SharedStore<S, A>,
    &ComponentRegistry<S, A>,
  );
}
pub use misc::*; // Re-export misc module for convenience.
