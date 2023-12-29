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
 *   Unless required by applicable law or agreed &to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

use r3bl_rs_utils_core::*;
use r3bl_rs_utils_macro::style;
use r3bl_tui::*;

use super::*;

/// Constants for the ids.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Id {
    Editor = 1,
    SimpleDialog = 2,
    AutocompleteDialog = 3,
    EditorStyleNameDefault = 4,
    DialogStyleNameBorder = 5,
    DialogStyleNameTitle = 6,
    DialogStyleNameEditor = 7,
    DialogStyleNameResultsPanel = 8,
}

mod id_impl {
    use super::*;

    impl From<Id> for u8 {
        fn from(id: Id) -> u8 { id as u8 }
    }

    impl From<Id> for FlexBoxId {
        fn from(id: Id) -> FlexBoxId { FlexBoxId(id as u8) }
    }
}

pub struct AppMain;

mod constructor {
    use super::*;

    impl Default for AppMain {
        fn default() -> Self {
            call_if_true!(DEBUG_TUI_MOD, {
                let msg = format!("🪙 {}", "construct ex_rc::AppMain");
                log_debug(msg);
            });
            Self
        }
    }

    impl AppMain {
        /// Note that this needs to be initialized before it can be used.
        pub fn new_boxed() -> BoxedSafeApp<State, AppSignal> {
            let it = Self::default();
            Box::new(it)
        }
    }
}

mod app_main_impl_app_trait {
    use super::*;

    impl App for AppMain {
        type S = State;
        type AS = AppSignal;

        fn app_init(
            &mut self,
            component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
            has_focus: &mut HasFocus,
        ) {
            populate_component_registry::create_components(
                component_registry_map,
                has_focus,
            );
        }

        fn app_handle_input_event(
            &mut self,
            input_event: InputEvent,
            global_data: &mut GlobalData<State, AppSignal>,
            component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
            has_focus: &mut HasFocus,
        ) -> CommonResult<EventPropagation> {
            // Things from global scope.
            let GlobalData { state, .. } = global_data;

            // Check to see if the modal dialog should be activated.
            if let modal_dialogs::ModalActivateResult::Yes =
                modal_dialogs::should_activate(
                    input_event.clone(),
                    component_registry_map,
                    has_focus,
                    state,
                )
            {
                return Ok(EventPropagation::ConsumedRender);
            }

            // If modal not activated, route the input event to the focused component.
            ComponentRegistry::route_event_to_focused_component(
                global_data,
                input_event.clone(),
                component_registry_map,
                has_focus,
            )
        }

        fn app_handle_signal(
            &mut self,
            _action: &AppSignal,
            _global_data: &mut GlobalData<State, AppSignal>,
        ) -> CommonResult<EventPropagation> {
            Ok(EventPropagation::ConsumedRender)
        }

        fn app_render(
            &mut self,
            global_data: &mut GlobalData<State, AppSignal>,
            component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
            has_focus: &mut HasFocus,
        ) -> CommonResult<RenderPipeline> {
            throws_with_return!({
                let window_size = global_data.window_size;

                // Create a surface and then run the SurfaceRenderer (ContainerSurfaceRender) on it.
                let mut surface = {
                    let mut it = surface!(stylesheet: stylesheet::create_stylesheet()?);

                    it.surface_start(SurfaceProps {
                        pos: position!(col_index: 0, row_index: 0),
                        size: size!(
                            col_count: window_size.col_count,
                            row_count: window_size.row_count - 1), // Bottom row for for status bar.
                    })?;

                    perform_layout::ContainerSurfaceRender { app: self }
                        .render_in_surface(
                            &mut it,
                            global_data,
                            component_registry_map,
                            has_focus,
                        )?;

                    it.surface_end()?;

                    it
                };

                // Render status bar.
                status_bar::render_status_bar(&mut surface.render_pipeline, window_size);

                // Return RenderOps pipeline (which will actually be painted elsewhere).
                surface.render_pipeline
            });
        }
    }
}

mod modal_dialogs {
    use super::*;

