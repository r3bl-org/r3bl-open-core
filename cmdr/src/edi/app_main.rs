/*
 *   Copyright (c) 2023 R3BL LLC
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

use r3bl_rs_utils_core::*;
use r3bl_rs_utils_macro::style;
use r3bl_tui::*;

use crate::edi::{AppSignal, State};

/// Constants for the ids.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Id {
    Editor = 1,
    // 00: rename this to something like AskUserForFilenameToSaveFile.
    SimpleDialog = 2,
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
                let msg = format!("🪙 {}", "construct edi::AppMain");
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
    use crate::edi::file_utils;

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
            let GlobalData {
                main_thread_channel_sender,
                ..
            } = global_data;

            // 00: [x] send SaveFile signal here by intercepting keybinding.
            if input_event.matches_keypress(KeyPress::WithModifiers {
                key: Key::Character('s'),
                mask: ModifierKeysMask::new().with_ctrl(),
            }) {
                send_signal!(
                    main_thread_channel_sender,
                    TerminalWindowMainThreadSignal::ApplyAction(AppSignal::SaveFile)
                );
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
            action: &AppSignal,
            global_data: &mut GlobalData<State, AppSignal>,
            component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
            has_focus: &mut HasFocus,
        ) -> CommonResult<EventPropagation> {
            match action {
                AppSignal::SaveFile => {
                    // 00: [x] handle SaveFile signal here.

                    // Save the file using information from state (editor buffer,
                    // filename, etc). If this fails, then return an error.
                    let GlobalData { state, .. } = global_data;

                    let maybe_editor_buffer =
                        state.editor_buffers.get_mut(&FlexBoxId::from(Id::Editor));

                    if let Some(editor_buffer) = maybe_editor_buffer {
                        let maybe_file_path =
                            editor_buffer.editor_content.maybe_file_path.clone();
                        let content: String = editor_buffer.get_as_string_with_newlines();

                        match maybe_file_path {
                            // Found file path in the editor buffer.
                            Some(file_path) => {
                                file_utils::save_content_to_file(file_path, content);
                            }
                            // Could not find file path in the editor buffer. This is a
                            // new buffer. Need to ask user via dialog box.
                            _ => {
                                if !editor_buffer.is_empty() {
                                    send_signal!(
                                        global_data.main_thread_channel_sender,
                                        TerminalWindowMainThreadSignal::ApplyAction(
                                            AppSignal::AskUserForFilenameThenSaveFile
                                        )
                                    );
                                }
                            }
                        }
                    }
                }
                AppSignal::AskUserForFilenameThenSaveFile => {
                    // 00: [_] handle AskUserForFilenameThenSaveFile signal here.
                    let GlobalData { state, .. } = global_data;

                    // Reset the dialog component prior to activating / showing it.
                    ComponentRegistry::reset_component(
                        component_registry_map,
                        FlexBoxId::from(Id::SimpleDialog),
                    );

                    match modal_dialogs::activate_simple_modal(
                        component_registry_map,
                        has_focus,
                        state,
                    ) {
                        Err(err) => {
                            if let Some(CommonError {
                                err_type: _,
                                err_msg: msg,
                            }) = err.downcast_ref::<CommonError>()
                            {
                                log_error(format!(
                                    "📣 Error activating simple modal: {msg:?}"
                                ));
                            }
                        }
                        _ => {}
                    };

                    return Ok(EventPropagation::ConsumedRender);
                }
                AppSignal::Noop => {}
            }

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
            let line: String = {
                if text.is_empty() {
                    "".to_string()
                } else {
                    text.clone()
                }
            };
            it.editor_buffer.set_lines(vec![line]);
            it
        };
        state.dialog_buffers.insert(id, dialog_buffer);
    }

    // 00: rename this function to something like AskUserForFilenameThenSaveFile.
    pub fn activate_simple_modal(
        _component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
        has_focus: &mut HasFocus,
        state: &mut State,
    ) -> CommonResult<()> {
        throws!({
            // Initialize the dialog buffer with title & text.
            // 00: rename this title.
            let title = "Simple Modal Dialog Title";
            let text = "".to_string();

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
            });
        }
    }
}

mod populate_component_registry {
    use crossterm::style::Stylize;
    use tokio::sync::mpsc::Sender;

    use super::*;
    use crate::edi::file_utils;

    pub fn create_components(
        component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
        has_focus: &mut HasFocus,
    ) {
        insert_editor_component(component_registry_map);
        insert_dialog_component_simple(component_registry_map);

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

            fn on_dialog_press_handler(
                dialog_choice: DialogChoice,
                state: &mut State,
                main_thread_channel_sender: &mut Sender<
                    TerminalWindowMainThreadSignal<AppSignal>,
                >,
            ) {
                match dialog_choice {
                    DialogChoice::Yes(text) => {
                        modal_dialogs::dialog_component_initialize_focused(
                            state,
                            FlexBoxId::from(Id::SimpleDialog),
                            "Yes".to_string(),
                            text.clone(),
                        );

                        // 00: [🔥] Save the file here!
                        let user_input_file_path = text.trim().to_string();
                        if user_input_file_path != "" {
                            call_if_true!(DEBUG_TUI_MOD, {
                                let msg = format!("\n💾💾💾 About to save the new buffer with given filename: {user_input_file_path:?}")
                                    .magenta()
                                    .to_string();
                                log_debug(msg);
                            });

                            let maybe_editor_buffer =
                                state.get_mut_editor_buffer(FlexBoxId::from(Id::Editor));

                            match maybe_editor_buffer {
                                Some(editor_buffer) => {
                                    // Set the file path.
                                    editor_buffer.editor_content.maybe_file_path =
                                        Some(user_input_file_path.clone());

                                    // Set the file extension.
                                    editor_buffer.editor_content.maybe_file_extension =
                                        Some(file_utils::get_file_extension(&Some(
                                            user_input_file_path.clone(),
                                        )));

                                    // 00: Review this code and remove it.
                                    /*
                                    // Get the content from the new editor buffer.
                                    let content =
                                        editor_buffer.get_as_string_with_newlines();

                                    call_if_true!(DEBUG_TUI_MOD, {
                                        let msg = format!(
                                            "\n💾💾💾 About to save this content: \n{:?}",
                                            editor_buffer.get_as_string_with_newlines()
                                        )
                                        .magenta()
                                        .to_string();
                                        log_debug(msg);
                                    });

                                    // Actually save the file.
                                    file_utils::save_content_to_file(
                                        user_input_file_path.clone(),
                                        content,
                                    );
                                    */

                                    // 00: Route a GlobalData here, to access main_thread_channel_sender
                                    // Should be able to just fire a signal to save the file.
                                    send_signal!(
                                        main_thread_channel_sender,
                                        TerminalWindowMainThreadSignal::ApplyAction(
                                            AppSignal::SaveFile
                                        )
                                    );
                                }
                                _ => {}
                            }
                        }
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

            fn on_dialog_editor_change_handler(
                _state: &mut State,
                _main_thread_channel_sender: &mut Sender<
                    TerminalWindowMainThreadSignal<AppSignal>,
                >,
            ) {
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
        let separator_style = style!(
            attrib: [dim]
            color_fg: TuiColor::Basic(ANSIBasicColor::DarkGrey)
        );

        let app_text = &UnicodeString::from("edi 🦜 ✶early access✶");

        let mut color_wheel = ColorWheel::new(vec![
            ColorWheelConfig::Rgb(
                Vec::from(["#3eff03", "#00e5ff"].map(String::from)),
                ColorWheelSpeed::Fast,
                15,
            ),
            ColorWheelConfig::Ansi256(
                Ansi256GradientIndex::MediumGreenToMediumBlue,
                ColorWheelSpeed::Fast,
            ),
        ]);

        let app_text_styled_texts = color_wheel.colorize_into_styled_texts(
            app_text,
            GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
            TextColorizationPolicy::ColorEachCharacter(None),
        );

        let styled_texts: StyledTexts = {
            let mut it = Default::default();
            it += app_text_styled_texts;
            it += styled_text! { @style: separator_style , @text: " │ "};
            it += styled_text! { @style: style!(attrib: [dim]) , @text: "Save: Ctrl+S "};
            it += styled_text! { @style: style!() , @text: "💾"};
            it += styled_text! { @style: separator_style , @text: " │ "};
            it += styled_text! { @style: style!(attrib: [dim]) , @text: "Exit: Ctrl+Q "};
            it += styled_text! { @style: style!() , @text: "🖖"};
            it
        };

        // 00: [x] show keybinding for save.
        // 00: [_] remove commented code.
        // let styled_texts = styled_texts! {
        // styled_text! { @style: edi_style,                    @text: "edi 🦜" },
        // styled_text! { @style: early_access_style ,          @text: " ★early access★"},
        // styled_text! { @style: separator_style ,             @text: " │ "},
        // styled_text! { @style: style!(attrib: [dim]) ,       @text: "Save: Ctrl+S "},
        // styled_text! { @style: style!() ,                    @text: "💾"},
        // styled_text! { @style: separator_style ,             @text: " │ "},
        // styled_text! { @style: style!(attrib: [dim]) ,       @text: "Exit: Ctrl+Q "},
        // styled_text! { @style: style!() ,                    @text: "🖖"},
        // styled_text! { @style: separator_style ,             @text: " │ "},
        // styled_text! { @style: style!(attrib: [dim]) ,       @text: "Ctrl+l: Simple "},
        // styled_text! { @style: style!() ,                    @text: "📣"},
        // styled_text! { @style: separator_style ,             @text: " │ "},
        // styled_text! { @style: style!(attrib: [dim]) ,       @text: "Ctrl+k: Auto "},
        // styled_text! { @style: style!() ,                    @text: "🤖"},
        // };

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
