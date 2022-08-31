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

use std::{collections::HashMap,
          fmt::{Debug, Display}};

use crate::*;

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ ComponentRegistry │
// ╯                   ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
/// This map is used to cache [Component]s that have been created and are meant to be reused
/// between multiple renders. It is entirely up to the [App] how to use this map. The
/// methods provided allow components to be added to the map.
#[derive(Default)]
pub struct ComponentRegistry<S, A>
where
  S: Default + Display + Clone + PartialEq + Debug + Sync + Send,
  A: Default + Display + Clone + Sync + Send,
{
  pub components: ComponentRegistryMap<S, A>,
}

pub type ComponentRegistryMap<S, A> = HashMap<String, SharedComponent<S, A>>;

impl<S, A> ComponentRegistry<S, A>
where
  S: Default + Display + Clone + PartialEq + Debug + Sync + Send,
  A: Default + Display + Clone + Sync + Send,
{
  pub fn put(&mut self, name: &str, component: SharedComponent<S, A>) {
    self.components.insert(name.to_string(), component);
  }

  pub fn id_does_not_exist(&self, name: &str) -> bool { !self.components.contains_key(name) }

  pub fn get(&self, name: &str) -> Option<&SharedComponent<S, A>> { self.components.get(name) }

  pub fn get_has_focus(&self, has_focus: &HasFocus) -> Option<&SharedComponent<S, A>> {
    match has_focus.id {
      Some(ref id_has_focus) => self.get(id_has_focus),
      None => None,
    }
  }

  pub fn remove(&mut self, id: &str) -> Option<SharedComponent<S, A>> { self.components.remove(id) }

  pub async fn route_event_to_focused_component(
    &mut self, has_focus: &HasFocus, input_event: &InputEvent, state: &S,
    shared_store: &SharedStore<S, A>,
  ) -> CommonResult<EventPropagation> {
    throws_with_return!({
      // If component has focus, then route input_event to it. Return its propagation enum.
      if let Some(shared_component_has_focus) = self.get_has_focus(has_focus) {
        call_handle_event!(
          shared_component: shared_component_has_focus,
          input_event: input_event,
          state: state,
          shared_store: shared_store
        )
      };

      // input_event not handled, propagate it.
      EventPropagation::Propagate
    });
  }
}

/// Macro to help with the boilerplate of calling
/// `ComponentRegistry::route_event_to_focused_component`.
#[macro_export]
macro_rules! route_event_to_focused_component {
  (
    registry:     $arg_component_registry : expr,
    has_focus:    $arg_has_focus          : expr,
    input_event:  $arg_input_event        : expr,
    state:        $arg_state              : expr,
    shared_store: $arg_shared_store       : expr
  ) => {
    $arg_component_registry
      .route_event_to_focused_component(
        &$arg_has_focus,
        $arg_input_event,
        $arg_state,
        $arg_shared_store,
      )
      .await
  };
}

/// Macro to help with the boilerplate of calling [Component::handle_event] on a [SharedComponent].
/// This is used by [route_event_to_focused_component!].
#[macro_export]
macro_rules! call_handle_event {
  (
    shared_component: $shared_component: expr,
    input_event:      $input_event: expr,
    state:            $state: expr,
    shared_store:     $shared_store: expr
  ) => {{
    let result_event_propagation = $shared_component
      .write()
      .await
      .handle_event($input_event, $state, $shared_store)
      .await?;
    return Ok(result_event_propagation);
  }};
}

mod debug_helpers {
  use super::*;

  impl<S, A> Debug for ComponentRegistry<S, A>
  where
    S: Default + Display + Clone + PartialEq + Debug + Sync + Send,
    A: Default + Display + Clone + Sync + Send,
  {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      f.debug_struct("ComponentRegistry")
        .field("components", &self.components.keys().enumerate())
        .finish()
    }
  }
}
