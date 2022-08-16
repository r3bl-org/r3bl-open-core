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
/// between multiple renders. It is entirely up to the [TWApp] how to use this map. The
/// methods provided allow components to be added to the map.
#[derive(Default)]
pub struct ComponentRegistry<S, A>
where
  S: Default + Display + Clone + PartialEq + Eq + Debug + Sync + Send,
  A: Default + Display + Clone + Sync + Send,
{
  pub components: ComponentRegistryMap<S, A>,
}

pub type ComponentRegistryMap<S, A> = HashMap<String, SharedComponent<S, A>>;

impl<S, A> Debug for ComponentRegistry<S, A>
where
  S: Default + Display + Clone + PartialEq + Eq + Debug + Sync + Send,
  A: Default + Display + Clone + Sync + Send,
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("ComponentRegistry")
      .field("components", &self.components.keys().enumerate())
      .finish()
  }
}

impl<S, A> ComponentRegistry<S, A>
where
  S: Default + Display + Clone + PartialEq + Eq + Debug + Sync + Send,
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
}

// ╭┄┄┄┄┄┄┄┄┄┄╮
// │ HasFocus │
// ╯          ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
/// There are certain fields that need to be in each state struct to represent global information
/// about keyboard focus.
///
/// 1. An `id` [String] is used to store which [TWBox] id currently holds keyboard focus.
///    This is global.
/// 2. Each `id` may have a [Position] associated with it, which is used to draw the "cursor" (the
///    meaning of which depends on the specific [Component] impl). This cursor is scoped to
///    each `id` so it isn't strictly a single global value (like `id` itself). Here are examples of
///    what a "cursor" might mean for various [Component]s:
///    - for an editor, it will be the insertion point where text is added / removed
///    - for a text viewer, it will be the cursor position which can be moved around
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct HasFocus {
  /// Map of id to its [Position]. Each cursor ([Position]) is scoped to an id. The map is global.
  pub cursor_position_map: CursorPositionMap,
  /// This id has keyboard focus. This is global.
  pub id: Option<String>,
}

pub type CursorPositionMap = HashMap<String, Option<Position>>;

impl HasFocus {
  /// Set the id of the [TWBox] that has keyboard focus.
  pub fn get_id(&self) -> Option<String> { self.id.clone() }

  /// Get the id of the [TWBox] that has keyboard focus.
  pub fn set_id(&mut self, id: &str) { self.id = Some(id.into()) }

  /// Check whether the given id currently has keyboard focus.
  pub fn does_id_have_focus(&self, id: &str) -> bool { self.id == Some(id.into()) }

  /// Check whether the id of the [TWBox] currently has keyboard focus.
  pub fn does_current_box_have_focus(&self, current_box: &TWBox) -> bool {
    self.does_id_have_focus(&current_box.id)
  }

  /// For a given [TWBox] id, set the position of the cursor inside of it.
  pub fn set_cursor_position_for_id(&mut self, id: &str, maybe_position: Option<Position>) {
    let map = &mut self.cursor_position_map;
    map.insert(id.into(), maybe_position);
  }

  /// For a given [TWBox] id, get the position of the cursor inside of it.
  pub fn get_cursor_position_for_id(&self, id: &str) -> Option<Position> {
    let map = &self.cursor_position_map;
    if map.contains_key(id) {
      *map.get(id).unwrap()
    } else {
      None
    }
  }
}
