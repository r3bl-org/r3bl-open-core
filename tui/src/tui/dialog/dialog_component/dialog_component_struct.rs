// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::fmt::Debug;

use crate::{Component, DEBUG_TUI_MOD, DialogEngine, DialogEngineApi,
            DialogEngineApplyResponse, DialogEngineArgs, DialogEngineConfigOptions,
            EditorEngineConfig, EventPropagation, FlexBox, FlexBoxId, GlobalData,
            HasDialogBuffers, HasFocus, InputEvent, OnDialogEditorChangeFn,
            OnDialogPressFn, RenderPipeline, SurfaceBounds,
            common::{CommonError, CommonErrorType, CommonResult}};

/// This is a shim which allows the reusable [`DialogEngine`] to be used in the context of
/// [Component]. The main methods here simply pass thru all their arguments to the
/// [`DialogEngine`].
#[derive(Debug, Default)]
pub struct DialogComponent<S, AS>
where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
{
    pub data: DialogComponentData<S, AS>,
}

#[derive(Debug, Default)]
pub struct DialogComponentData<S, AS>
where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
{
    pub id: FlexBoxId,
    pub dialog_engine: DialogEngine,
    /// Make sure to dispatch actions to handle the user's dialog choice
    /// [`crate::DialogChoice`].
    pub on_dialog_press_handler: Option<OnDialogPressFn<S, AS>>,
    /// Make sure to dispatch an action to update the dialog buffer's editor buffer.
    pub on_dialog_editor_change_handler: Option<OnDialogEditorChangeFn<S, AS>>,
    _phantom: std::marker::PhantomData<AS>,
}