    // This runs on every keystroke, so it should be fast.
    pub fn dialog_component_update_content(state: &mut State, id: FlexBoxId) {
        // This is Some only if the content has changed (ignoring caret movements).
        let maybe_changed_results: Option<Vec<String>> = {
            if let Some(dialog_buffer) = state.dialog_buffers.get_mut(&id) {
                let vec_result = generate_random_results(
                    dialog_buffer.editor_buffer.get_as_string().as_str(),
                );
                Some(vec_result)
            } else {
                None
            }
        };

        state
            .dialog_buffers
            .entry(id)
            .and_modify(|it| {
                if let Some(results) = maybe_changed_results {
                    it.maybe_results = Some(results);
                }
            })
            .or_insert_with(
                // This code path should never execute, since to update the buffer given an id,
                // it should have already existed in the first place, which is created by:
                // 1. [Action::SimpleDialogComponentInitializeFocused].
                // 2. [Action::AutocompleteDialogComponentInitializeFocused].
                || {
                    let mut it = DialogBuffer::new_empty();
                    it.editor_buffer = EditorBuffer::new_empty(None);
                    it
                },
            );

        // Content is empty.
        if let Some(dialog_buffer) = state.dialog_buffers.get_mut(&id) {
            if dialog_buffer.editor_buffer.get_as_string() == "" {
                if let Some(it) = state.dialog_buffers.get_mut(&id) {
                    it.maybe_results = None;
                }
            }
        }
    }

    fn generate_random_results(content: &str) -> Vec<String> {
        let vec_result = {
            let start_rand_num = rand::random::<u8>() as usize;
            let max = 10;
            let mut it = Vec::with_capacity(max);
            for index in start_rand_num..(start_rand_num + max) {
                it.push(format!("{content}{index}"));
            }
            it
        };
        vec_result
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum ModalActivateResult {
        Yes,
        No,
    }

    pub fn should_activate(
        input_event: InputEvent,
        component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
        has_focus: &mut HasFocus,
        state: &mut State,
    ) -> ModalActivateResult {
        // "Ctrl + l" => activate Simple.
        if input_event.matches_keypress(KeyPress::WithModifiers {
            key: Key::Character('l'),
            mask: ModifierKeysMask::new().with_ctrl(),
        }) {
            // Reset the dialog component prior to activating / showing it.
            ComponentRegistry::reset_component(
                component_registry_map,
                FlexBoxId::from(Id::SimpleDialog),
            );
            return match activate_simple_modal(component_registry_map, has_focus, state) {
                Ok(_) => ModalActivateResult::Yes,
                Err(err) => {
                    if let Some(CommonError {
                        err_type: _,
                        err_msg: msg,
                    }) = err.downcast_ref::<CommonError>()
                    {
                        log_error(format!("📣 Error activating simple modal: {msg:?}"));
                    }
                    ModalActivateResult::No
                }
            };
        };

        // "Ctrl + k" => activate Autocomplete.
        if input_event.matches_keypress(KeyPress::WithModifiers {
            key: Key::Character('k'),
            mask: ModifierKeysMask::new().with_ctrl(),
        }) {
            // Reset the dialog component prior to activating / showing it.
            ComponentRegistry::reset_component(
                component_registry_map,
                FlexBoxId::from(Id::AutocompleteDialog),
            );
            return match activate_autocomplete_modal(
                component_registry_map,
                has_focus,
                state,
            ) {
                Ok(_) => ModalActivateResult::Yes,
                Err(err) => {
                    if let Some(CommonError {
                        err_type: _,
                        err_msg: msg,
                    }) = err.downcast_ref::<CommonError>()
                    {
                        log_error(format!(
                            "📣 Error activating autocomplete modal: {msg:?}"
                        ));
                    }
                    ModalActivateResult::No
                }
            };
        };

        ModalActivateResult::No
    }

    /// If `input_event` matches <kbd>Ctrl+l</kbd> or <kbd>Ctrl+k</kbd>, then toggle the modal
    /// dialog.
    ///
    /// Note that this returns a [EventPropagation::Consumed] and not
    /// [EventPropagation::ConsumedRender] because both the following dispatched to the store &
    /// that will cause a rerender:
    /// 1. [Action::SimpleDialogComponentInitializeFocused].
    /// 2. [Action::AutocompleteDialogComponentInitializeFocused].
    pub fn dialog_component_initialize_focused(
        state: &mut State,
        id: FlexBoxId,
        title: String,
        text: String,
    ) {
        let dialog_buffer = {
            let mut it = DialogBuffer::new_empty();
            it.title = title.into();
            let max_width = 100;
            let line: String = {
                if text.is_empty() {
                    "".to_string()
                } else if text.len() > max_width {
                    text.split_at(max_width).0.to_string()
                } else {
                    text.clone()
                }
            };
            it.editor_buffer.set_lines(vec![line]);
            it
        };
        state.dialog_buffers.insert(id, dialog_buffer);
    }

    fn activate_simple_modal(
        _component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
        has_focus: &mut HasFocus,
        state: &mut State,
    ) -> CommonResult<()> {
        throws!({
            // Initialize the dialog buffer with title & text.
            let title = "Simple Modal Dialog Title";
            let text = {
                if let Some(editor_buffer) =
                    state.get_mut_editor_buffer(FlexBoxId::from(Id::Editor))
                {
                    editor_buffer.get_as_string()
                } else {
                    "".to_string()
                }
            };

            // Setting the has_focus to ComponentId::SimpleDialog will cause the dialog to
            // appear on the next render.
            has_focus.try_set_modal_id(FlexBoxId::from(Id::SimpleDialog))?;

            // Change the state so that it will trigger a render. This will show the title
            // & text on the next render.
            dialog_component_initialize_focused(
                state,
                FlexBoxId::from(Id::SimpleDialog),
                title.to_owned(),
                text,
            );

            call_if_true!(DEBUG_TUI_MOD, {
                let msg = format!("📣 activate modal simple: {:?}", has_focus);
                log_debug(msg);
            });
        });
    }

    fn activate_autocomplete_modal(
        _component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
        has_focus: &mut HasFocus,
        state: &mut State,
    ) -> CommonResult<()> {
        // Initialize the dialog buffer with title & text.
        let title = "Autocomplete Modal Dialog Title";
        let text = {
            if let Some(editor_buffer) =
                state.get_mut_editor_buffer(FlexBoxId::from(Id::Editor))
            {
                editor_buffer.get_as_string()
            } else {
                "".to_string()
            }
        };

        // Setting the has_focus to Id::Dialog will cause the dialog to appear on the next
        // render.
        has_focus.try_set_modal_id(FlexBoxId::from(Id::AutocompleteDialog))?;

        // Change the state so that it will trigger a render. This will show the title &
        // text on the next render.
        dialog_component_initialize_focused(
            state,
            FlexBoxId::from(Id::AutocompleteDialog),
            title.to_owned(),
            text,
        );

        call_if_true!(DEBUG_TUI_MOD, {
            let msg = format!("📣 activate modal autocomplete: {:?}", has_focus);
            log_debug(msg);
        });

        Ok(())
    }
}

mod perform_layout {
    use super::*;

