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
