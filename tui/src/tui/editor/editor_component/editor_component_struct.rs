/*
 *   Copyright (c) 2022-2025 R3BL LLC
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

use tokio::sync::mpsc::Sender;

use crate::{BoxedSafeComponent, CommonResult, Component, DEFAULT_SYN_HI_FILE_EXT,
            EditorBuffer, EditorEngine, EditorEngineApplyEventResult,
            EditorEngineConfig, EventPropagation, FlexBox, FlexBoxId, GlobalData,
            HasEditorBuffers, HasFocus, InputEvent, RenderPipeline, SurfaceBounds,
            SystemClipboard, TerminalWindowMainThreadSignal,
            editor_engine::engine_public_api, ok};

#[derive(Debug)]
/// This is a shim which allows the reusable [`EditorEngine`] to be used in the context of
/// [`crate::Component`] and [`crate::App`].
///
/// The main methods here simply pass thru all their
/// arguments to the [`EditorEngine`].
pub struct EditorComponent<S, AS>
where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
{
    pub data: EditorComponentData<S, AS>,
}

#[derive(Debug, Default)]
pub struct EditorComponentData<S, AS>
where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
{
    pub editor_engine: EditorEngine,
    pub id: FlexBoxId,
    pub on_editor_buffer_change_handler: Option<OnEditorBufferChangeFn<AS>>,
    _phantom: std::marker::PhantomData<S>,
}

pub type OnEditorBufferChangeFn<A> =
    fn(FlexBoxId, Sender<TerminalWindowMainThreadSignal<A>>);

pub mod editor_component_impl_component_trait {
    use super::{CommonResult, Component, DEFAULT_SYN_HI_FILE_EXT, Debug, EditorBuffer,
                EditorComponent, EditorComponentData, EditorEngineApplyEventResult,
                EventPropagation, FlexBox, FlexBoxId, GlobalData, HasEditorBuffers,
                HasFocus, InputEvent, RenderPipeline, SurfaceBounds, SystemClipboard,
                engine_public_api, ok};

    fn get_existing_mut_editor_buffer_from_state_or_create_new_one<S>(
        mut_state: &mut S,
        self_id: FlexBoxId,
    ) -> &mut EditorBuffer
    where
        S: HasEditorBuffers + Default + Clone + Debug + Sync + Send,
    {
        // Add an empty editor buffer if it doesn't exist.
        if !mut_state.contains_editor_buffer(self_id) {
            let it = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
            mut_state.insert_editor_buffer(self_id, it);
        }
        // Safe to call unwrap here, since we are guaranteed to have an editor buffer.
        mut_state.get_mut_editor_buffer(self_id).unwrap()
    }

    impl<S, AS> Component<S, AS> for EditorComponent<S, AS>
    where
        S: HasEditorBuffers + Default + Clone + Debug + Sync + Send,
        AS: Debug + Default + Clone + Sync + Send,
    {
        fn reset(&mut self) { self.data.editor_engine.clear_ast_cache(); }

        fn get_id(&self) -> FlexBoxId { self.data.id }

        /// This shim simply calls [`engine_public_api::render_engine`] w/ all the
        /// necessary arguments:
        /// - Global scope: [`GlobalData`] containing app state.
        /// - Current box: [`FlexBox`] containing the current box's bounds.
        /// - Has focus: [`HasFocus`] containing whether the current box has focus.
        fn render(
            &mut self,
            global_data: &mut GlobalData<S, AS>,
            current_box: FlexBox,
            _surface_bounds: SurfaceBounds, /* Ignore this. */
            has_focus: &mut HasFocus,
        ) -> CommonResult<RenderPipeline> {
            let GlobalData { state, .. } = global_data;

            let EditorComponentData {
                editor_engine, id, ..
            } = &mut self.data;

            let self_id = *id;

            let editor_buffer =
                get_existing_mut_editor_buffer_from_state_or_create_new_one(
                    state, self_id,
                );

            engine_public_api::render_engine(
                editor_engine,
                editor_buffer,
                current_box,
                has_focus,
                global_data.window_size,
            )
        }

        /// This shim simply calls [`engine_public_api::apply_event`] w/ all the
        /// necessary arguments:
        /// - Global scope: [`GlobalData`] (containing app state).
        /// - User input (from [`crate::main_event_loop`]): [`InputEvent`].
        ///
        /// Usually a component must have focus in order for the [`crate::App`] to
        /// [`route_event_to_focused_component`](crate::ComponentRegistry::route_event_to_focused_component)
        /// in the first place.
        fn handle_event(
            &mut self,
            global_data: &mut GlobalData<S, AS>,
            input_event: InputEvent,
            _: &mut HasFocus,
        ) -> CommonResult<EventPropagation> {
            let GlobalData { state, .. } = global_data;

            let EditorComponentData {
                editor_engine,
                id,
                on_editor_buffer_change_handler,
                ..
            } = &mut self.data;

            let self_id = *id;

            let mut_editor_buffer: &mut EditorBuffer =
                get_existing_mut_editor_buffer_from_state_or_create_new_one(
                    state, self_id,
                );

            // XMARK: Editor component processes input event here
            // Try to apply the `input_event` to `editor_engine` to decide whether to
            // fire action.
            let result = engine_public_api::apply_event(
                mut_editor_buffer,
                editor_engine,
                input_event,
                &mut SystemClipboard,
            )?;

            ok!(match result {
                EditorEngineApplyEventResult::Applied => {
                    if let Some(on_change_handler) = on_editor_buffer_change_handler {
                        on_change_handler(
                            self_id,
                            global_data.main_thread_channel_sender.clone(),
                        );
                    }
                    EventPropagation::Consumed
                }
                EditorEngineApplyEventResult::NotApplied => {
                    // Optional: handle any `input_event` not consumed by
                    // `editor_engine`.
                    EventPropagation::Propagate
                }
            })
        }
    }
}

pub mod constructor {
    use super::{BoxedSafeComponent, Debug, EditorComponent, EditorComponentData,
                EditorEngine, EditorEngineConfig, FlexBoxId, HasEditorBuffers,
                OnEditorBufferChangeFn};

    impl<S, AS> EditorComponent<S, AS>
    where
        S: Debug + Default + Clone + Sync + Send + HasEditorBuffers + 'static,
        AS: Debug + Default + Clone + Sync + Send + 'static,
    {
        /// The `on_change_handler` is a lambda that is called if the editor buffer
        /// changes. Typically this results in a Redux action being created and
        /// then dispatched to the given store.
        pub fn new(
            id: FlexBoxId,
            config_options: EditorEngineConfig,
            on_buffer_change: OnEditorBufferChangeFn<AS>,
        ) -> Self {
            Self {
                data: EditorComponentData {
                    editor_engine: EditorEngine::new(config_options),
                    id,
                    on_editor_buffer_change_handler: Some(on_buffer_change),
                    ..Default::default()
                },
            }
        }

        pub fn new_boxed(
            id: FlexBoxId,
            config_options: EditorEngineConfig,
            on_buffer_change: OnEditorBufferChangeFn<AS>,
        ) -> BoxedSafeComponent<S, AS> {
            let it = EditorComponent::new(id, config_options, on_buffer_change);
            Box::new(it)
        }
    }
}
