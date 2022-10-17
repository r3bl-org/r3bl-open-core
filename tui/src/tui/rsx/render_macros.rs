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

#[macro_export]
macro_rules! render {
  (
    in:           $arg_surface          : expr, // Eg: in: surface
    component_id: $arg_component_id     : expr, // Eg: "component1"
    from:         $arg_registry         : expr, // Eg: from: registry
    state:        $arg_state            : expr, // Eg: state
    shared_store: $arg_shared_store     : expr, // Eg: shared_store
    shared_tw_data: $arg_shared_tw_data : expr, // Eg: shared_tw_data
    window_size:  $arg_window_size      : expr // Eg: window_size
  ) => {
    let maybe_component_ref = ComponentRegistry::get_component_ref_by_id(&mut $arg_registry, $arg_component_id);

    if let Some(component_ref) = maybe_component_ref {
      let current_box = $arg_surface.current_box()?;
      let queue = component_ref
        .write()
        .await
        .render(
          ComponentScopeArgs {
            shared_tw_data: $arg_shared_tw_data,
            shared_store: $arg_shared_store,
            state: $arg_state,
            component_registry: &mut $arg_registry,
            window_size: $arg_window_size,
          },
          current_box,
        )
        .await?;
      $arg_surface.render_pipeline += queue;
    }
  };
}
