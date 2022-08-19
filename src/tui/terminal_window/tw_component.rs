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

use std::fmt::{Debug, Display};

use async_trait::async_trait;

use crate::*;

/// See [TWApp].
#[async_trait]
pub trait Component<S, A>
where
  S: Default + Display + Clone + PartialEq + Eq + Debug + Sync + Send,
  A: Default + Display + Clone + Sync + Send,
{
  /// Arguments: Get from `current_box`:
  ///   - box_origin_pos: Position
  ///   - box_bounding_size: Size
  ///   - maybe_box_style: Option<Style>
  ///  
  ///   Get from `state`:
  ///   - Content to render
  ///   - get_focus_id(): String to determine if this component has keyboard focus (might affect the
  ///     way it gets rendered)
  ///  
  ///   Maybe use `shared_store`:
  ///   - Dispatch an action if needed
  ///
  /// Returns:
  ///   - [TWCommandQueue] which must be rendered by the caller
  ///
  /// Clipping, scrolling, overdrawing:
  ///   - Each implementation of this trait is solely responsible of taking care of these behaviors
  async fn render(
    &mut self, has_focus: &HasFocus, current_box: &TWBox, state: &S,
    shared_store: &SharedStore<S, A>,
  ) -> CommonResult<TWCommandQueue>;

  async fn handle_event(
    &mut self, input_event: &TWInputEvent, state: &S, shared_store: &SharedStore<S, A>,
  ) -> CommonResult<EventPropagation>;
}

// FIXME: Add tests for Component, ComponentRegistry, HasFocus, and this macro.
// REFACTOR: add macro for easy render of component from registry
#[macro_export]
macro_rules! render_component {
  (
    in:           $arg_surface        : expr, // Eg: in: surface
    from:         $arg_registry       : expr, // Eg: from: registry
    id:           $arg_component_id   : expr, // Eg: "component1"
    has_focus:    $arg_has_focus      : expr, // Eg: has_focus
    state:        $arg_state          : expr, // Eg: state
    shared_store: $arg_shared_store   : expr  // Eg: shared_store
  ) => {
    if let Some(shared_component) = $arg_registry.get($arg_component_id) {
      let current_box = $arg_surface.current_box()?;
      let queue = shared_component
        .write()
        .await
        .render(&$arg_has_focus, current_box, $arg_state, $arg_shared_store)
        .await?;
      $arg_surface.render_buffer += queue;
    }
  };
}
