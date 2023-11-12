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

use r3bl_rs_utils_core::*;

use crate::*;

/// An app is typically a holder for [ComponentRegistry]. It then lays out a bunch of [Component]s
/// on its [Surface] which do all the work of rendering and input event handling. There are examples
/// of structs that implement this train in the [examples
/// module](https://github.com/r3bl-org/r3bl-open-core/blob/autocomplete/tui/examples/demo/ex_editor/app.rs).
///
/// Notes:
/// - Async trait docs: <https://doc.rust-lang.org/book/ch10-02-traits.html>
/// - Limitations of linking to examples module: <https://users.rust-lang.org/t/how-to-link-to-examples/67918>
pub trait App {
    type S: Debug + Default + Clone + Sync + Send;
    type A: Debug + Default + Clone + Sync + Send;

    /// This is called once at the beginning of the app's lifecycle. It is used to
    /// initialize the [ComponentRegistryMap] and [HasFocus] structs. It is called before the
    /// first render by the [TerminalWindow::main_event_loop].
    fn app_init(
        &mut self,
        component_registry_map: &mut ComponentRegistryMap<Self::S, Self::A>,
        has_focus: &mut HasFocus,
    );

    /// At a high level:
    /// - Use the `input_event` to dispatch an action to the store if needed.
    /// - It returns an [EventPropagation].
    ///
    /// More than likely a bunch of other [Component::handle_event]s will perform the actual event
    /// handling.
    fn app_handle_input_event(
        &mut self,
        input_event: InputEvent,
        global_data: &mut GlobalData<Self::S, Self::A>,
        component_registry_map: &mut ComponentRegistryMap<Self::S, Self::A>,
        has_focus: &mut HasFocus,
    ) -> CommonResult<EventPropagation>;

    /// At a high level:
    /// - Use the `action` to dispatch an action to the store if needed.
    /// - It returns an [EventPropagation].
    ///
    /// More than likely a bunch of other [Component::handle_event]s will perform the actual event
    /// handling.
    fn app_handle_signal(
        &mut self,
        action: &Self::A,
        global_data: &mut GlobalData<Self::S, Self::A>,
    ) -> CommonResult<EventPropagation>;

    /// Use the state to render the output (via crossterm). The state is immutable. If you want to
    /// change it then it should be done in the [App::app_handle_input_event] method.
    ///
    /// More than likely a bunch of other [Component::render]s will perform the actual rendering.
    fn app_render(
        &mut self,
        global_data: &mut GlobalData<Self::S, Self::A>,
        component_registry_map: &mut ComponentRegistryMap<Self::S, Self::A>,
        has_focus: &mut HasFocus,
    ) -> CommonResult<RenderPipeline>;
}
