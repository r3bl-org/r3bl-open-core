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

use std::{fmt::Debug, sync::Arc};

use async_trait::async_trait;
use r3bl_rs_utils_core::*;
use tokio::sync::RwLock;

use crate::*;

/// An app is typically a holder for [ComponentRegistry]. It then lays out a bunch of [Component]s
/// on its [Surface] which do all the work of rendering and input event handling. There are examples
/// of structs that implement this train in the [examples
/// module](https://github.com/r3bl-org/r3bl-open-core/blob/autocomplete/tui/examples/demo/ex_editor/app.rs).
///
/// Notes:
/// - Async trait docs: <https://doc.rust-lang.org/book/ch10-02-traits.html>
/// - Limitations of linking to examples module: <https://users.rust-lang.org/t/how-to-link-to-examples/67918>
#[async_trait]
pub trait App<S, A>
where
    S: Debug + Default + Clone + PartialEq + Sync + Send,
    A: Debug + Default + Clone + Sync + Send,
{
    /// Use the state to render the output (via crossterm). The state is immutable. If you want to
    /// change it then it should be done in the [App::app_handle_event] method.
    ///
    /// More than likely a bunch of other [Component::render]s will perform the actual rendering.
    async fn app_render(
        &mut self,
        args: GlobalScopeArgs<'_, S, A>,
    ) -> CommonResult<RenderPipeline>;

    /// At a high level:
    /// - Use the `input_event` to dispatch an action to the store if needed.
    /// - It returns an [EventPropagation].
    ///
    /// More than likely a bunch of other [Component::handle_event]s will perform the actual event
    /// handling.
    async fn app_handle_event(
        &mut self,
        args: GlobalScopeArgs<'_, S, A>,
        input_event: &InputEvent,
    ) -> CommonResult<EventPropagation>;

    /// Wrap a new instance in [Box].
    fn new_owned() -> BoxedSafeApp<S, A>
    where
        Self: Default + Sync + Send + 'static,
    {
        Box::<Self>::default()
    }

    /// Wrap a new instance in [std::sync::Arc] & [tokio::sync::RwLock].
    fn new_shared() -> SharedApp<S, A>
    where
        Self: Default + Sync + Send + 'static,
    {
        Arc::new(RwLock::new({
            let mut it = Self::default();
            it.init();
            it
        }))
    }

    /// Called when [App::new_shared] runs.
    fn init(&mut self);

    /// It is a requirement that the [App] trait impl have one of these as a field.
    fn get_component_registry(&mut self) -> &mut ComponentRegistry<S, A>;
}
