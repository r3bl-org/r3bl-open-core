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

use r3bl_redux::*;
use r3bl_rs_utils_core::*;

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
  pub has_focus: HasFocus,
  // FUTURE: 🐵 add user_data in ComponentRegistry
  pub user_data: HashMap<String, HashMap<String, String>>,
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

  pub fn remove(&mut self, id: &str) -> Option<SharedComponent<S, A>> { self.components.remove(id) }
}

pub mod user_data_ops {
  use super::*;

  pub fn get<S, A>(
    component_registry: &ComponentRegistry<S, A>,
    id: &str,
    key: &str,
  ) -> Option<String>
  where
    S: Default + Display + Clone + PartialEq + Debug + Sync + Send,
    A: Default + Display + Clone + Sync + Send,
  {
    component_registry
      .user_data
      .get(id)
      .and_then(|map| map.get(key))
      .map(|s| s.to_string())
  }

  pub fn put<S, A>(
    component_registry: &mut ComponentRegistry<S, A>,
    id: &str,
    key: &str,
    value: &str,
  ) where
    S: Default + Display + Clone + PartialEq + Debug + Sync + Send,
    A: Default + Display + Clone + Sync + Send,
  {
    let map = component_registry
      .user_data
      .entry(id.to_string())
      .or_default();
    map.insert(key.to_string(), value.to_string());
  }
}

impl<S, A> ComponentRegistry<S, A>
where
  S: Default + Display + Clone + PartialEq + Debug + Sync + Send,
  A: Default + Display + Clone + Sync + Send,
{
  pub fn get_focused_component_ref(
    this: &mut ComponentRegistry<S, A>,
  ) -> Option<SharedComponent<S, A>> {
    if let Some(ref id) = this.has_focus.id {
      if let Some(component) = this.get(id) {
        return Some(component.clone());
      }
    }
    None
  }

  pub fn get_component_ref_by_id(
    this: &mut ComponentRegistry<S, A>,
    id: &str,
  ) -> Option<SharedComponent<S, A>> {
    if let Some(component) = this.get(id) {
      return Some(component.clone());
    }
    None
  }

  pub async fn route_event_to_focused_component(
    this: &mut ComponentRegistry<S, A>,
    input_event: &InputEvent,
    state: &S,
    shared_store: &SharedStore<S, A>,
    shared_tw_data: &SharedTWData,
  ) -> CommonResult<EventPropagation> {
    throws_with_return!({
      // If component has focus, then route input_event to it. Return its propagation enum.
      if let Some(shared_component_has_focus) = ComponentRegistry::get_focused_component_ref(this) {
        call_handle_event!(
          component_registry: this,
          shared_component: shared_component_has_focus,
          input_event: input_event,
          state: state,
          shared_store: shared_store,
          shared_tw_data: shared_tw_data
        )
      };

      // input_event not handled, propagate it.
      EventPropagation::Propagate
    });
  }
}

/// Macro to help with the boilerplate of calling
/// [route_event_to_focused_component()](ComponentRegistry::route_event_to_focused_component)
#[macro_export]
macro_rules! route_event_to_focused_component {
  (
    registry:       $arg_component_registry : expr,
    has_focus:      $arg_has_focus          : expr,
    input_event:    $arg_input_event        : expr,
    state:          $arg_state              : expr,
    shared_store:   $arg_shared_store       : expr,
    shared_tw_data: $arg_shared_tw_data     : expr
  ) => {
    ComponentRegistry::route_event_to_focused_component(
      &mut $arg_component_registry,
      $arg_input_event,
      $arg_state,
      $arg_shared_store,
      $arg_shared_tw_data,
    )
    .await
  };
}

/// Macro to help with the boilerplate of calling [Component::handle_event] on a [SharedComponent].
/// This is used by [route_event_to_focused_component!].
#[macro_export]
macro_rules! call_handle_event {
  (
    component_registry: $component_registry : expr,
    shared_component:   $shared_component: expr,
    input_event:        $input_event: expr,
    state:              $state: expr,
    shared_store:       $shared_store: expr,
    shared_tw_data:     $shared_tw_data: expr
  ) => {{
    let result_event_propagation = $shared_component
      .write()
      .await
      .handle_event(
        ComponentScopeArgs {
          shared_tw_data: $shared_tw_data,
          shared_store: $shared_store,
          state: $state,
          component_registry: $component_registry,
        },
        $input_event,
      )
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
        .field("has_focus", &self.has_focus)
        .finish()
    }
  }
}