    pub struct ContainerSurfaceRender<'a> {
        pub app: &'a mut AppMain,
    }

    impl SurfaceRender<State, AppSignal> for ContainerSurfaceRender<'_> {
        fn render_in_surface(
            &mut self,
            surface: &mut Surface,
            global_data: &mut GlobalData<State, AppSignal>,
            component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
            has_focus: &mut HasFocus,
        ) -> CommonResult<()> {
            throws!({
                // Layout editor component, and render it.
                {
                    box_start! (
                        in:                     surface,
                        id:                     FlexBoxId::from(Id::Editor),
                        dir:                    LayoutDirection::Vertical,
                        requested_size_percent: requested_size_percent!(width: 100, height: 100),
                        styles:                 [Id::EditorStyleNameDefault.into()]
                    );
                    render_component_in_current_box!(
                        in:                 surface,
                        component_id:       FlexBoxId::from(Id::Editor),
                        from:               component_registry_map,
                        global_data:        global_data,
                        has_focus:          has_focus
                    );
                    box_end!(in: surface);
                }

                // Then, render simple modal dialog (if it is active, on top of the editor
                // component).
                if has_focus.is_modal_id(FlexBoxId::from(Id::SimpleDialog)) {
                    render_component_in_given_box! {
                      in:                 surface,
                      box:                FlexBox::default(), /* This is not used as the modal breaks out of its box. */
                      component_id:       FlexBoxId::from(Id::SimpleDialog),
                      from:               component_registry_map,
                      global_data:        global_data,
                      has_focus:          has_focus
                    };
                }

                // Or, render autocomplete modal dialog (if it is active, on top of the editor
                // component).
                if has_focus.is_modal_id(FlexBoxId::from(Id::AutocompleteDialog)) {
                    render_component_in_given_box! {
                      in:                 surface,
                      box:                FlexBox::default(), /* This is not used as the modal breaks out of its box. */
                      component_id:       FlexBoxId::from(Id::AutocompleteDialog),
                      from:               component_registry_map,
                      global_data:        global_data,
                      has_focus:          has_focus
                    };
                }
            });
        }
    }
}

mod populate_component_registry {
    use tokio::sync::mpsc::Sender;

    use super::*;

    pub fn create_components(
        component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
        has_focus: &mut HasFocus,
    ) {
        insert_editor_component(component_registry_map);
        insert_dialog_component_simple(component_registry_map);
        insert_dialog_component_autocomplete(component_registry_map);

        // Switch focus to the editor component if focus is not set.
        let id = FlexBoxId::from(Id::Editor);
        has_focus.set_id(id);

        call_if_true!(DEBUG_TUI_MOD, {
            {
                let msg = format!("🪙 {} = {:?}", "init has_focus", has_focus.get_id());
                log_debug(msg);
            }
        });
    }

