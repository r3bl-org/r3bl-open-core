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

use std::{borrow::Cow, fmt::Debug, sync::Arc};

use async_trait::async_trait;
use r3bl_redux::*;
use r3bl_rs_utils_core::*;
use tokio::sync::RwLock;

use crate::*;

/// This is a shim which allows the reusable [EditorEngine] to be used in the context of [Component]
/// and [Store]. The main methods here simply pass thru all their arguments to the [EditorEngine].
#[derive(Clone, Default)]
pub struct EditorComponent<S, A>
where
    S: Debug + Default + Clone + PartialEq + Sync + Send,
    A: Debug + Default + Clone + Sync + Send,
{
    pub editor_engine: EditorEngine,
    pub id: FlexBoxId,
    pub on_editor_buffer_change_handler: Option<OnEditorBufferChangeFn<S, A>>,
}

pub type OnEditorBufferChangeFn<S, A> = fn(&SharedStore<S, A>, FlexBoxId, EditorBuffer);

pub mod editor_component_impl {
    use super::*;

    #[async_trait]
    impl<S, A> Component<S, A> for EditorComponent<S, A>
    where
        S: HasEditorBuffers + Default + Clone + PartialEq + Debug + Sync + Send,
        A: Debug + Default + Clone + Sync + Send,
    {
        fn reset(&mut self) {}

        fn get_id(&self) -> FlexBoxId { self.id }

        /// This shim simply calls [EditorEngineApi::apply_event](EditorEngineApi::apply_event) w/
        /// all the necessary arguments:
        /// - Global scope: [SharedStore], [SharedGlobalData].
        /// - App scope: `S`, [ComponentRegistry<S, A>].
        /// - User input (from [main_event_loop]): [InputEvent].
        ///
        /// Usually a component must have focus in order for the [App] to
        /// [route_event_to_focused_component](ComponentRegistry::route_event_to_focused_component)
        /// in the first place.
        async fn handle_event(
            &mut self,
            args: ComponentScopeArgs<'_, S, A>,
            input_event: &InputEvent,
        ) -> CommonResult<EventPropagation> {
            throws_with_return!({
                let ComponentScopeArgs {
                    shared_global_data,
                    shared_store,
                    state,
                    component_registry,
                    ..
                } = args;

                let cow_buffer: Cow<EditorBuffer> = {
                    // Either: get existing buffer ref from state.
                    if let Some(existing_buffer_ref) = state.get_editor_buffer(self.get_id()) {
                        Cow::Borrowed(existing_buffer_ref)
                    }
                    // Or: create a new owned buffer.
                    else {
                        Cow::Owned(EditorBuffer::new_empty(
                            self.editor_engine
                                .config_options
                                .syntax_highlight
                                .get_file_extension_for_new_empty_buffer(),
                        ))
                    }
                };

                // 00: editor component processes input event here
                // Try to apply the `input_event` to `editor_engine` to decide whether to fire
                // action.
                let result = EditorEngineApi::apply_event(
                    EditorEngineArgs {
                        state,
                        editor_buffer: &cow_buffer,
                        component_registry,
                        shared_global_data,
                        shared_store,
                        self_id: self.id,
                        editor_engine: &mut self.editor_engine,
                    },
                    input_event,
                )
                .await?;

                match result {
                    EditorEngineApplyEventResult::Applied(new_buffer) => {
                        if let Some(on_change_handler) = self.on_editor_buffer_change_handler {
                            on_change_handler(shared_store, self.get_id(), new_buffer);
                        }
                        EventPropagation::Consumed
                    }
                    EditorEngineApplyEventResult::NotApplied => {
                        // Optional: handle any `input_event` not consumed by `editor_engine`.
                        EventPropagation::Propagate
                    }
                }
            });
        }

        /// This shim simply calls [EditorEngineApi::render_engine](EditorEngineApi::render_engine)
        /// w/ all the necessary arguments:
        /// - Global scope: [SharedStore], [SharedGlobalData].
        /// - App scope: `S`, [ComponentRegistry<S, A>].
        /// - User input (from [main_event_loop]): [InputEvent].
        async fn render(
            &mut self,
            args: ComponentScopeArgs<'_, S, A>,
            current_box: &FlexBox,
            _surface_bounds: SurfaceBounds, /* Ignore this. */
        ) -> CommonResult<RenderPipeline> {
            let ComponentScopeArgs {
                state,
                shared_store,
                shared_global_data,
                component_registry,
                ..
            } = args;

            let my_buffer: Cow<EditorBuffer> = {
                if let Some(buffer) = state.get_editor_buffer(self.get_id()) {
                    Cow::Borrowed(buffer)
                } else {
                    Cow::Owned(EditorBuffer::new_empty(
                        self.editor_engine
                            .config_options
                            .syntax_highlight
                            .get_file_extension_for_new_empty_buffer(),
                    ))
                }
            };

            let render_args = EditorEngineArgs {
                editor_engine: &mut self.editor_engine,
                state,
                editor_buffer: &my_buffer,
                component_registry,
                shared_global_data,
                shared_store,
                self_id: self.id,
            };

            EditorEngineApi::render_engine(render_args, current_box).await
        }
    }
}
pub use editor_component_impl::*;

pub mod constructor {
    use super::*;

    impl<S, A> EditorComponent<S, A>
    where
        S: Debug + Default + Clone + PartialEq + Sync + Send,
        A: Debug + Default + Clone + Sync + Send,
    {
        /// The on_change_handler is a lambda that is called if the editor buffer changes. Typically this
        /// results in a Redux action being created and then dispatched to the given store.
        pub fn new(
            id: FlexBoxId,
            config_options: EditorEngineConfig,
            on_buffer_change: OnEditorBufferChangeFn<S, A>,
        ) -> Self {
            Self {
                editor_engine: EditorEngine::new(config_options),
                id,
                on_editor_buffer_change_handler: Some(on_buffer_change),
            }
        }

        pub fn new_shared(
            id: FlexBoxId,
            config_options: EditorEngineConfig,
            on_buffer_change: OnEditorBufferChangeFn<S, A>,
        ) -> Arc<RwLock<Self>> {
            Arc::new(RwLock::new(EditorComponent::new(
                id,
                config_options,
                on_buffer_change,
            )))
        }
    }
}
pub use constructor::*;
