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
use tokio::sync::mpsc::Sender;

use crate::*;

#[derive(Debug)]
/// This is a shim which allows the reusable [EditorEngine] to be used in the context of
/// [Component] and [App]. The main methods here simply pass thru all their
/// arguments to the [EditorEngine].
pub struct EditorComponent<S, A>
where
    S: Debug + Default + Clone + Sync + Send,
    A: Debug + Default + Clone + Sync + Send,
{
    pub data: EditorComponentData<S, A>,
}

#[derive(Debug, Default)]
pub struct EditorComponentData<S, A>
where
    S: Debug + Default + Clone + Sync + Send,
    A: Debug + Default + Clone + Sync + Send,
{
    pub editor_engine: EditorEngine,
    pub id: FlexBoxId,
    pub on_editor_buffer_change_handler: Option<OnEditorBufferChangeFn<A>>,
    _phantom: std::marker::PhantomData<S>,
}

pub type OnEditorBufferChangeFn<A> =
    fn(FlexBoxId, Sender<TerminalWindowMainThreadSignal<A>>);

pub mod editor_component_impl_component_trait {
    use super::*;
    use crate::editor_buffer_clipboard_support::system_clipboard_service_provider::SystemClipboard;

    fn get_existing_mut_editor_buffer_from_state_or_create_new_one<'a, S>(
        mut_state: &'a mut S,
        self_id: FlexBoxId,
        maybe_file_extension_for_new_empty_buffer: Option<String>,
    ) -> &'a mut EditorBuffer
    where
        S: HasEditorBuffers + Default + Clone + Debug + Sync + Send,
    {
        // Add an empty editor buffer if it doesn't exist.
        if !mut_state.contains_editor_buffer(self_id) {
            let it = EditorBuffer::new_empty(maybe_file_extension_for_new_empty_buffer);
            mut_state.insert_editor_buffer(self_id, it);
        }
        // Safe to call unwrap here, since we are guaranteed to have an editor buffer.
        mut_state.get_mut_editor_buffer(self_id).unwrap()
    }

    impl<S, A> Component<S, A> for EditorComponent<S, A>
    where
        S: HasEditorBuffers + Default + Clone + Debug + Sync + Send,
        A: Debug + Default + Clone + Sync + Send,
    {
        fn reset(&mut self) {}

        fn get_id(&self) -> FlexBoxId { self.data.id }

        /// This shim simply calls
        /// [EditorEngineApi::render_engine](EditorEngineApi::render_engine) w/ all the
        /// necessary arguments:
        /// - Global scope: [GlobalData] containing app state.
        /// - Current box: [FlexBox] containing the current box's bounds.
        /// - Has focus: [HasFocus] containing whether the current box has focus.
        fn render(
            &mut self,
            global_data: &mut GlobalData<S, A>,
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
                    state,
                    self_id,
                    editor_engine
                        .config_options
                        .syntax_highlight
                        .get_file_extension_for_new_empty_buffer(),
                );

            EditorEngineApi::render_engine(
                editor_engine,
                editor_buffer,
                current_box,
                has_focus,
                global_data.window_size,
            )
        }

        /// This shim simply calls [EditorEngineApi::apply_event](EditorEngineApi::apply_event) w/
        /// all the necessary arguments:
        /// - Global scope: [GlobalData] (containing app state).
        /// - User input (from [main_event_loop]): [InputEvent].
        ///
        /// Usually a component must have focus in order for the [App] to
        /// [route_event_to_focused_component](ComponentRegistry::route_event_to_focused_component)
        /// in the first place.
        fn handle_event(
            &mut self,
            global_data: &mut GlobalData<S, A>,
            input_event: InputEvent,
            _: &mut HasFocus,
        ) -> CommonResult<EventPropagation> {
            throws_with_return!({
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
                        state,
                        self_id,
                        editor_engine
                            .config_options
                            .syntax_highlight
                            .get_file_extension_for_new_empty_buffer(),
                    );

                // BOOKM: editor component processes input event here
                // Try to apply the `input_event` to `editor_engine` to decide whether to
                // fire action.
                let result = EditorEngineApi::apply_event(
                    mut_editor_buffer,
                    editor_engine,
                    input_event,
                    &mut SystemClipboard,
                )?;

                match result {
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
                        // Optional: handle any `input_event` not consumed by `editor_engine`.
                        EventPropagation::Propagate
                    }
                }
            });
        }
    }
}

pub mod constructor {
    use super::*;

    impl<S, A> EditorComponent<S, A>
    where
        S: Debug + Default + Clone + Sync + Send + HasEditorBuffers + 'static,
        A: Debug + Default + Clone + Sync + Send + 'static,
    {
        /// The on_change_handler is a lambda that is called if the editor buffer changes. Typically this
        /// results in a Redux action being created and then dispatched to the given store.
        pub fn new(
            id: FlexBoxId,
            config_options: EditorEngineConfig,
            on_buffer_change: OnEditorBufferChangeFn<A>,
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
            on_buffer_change: OnEditorBufferChangeFn<A>,
        ) -> BoxedSafeComponent<S, A> {
            let it = EditorComponent::new(id, config_options, on_buffer_change);
            Box::new(it)
        }
    }
}