    /// Insert editor component into registry if it's not already there.
    fn insert_editor_component(
        component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
    ) {
        let id = FlexBoxId::from(Id::Editor);
        let boxed_editor_component = {
            fn on_buffer_change(
                my_id: FlexBoxId,
                main_thread_channel_sender: Sender<
                    TerminalWindowMainThreadSignal<AppSignal>,
                >,
            ) {
                send_signal!(
                    main_thread_channel_sender,
                    TerminalWindowMainThreadSignal::Render(Some(my_id))
                );
            }

            let config_options = EditorEngineConfig::default();
            EditorComponent::new_boxed(id, config_options, on_buffer_change)
        };

        ComponentRegistry::put(component_registry_map, id, boxed_editor_component);

        call_if_true!(DEBUG_TUI_MOD, {
            let msg = format!("🪙 {}", "construct EditorComponent { on_buffer_change }");
            log_debug(msg);
        });
    }

    /// Insert simple dialog component into registry if it's not already there.
    fn insert_dialog_component_simple(
        component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
    ) {
        let result_stylesheet = stylesheet::create_stylesheet();

        let dialog_options = DialogEngineConfigOptions {
            mode: DialogEngineMode::ModalSimple,
            maybe_style_border: get_style! { @from_result: result_stylesheet , Id::DialogStyleNameBorder.into() },
            maybe_style_title: get_style! { @from_result: result_stylesheet , Id::DialogStyleNameTitle.into() },
            maybe_style_editor: get_style! { @from_result: result_stylesheet , Id::DialogStyleNameEditor.into() },
            maybe_style_results_panel: get_style! { @from_result: result_stylesheet , Id::DialogStyleNameResultsPanel.into() },
            ..Default::default()
        };

        let editor_options = EditorEngineConfig {
            multiline_mode: LineMode::SingleLine,
            syntax_highlight: SyntaxHighlightMode::Disable,
            edit_mode: EditMode::ReadWrite,
        };

        let boxed_dialog_component = {
            let it = DialogComponent::new_boxed(
                FlexBoxId::from(Id::SimpleDialog),
                dialog_options,
                editor_options,
                on_dialog_press_handler,
                on_dialog_editor_change_handler,
            );

            fn on_dialog_press_handler(dialog_choice: DialogChoice, state: &mut State) {
                match dialog_choice {
                    DialogChoice::Yes(text) => {
                        modal_dialogs::dialog_component_initialize_focused(
                            state,
                            FlexBoxId::from(Id::SimpleDialog),
                            "Yes".to_string(),
                            text,
                        );
                    }
                    DialogChoice::No => {
                        modal_dialogs::dialog_component_initialize_focused(
                            state,
                            FlexBoxId::from(Id::SimpleDialog),
                            "No".to_string(),
                            "".to_string(),
                        );
                    }
                }
            }

            fn on_dialog_editor_change_handler(state: &mut State) {
                modal_dialogs::dialog_component_update_content(
                    state,
                    FlexBoxId::from(Id::SimpleDialog),
                );
            }

            it
        };

        ComponentRegistry::put(
            component_registry_map,
            FlexBoxId::from(Id::SimpleDialog),
            boxed_dialog_component,
        );

        call_if_true!(DEBUG_TUI_MOD, {
            let msg = format!(
                "🪙 {}",
                "construct DialogComponent (simple) { on_dialog_press }"
            );
            log_debug(msg);
        });
    }