impl<S, AS> Component<S, AS> for DialogComponent<S, AS>
where
    S: Debug + Default + Clone + Sync + Send + HasDialogBuffers,
    AS: Debug + Default + Clone + Sync + Send,
{
    fn reset(&mut self) { self.data.dialog_engine.reset(); }

    fn get_id(&self) -> FlexBoxId { self.data.id }

    /// This shim simply calls
    /// [DialogEngineApi::render_engine](DialogEngineApi::render_engine) w/ all the
    /// necessary arguments:
    /// - Global scope: [`GlobalData`] containing the app's state.
    /// - Has focus: [`HasFocus`] containing whether the current box has focus.
    /// - Surface bounds: [`SurfaceBounds`] containing the bounds of the current box.
    ///
    /// Note:
    /// 1. The 3rd argument `_current_box` [`FlexBox`] is ignored since the dialog
    ///    component breaks out of whatever box the layout places it in, and ends up
    ///    painting itself over the entire screen.
    /// 2. However, [`SurfaceBounds`] is saved for later use. And it is used to restrict
    ///    where the dialog can be placed on the screen.
    fn render(
        &mut self,
        global_data: &mut GlobalData<S, AS>,
        _current_box: FlexBox,         /* Ignore this. */
        surface_bounds: SurfaceBounds, /* Save this. */
        has_focus: &mut HasFocus,
    ) -> CommonResult<RenderPipeline> {
        // Unpack the global data.
        let GlobalData { state, .. } = global_data;

        // Unpack the component data.
        let DialogComponentData {
            id, dialog_engine, ..
        } = &mut self.data;

        let self_id = *id;

        dialog_engine.maybe_surface_bounds = Some(surface_bounds);

        match state.get_mut_dialog_buffer(self_id) {
            Some(_) => {
                let args = {
                    DialogEngineArgs {
                        self_id,
                        global_data,
                        engine: dialog_engine,
                        has_focus,
                    }
                };
                DialogEngineApi::render_engine(args)
            }
            None => Ok(RenderPipeline::default()),
        }
    }

    /// This shim simply calls
    /// [DialogEngineApi::apply_event](DialogEngineApi::apply_event) w/ all the necessary
    /// arguments:
    /// - Global scope: [`GlobalData`] containing the app's state.
    /// - User input (from [`crate::main_event_loop`]): [`InputEvent`].
    /// - Has focus: [`HasFocus`] containing whether the current box has focus.
    ///
    /// Usually a component must have focus in order for the [`crate::App`] to
    /// [`route_event_to_focused_component`](crate::ComponentRegistry::route_event_to_focused_component)
    /// in the first place.
    fn handle_event(
        &mut self,
        global_data: &mut GlobalData<S, AS>,
        input_event: InputEvent,
        has_focus: &mut HasFocus,
    ) -> CommonResult<EventPropagation> {
        // Unpack the global data.
        let GlobalData {
            state,
            main_thread_channel_sender,
            ..
        } = global_data;

        let DialogComponentData {
            id,
            dialog_engine,
            on_dialog_press_handler,
            on_dialog_editor_change_handler,
            ..
        } = &mut self.data;

        let id = *id;

        if state.get_mut_dialog_buffer(id).is_some() {
            use DialogEngineApplyResponse::{DialogChoice, Noop,
                                            SelectScrollResultsPanel, UpdateEditorBuffer};

            match DialogEngineApi::apply_event::<S, AS>(
                state,
                id,
                dialog_engine,
                input_event,
            )? {
                // Handler user's choice.
                DialogChoice(dialog_choice) => {
                    has_focus.reset_modal_id();

                    DEBUG_TUI_MOD.then(|| {
                        // % is Display, ? is Debug.
                        tracing::debug!(
                            message = "🐝 restore focus to non modal",
                            has_focus = ?has_focus
                        );
                    });

                    // Run the handler (if any) w/ `dialog_choice`.
                    if let Some(fun) = &on_dialog_press_handler {
                        fun(
                            dialog_choice,
                            state,
                            &mut main_thread_channel_sender.clone(),
                        );
                    }

                    // Trigger re-render, now that focus has been restored to
                    // non-modal component.
                    Ok(EventPropagation::ConsumedRender)
                }

                // Handler user input that has updated the
                // dialog_buffer.editor_buffer.
                UpdateEditorBuffer => {
                    // Run the handler (if any) w/ `new_editor_buffer`.
                    if let Some(it) = &on_dialog_editor_change_handler {
                        it(state, &mut main_thread_channel_sender.clone());
                    }

                    // The handler should dispatch action to change state since
                    // dialog_buffer.editor_buffer is updated.
                    Ok(EventPropagation::ConsumedRender)
                }

                // Handle user input that has updated the results panel.
                SelectScrollResultsPanel => Ok(EventPropagation::ConsumedRender),

                // All else.
                Noop => Ok(EventPropagation::Propagate),
            }
        } else {
            let msg = format!(
                "🐝 DialogComponent::handle_event: dialog_buffer is None for id: {id:?}"
            );
            CommonError::new_error_result(CommonErrorType::NotFound, &msg)
        }
    }
}

impl<S, AS> DialogComponent<S, AS>
where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
{
    /// The `on_dialog_press_handler` is a lambda that is called if the user presses enter
    /// or escape. Typically this results in a Redux action being created and then
    /// dispatched to the given store.
    pub fn new(
        id: FlexBoxId,
        dialog_options: DialogEngineConfigOptions,
        editor_options: EditorEngineConfig,
        on_dialog_press_handler: OnDialogPressFn<S, AS>,
        on_dialog_editor_change_handler: OnDialogEditorChangeFn<S, AS>,
    ) -> Self {
        let dialog_engine = DialogEngine::new(dialog_options, editor_options);
        Self {
            data: DialogComponentData {
                id,
                dialog_engine,
                on_dialog_press_handler: Some(on_dialog_press_handler),
                on_dialog_editor_change_handler: Some(on_dialog_editor_change_handler),
                ..Default::default()
            },
        }
    }

    pub fn new_boxed(
        id: FlexBoxId,
        dialog_options: DialogEngineConfigOptions,
        editor_options: EditorEngineConfig,
        on_dialog_press_handler: OnDialogPressFn<S, AS>,
        on_dialog_editor_change_handler: OnDialogEditorChangeFn<S, AS>,
    ) -> Box<Self> {
        let it = DialogComponent::new(
            id,
            dialog_options,
            editor_options,
            on_dialog_press_handler,
            on_dialog_editor_change_handler,
        );
        Box::new(it)
    }
}
