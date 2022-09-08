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

// FUTURE: Add tests for Component, ComponentRegistry, HasFocus, and this macro to `test_component.rs`
#[macro_export]
macro_rules! render {
  (
    in:           $arg_surface        : expr, // Eg: in: surface
    component_id: $arg_component_id   : expr, // Eg: "component1"
    from:         $arg_registry       : expr, // Eg: from: registry
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
      $arg_surface.render_pipeline += queue;
    }
  };
}