    /// Insert autocomplete dialog component into registry if it's not already there.
    fn insert_dialog_component_autocomplete(
        component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
    ) {
        let result_stylesheet = stylesheet::create_stylesheet();

        let dialog_options = DialogEngineConfigOptions {
            mode: DialogEngineMode::ModalAutocomplete,
            maybe_style_border: get_style! { @from_result: result_stylesheet , Id::DialogStyleNameBorder.into() },
            maybe_style_title: get_style! { @from_result: result_stylesheet , Id::DialogStyleNameTitle.into() },
            maybe_style_editor: get_style! { @from_result: result_stylesheet , Id::DialogStyleNameEditor.into() },
            maybe_style_results_panel: get_style! { @from_result: result_stylesheet , Id::DialogStyleNameResultsPanel.into() },
            ..Default::default()
        };

        let editor_options = EditorEngineConfig {
            multiline_mode: LineMode::SingleLine,
            syntax_highlight: SyntaxHighlightMode::Disable,
            edit_mode: EditMode::ReadWrite,
        };

        let boxed_dialog_component = {
            let it = DialogComponent::new_boxed(
                FlexBoxId::from(Id::AutocompleteDialog),
                dialog_options,
                editor_options,
                on_dialog_press_handler,
                on_dialog_editor_change_handler,
            );

            fn on_dialog_press_handler(dialog_choice: DialogChoice, state: &mut State) {
                match dialog_choice {
                    DialogChoice::Yes(text) => {
                        modal_dialogs::dialog_component_initialize_focused(
                            state,
                            FlexBoxId::from(Id::AutocompleteDialog),
                            "Yes".to_string(),
                            text,
                        );
                    }
                    DialogChoice::No => {
                        modal_dialogs::dialog_component_initialize_focused(
                            state,
                            FlexBoxId::from(Id::AutocompleteDialog),
                            "No".to_string(),
                            "".to_string(),
                        );
                    }
                }
            }

            fn on_dialog_editor_change_handler(state: &mut State) {
                modal_dialogs::dialog_component_update_content(
                    state,
                    FlexBoxId::from(Id::AutocompleteDialog),
                );
            }

            it
        };

        ComponentRegistry::put(
            component_registry_map,
            FlexBoxId::from(Id::AutocompleteDialog),
            boxed_dialog_component,
        );

        call_if_true!(DEBUG_TUI_MOD, {
            let msg = format!(
                "🪙 {}",
                "construct DialogComponent (autocomplete) { on_dialog_press }"
            );
            log_debug(msg);
        });
    }
}

mod stylesheet {
    use super::*;

    pub fn create_stylesheet() -> CommonResult<Stylesheet> {
        throws_with_return!({
            stylesheet! {
              style! {
                id: Id::EditorStyleNameDefault.into()
                padding: 1
                // These are ignored due to syntax highlighting.
                // attrib: [bold]
                // color_fg: TuiColor::Blue
              },
              style! {
                id: Id::DialogStyleNameTitle.into()
                lolcat: true
                // These are ignored due to lolcat: true.
                // attrib: [bold]
                // color_fg: TuiColor::Yellow
              },
              style! {
                id: Id::DialogStyleNameBorder.into()
                lolcat: true
                // These are ignored due to lolcat: true.
                // attrib: [dim]
                // color_fg: TuiColor::Green
              },
              style! {
                id: Id::DialogStyleNameEditor.into()
                attrib: [bold]
                color_fg: TuiColor::Basic(ANSIBasicColor::Magenta)
              },
              style! {
                id: Id::DialogStyleNameResultsPanel.into()
                // attrib: [bold]
                color_fg: TuiColor::Basic(ANSIBasicColor::Blue)
              }
            }
        })
    }
}

mod status_bar {
    use super::*;

    /// Shows helpful messages at the bottom row of the screen.
    pub fn render_status_bar(pipeline: &mut RenderPipeline, size: Size) {
        let styled_texts = styled_texts! {
            styled_text! { @style: style!(attrib: [bold, dim]) ,      @text: "Hints: "},
            styled_text! { @style: style!(attrib: [dim, underline]) , @text: "Ctrl + q"},
            styled_text! { @style: style!(attrib: [bold]) ,           @text: " : Exit 🖖"},
            styled_text! { @style: style!(attrib: [dim]) ,            @text: " … "},
            styled_text! { @style: style!(attrib: [dim, underline]) , @text: "Ctrl + l"},
            styled_text! { @style: style!(attrib: [bold]) ,           @text: " : Simple 📣"},
            styled_text! { @style: style!(attrib: [dim]) ,            @text: " … "},
            styled_text! { @style: style!(attrib: [dim, underline]) , @text: "Ctrl + k"},
            styled_text! { @style: style!(attrib: [bold]) ,           @text: " : Autocomplete 🤖"},
            styled_text! { @style: style!(attrib: [dim]) ,            @text: " … "},
            styled_text! { @style: style!(attrib: [underline]) ,      @text: "Type content 🌊"},
        };

        let display_width = styled_texts.display_width();
        let col_center: ChUnit = (size.col_count - display_width) / 2;
        let row_bottom: ChUnit = size.row_count - 1;
        let center: Position = position!(col_index: col_center, row_index: row_bottom);

        let mut render_ops = render_ops!();
        render_ops.push(RenderOp::MoveCursorPositionAbs(center));
        styled_texts.render_into(&mut render_ops);
        pipeline.push(ZOrder::Normal, render_ops);
    }
}
