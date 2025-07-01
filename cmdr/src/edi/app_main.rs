/*
 *   Copyright (c) 2023-2025 R3BL LLC
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

use r3bl_tui::{Ansi256GradientIndex,
               App,
               BoxedSafeApp,
               ColorWheel,
               ColorWheelConfig,
               ColorWheelSpeed,
               ComponentRegistry,
               ComponentRegistryMap,
               DEBUG_TUI_MOD,
               DialogBuffer,
               DialogChoice,
               DialogComponent,
               DialogEngineConfigOptions,
               DialogEngineMode,
               EditMode,
               EditorComponent,
               EditorEngineConfig,
               EventPropagation,
               FlexBox,
               FlexBoxId,
               GlobalData,
               HasEditorBuffers,
               HasFocus,
               InputEvent,
               Key,
               KeyPress,
               LayoutDirection,
               LayoutManagement,
               LineMode,
               ModifierKeysMask,
               PerformPositioningAndSizing,
               RenderOp,
               RenderPipeline,
               Surface,
               SurfaceProps,
               SurfaceRender,
               SyntaxHighlightMode,
               TerminalWindowMainThreadSignal,
               ZOrder,
               box_end,
               box_start,
               col,
               fg_green,
               fg_magenta,
               fg_red,
               glyphs,
               height,
               inline_string,
               new_style,
               render_component_in_current_box,
               render_component_in_given_box,
               render_ops,
               render_tui_styled_texts_into,
               req_size_pc,
               row,
               surface,
               tui_color,
               tui_stylesheet};
use smallvec::smallvec;
use tokio::sync::mpsc::Sender;

use crate::edi::State;

/// Signals that can be sent to the app.
#[derive(Default, Clone, Debug)]
#[non_exhaustive]
pub enum AppSignal {
    AskForFilenameToSaveFile,
    SaveFile,
    #[default]
    Noop,
}

/// Constants for the ids.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Id {
    // Components.
    ComponentEditor = 1,
    ComponentSimpleDialogAskForFilenameToSaveFile = 2,

    // Styles.
    StyleEditorDefault = 10,
    StyleDialogBorder = 11,
    StyleDialogTitle = 12,
    StyleDialogEditor = 13,
    StyleDialogResultsPanel = 14,
}

mod id_impl {
    use super::*;

    impl From<Id> for u8 {
        fn from(id: Id) -> u8 { id as u8 }
    }

    impl From<Id> for FlexBoxId {
        fn from(id: Id) -> FlexBoxId { FlexBoxId::new(id) }
    }
}

/// The main app struct.
pub struct AppMain;

mod app_main_constructor {
    use super::*;

    impl Default for AppMain {
        fn default() -> Self {
            DEBUG_TUI_MOD.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(message = "🪙 construct edi::AppMain");
            });
            Self
        }
    }

    impl AppMain {
        /// Note that this needs to be initialized before it can be used.
        #[must_use]
        pub fn new_boxed() -> BoxedSafeApp<State, AppSignal> {
            let it = Self;
            Box::new(it)
        }
    }
}

mod app_main_impl_app_trait {
    use r3bl_tui::{CommonError, CommonResult, send_signal, throws_with_return};

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
            // Handle Ctrl + s.
            if input_event.matches_keypress(KeyPress::WithModifiers {
                key: Key::Character('s'),
                mask: ModifierKeysMask::new().with_ctrl(),
            }) {
                send_signal!(
                    global_data.main_thread_channel_sender,
                    TerminalWindowMainThreadSignal::ApplyAppSignal(AppSignal::SaveFile)
                );

                return Ok(EventPropagation::Consumed);
            }

            // Handle Ctrl + k.
            if input_event.matches_keypress(KeyPress::WithModifiers {
                key: Key::Character('k'),
                mask: ModifierKeysMask::new().with_ctrl(),
            }) {
                let link_url =
                    "https://github.com/r3bl-org/r3bl-open-core/issues/new/choose";
                let result_open = open::that(link_url);
                match result_open {
                    Ok(()) => {
                        DEBUG_TUI_MOD.then(|| {
                            // % is Display, ? is Debug.
                            tracing::debug!(
                                message = "📣 Opened feedback link",
                                link = %fg_green(&inline_string!("{link_url:?}"))
                            );
                        });
                    }
                    Err(err) => {
                        // % is Display, ? is Debug.
                        tracing::error!(
                            message = "📣 Error opening feedback link",
                            error = %fg_red(&inline_string!("{err:?}"))
                        );
                    }
                }

                return Ok(EventPropagation::Consumed);
            }

            // If modal not activated, route the input event to the focused component.
            ComponentRegistry::route_event_to_focused_component(
                global_data,
                input_event,
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
                    // Save the file using information from state (editor buffer,
                    // filename, etc).
                    let GlobalData { state, .. } = global_data;

                    let maybe_editor_buffer = state
                        .editor_buffers
                        .get_mut(&FlexBoxId::from(Id::ComponentEditor));

                    if let Some(editor_buffer) = maybe_editor_buffer {
                        let maybe_file_path =
                            editor_buffer.content.maybe_file_path.clone();
                        let content = editor_buffer.get_as_string_with_newlines();

                        match maybe_file_path {
                            // Found file path in the editor buffer.
                            Some(file_path) => {
                                file_utils::save_content_to_file(&file_path, &content);
                            }
                            // Could not find file path in the editor buffer. This is a
                            // new buffer. Need to ask user via dialog box.
                            _ => {
                                if !editor_buffer.is_empty() {
                                    send_signal!(
                                        global_data.main_thread_channel_sender,
                                        TerminalWindowMainThreadSignal::ApplyAppSignal(
                                            AppSignal::AskForFilenameToSaveFile
                                        )
                                    );
                                }
                            }
                        }
                    }
                }
                AppSignal::AskForFilenameToSaveFile => {
                    let GlobalData { state, .. } = global_data;

                    // Reset the dialog component prior to activating / showing it.
                    ComponentRegistry::reset_component(
                        component_registry_map,
                        FlexBoxId::from(
                            Id::ComponentSimpleDialogAskForFilenameToSaveFile,
                        ),
                    );

                    if let Err(err) = modal_dialog_ask_for_filename_to_save_file::show(
                        component_registry_map,
                        has_focus,
                        state,
                    ) {
                        match err.downcast_ref::<CommonError>() {
                            // err is of concrete type CommonError.
                            Some(common_error) => {
                                // % is Display, ? is Debug.
                                tracing::error!(
                                    message = "📣 Error activating simple modal",
                                    error = ?common_error
                                );
                            }
                            // err is not of concrete type CommonError.
                            _ => { /* do nothing */ }
                        }
                    }

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

                // Create a surface and then run the SurfaceRenderer
                // (ContainerSurfaceRender) on it.
                let mut surface = {
                    let mut it = surface!(stylesheet: stylesheet::create_stylesheet()?);

                    it.surface_start(SurfaceProps {
                        pos: row(0) + col(0),
                        size: window_size.col_width
                            + (window_size.row_height - height(1)), // Bottom row for for status bar.
                    })?;

                    perform_layout::ContainerSurfaceRender { _app: self }
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

mod modal_dialog_ask_for_filename_to_save_file {
    use r3bl_tui::{CommonResult, InlineString, get_tui_style, send_signal, throws};

    use super::*;
    use crate::edi::file_utils;

    pub fn initialize(
        state: &mut State,
        id: FlexBoxId,
        title: InlineString,
        text: InlineString,
    ) {
        let new_dialog_buffer = {
            let mut it = DialogBuffer::new_empty();
            it.title = title;
            it.editor_buffer.set_lines(text.lines());
            it
        };
        state.dialog_buffers.insert(id, new_dialog_buffer);
    }

    pub fn show(
        _component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
        has_focus: &mut HasFocus,
        state: &mut State,
    ) -> CommonResult<()> {
        throws!({
            // Initialize the dialog buffer with title & text.
            let title = "File name or path to save content to:";
            let text = "";

            // Setting the has_focus to Id::ComponentSimpleDialogAskForFilenameToSaveFile
            // will cause the dialog to appear on the next render.
            has_focus.try_set_modal_id(FlexBoxId::from(
                Id::ComponentSimpleDialogAskForFilenameToSaveFile,
            ))?;

            // Change the state so that it will trigger a render. This will show the title
            // & text on the next render.
            initialize(
                state,
                FlexBoxId::from(Id::ComponentSimpleDialogAskForFilenameToSaveFile),
                title.into(),
                text.into(),
            );

            DEBUG_TUI_MOD.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "📣 activate modal simple",
                    has_focus = ?has_focus
                );
            });
        });
    }

    /// Insert simple dialog component into registry if it's not already there.
    pub fn insert_component_into_registry(
        component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
    ) {
        let result_stylesheet = stylesheet::create_stylesheet();

        let dialog_options = DialogEngineConfigOptions {
            mode: DialogEngineMode::ModalSimple,
            maybe_style_border: get_tui_style! { @from_result: result_stylesheet , Id::StyleDialogBorder },
            maybe_style_title: get_tui_style! { @from_result: result_stylesheet , Id::StyleDialogTitle },
            maybe_style_editor: get_tui_style! { @from_result: result_stylesheet , Id::StyleDialogEditor },
            maybe_style_results_panel: get_tui_style! { @from_result: result_stylesheet , Id::StyleDialogResultsPanel },
            ..Default::default()
        };

        let editor_options = EditorEngineConfig {
            multiline_mode: LineMode::SingleLine,
            syntax_highlight: SyntaxHighlightMode::Disable,
            edit_mode: EditMode::ReadWrite,
        };

        let boxed_dialog_component = {
            fn on_dialog_press_handler(
                dialog_choice: DialogChoice,
                state: &mut State,
                main_thread_channel_sender: &mut Sender<
                    TerminalWindowMainThreadSignal<AppSignal>,
                >,
            ) {
                match dialog_choice {
                    DialogChoice::Yes(text) => {
                        modal_dialog_ask_for_filename_to_save_file::initialize(
                            state,
                            FlexBoxId::from(
                                Id::ComponentSimpleDialogAskForFilenameToSaveFile,
                            ),
                            "Yes".into(),
                            text.clone(),
                        );

                        let user_input_file_path = text.trim();
                        if !user_input_file_path.is_empty() {
                            DEBUG_TUI_MOD.then(|| {
                                // % is Display, ? is Debug.
                                tracing::debug!(
                                    message = "💾💾💾 About to save the new buffer with given filename",
                                    file_path = %fg_magenta(&inline_string!("{user_input_file_path}"))
                                );
                            });

                            let maybe_editor_buffer = state.get_mut_editor_buffer(
                                FlexBoxId::from(Id::ComponentEditor),
                            );

                            if let Some(editor_buffer) = maybe_editor_buffer {
                                // Set the file path.
                                editor_buffer.content.maybe_file_path =
                                    Some(user_input_file_path.into());

                                // Set the file extension.

                                editor_buffer.content.maybe_file_extension = {
                                    let one = Some(user_input_file_path);
                                    let two = file_utils::get_file_extension(one);
                                    Some(two)
                                };

                                // Fire a signal to save the file.
                                send_signal!(
                                    main_thread_channel_sender,
                                    TerminalWindowMainThreadSignal::ApplyAppSignal(
                                        AppSignal::SaveFile
                                    )
                                );
                            }
                        }
                    }
                    DialogChoice::No => {
                        modal_dialog_ask_for_filename_to_save_file::initialize(
                            state,
                            FlexBoxId::from(
                                Id::ComponentSimpleDialogAskForFilenameToSaveFile,
                            ),
                            "No".into(),
                            "".into(),
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

            DialogComponent::new_boxed(
                FlexBoxId::from(Id::ComponentSimpleDialogAskForFilenameToSaveFile),
                dialog_options,
                editor_options,
                on_dialog_press_handler,
                on_dialog_editor_change_handler,
            )
        };

        ComponentRegistry::put(
            component_registry_map,
            FlexBoxId::from(Id::ComponentSimpleDialogAskForFilenameToSaveFile),
            boxed_dialog_component,
        );

        DEBUG_TUI_MOD.then(|| {
            // % is Display, ? is Debug.
            tracing::debug!(
                message =
                    "app_main construct DialogComponent (simple) [ on_dialog_press ]"
            );
        });
    }
}

mod perform_layout {
    use r3bl_tui::{CommonResult, throws};

    use super::*;

    pub struct ContainerSurfaceRender<'a> {
        pub _app: &'a mut AppMain,
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
                        id:                     FlexBoxId::from(Id::ComponentEditor),
                        dir:                    LayoutDirection::Vertical,
                        requested_size_percent: req_size_pc!(width: 100, height: 100),
                        styles:                 [Id::StyleEditorDefault]
                    );
                    render_component_in_current_box!(
                        in:                 surface,
                        component_id:       FlexBoxId::from(Id::ComponentEditor),
                        from:               component_registry_map,
                        global_data:        global_data,
                        has_focus:          has_focus
                    );
                    box_end!(in: surface);
                }

                // Then, render simple modal dialog (if it is active, on top of the editor
                // component).
                if has_focus.is_modal_id(FlexBoxId::from(
                    Id::ComponentSimpleDialogAskForFilenameToSaveFile,
                )) {
                    render_component_in_given_box! {
                      in:                 surface,
                      box:                FlexBox::default(), /* This is not used as the modal breaks out of its box. */
                      component_id:       FlexBoxId::from(Id::ComponentSimpleDialogAskForFilenameToSaveFile),
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
    use r3bl_tui::send_signal;

    use super::*;

    pub fn create_components(
        component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
        has_focus: &mut HasFocus,
    ) {
        insert_editor_component(component_registry_map);
        modal_dialog_ask_for_filename_to_save_file::insert_component_into_registry(
            component_registry_map,
        );

        // Switch focus to the editor component if focus is not set.
        let id = FlexBoxId::from(Id::ComponentEditor);
        has_focus.set_id(id);

        DEBUG_TUI_MOD.then(|| {
            // % is Display, ? is Debug.
            tracing::info!(
                message = %inline_string!("app_main init has_focus {ch}", ch = glyphs::FOCUS_GLYPH),
                has_focus = ?has_focus.get_id()
            );
        });
    }

    /// Insert editor component into registry if it's not already there.
    fn insert_editor_component(
        component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
    ) {
        let id = FlexBoxId::from(Id::ComponentEditor);
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

        DEBUG_TUI_MOD.then(|| {
            // % is Display, ? is Debug.
            tracing::debug!(
                message = "app_main construct EditorComponent [ on_buffer_change ]"
            );
        });
    }
}

mod stylesheet {
    use r3bl_tui::{CommonResult, TuiStylesheet, throws_with_return};

    use super::*;

    pub fn create_stylesheet() -> CommonResult<TuiStylesheet> {
        throws_with_return!({
            tui_stylesheet! {
                new_style!(
                    id: {Id::StyleEditorDefault}
                    padding: {1}
                    // These are ignored due to syntax highlighting.
                    // attrib: [bold]
                    // color_fg: TuiColor::Blue
                ),
                new_style!(
                    id: {Id::StyleDialogTitle}
                    lolcat
                    // These are ignored due to lolcat: true.
                    // attrib: [bold]
                    // color_fg: TuiColor::Yellow
                ),
                new_style!(
                    id: {Id::StyleDialogBorder}
                    lolcat
                    // These are ignored due to lolcat: true.
                    // attrib: [dim]
                    // color_fg: TuiColor::Green
                ),
                new_style!(
                    id: {Id::StyleDialogEditor}
                    bold
                    color_fg: {tui_color!(magenta)}
                ),
                new_style!(
                    id: {Id::StyleDialogResultsPanel}
                    // bold
                    color_fg: {tui_color!(blue)}
                )
            }
        })
    }
}

mod status_bar {
    use r3bl_tui::{GCStringExt,
                   GradientGenerationPolicy,
                   Size,
                   TextColorizationPolicy,
                   TuiStyledTexts,
                   col,
                   new_style,
                   tui_styled_text};

    use super::*;

    /// Shows helpful messages at the bottom row of the screen.
    pub fn render_status_bar(pipeline: &mut RenderPipeline, size: Size) {
        let separator_style = new_style!(dim color_fg: {tui_color!(dark_gray)});

        let app_text = "edi 🦜 ✶early access✶";

        let mut color_wheel = ColorWheel::new(smallvec![
            ColorWheelConfig::Rgb(
                smallvec!["#3eff03".into(), "#00e5ff".into()],
                ColorWheelSpeed::Fast,
                15,
            ),
            ColorWheelConfig::Ansi256(
                Ansi256GradientIndex::MediumGreenToMediumBlue,
                ColorWheelSpeed::Fast,
            ),
        ]);

        let app_text_gcs = app_text.grapheme_string();
        let app_text_styled_texts = color_wheel.colorize_into_styled_texts(
            &app_text_gcs,
            GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
            TextColorizationPolicy::ColorEachCharacter(None),
        );

        let styled_texts: TuiStyledTexts = {
            let mut it = TuiStyledTexts::default();
            it += app_text_styled_texts;
            it += tui_styled_text! { @style: separator_style , @text: " │ "};
            it += tui_styled_text! { @style: new_style!(dim) , @text: "Save: Ctrl+S "};
            it += tui_styled_text! { @style: new_style!() , @text: "💾"};
            it += tui_styled_text! { @style: separator_style , @text: " │ "};
            it +=
                tui_styled_text! { @style: new_style!(dim) , @text: "Feedback: Ctrl+K "};
            it += tui_styled_text! { @style: new_style!() , @text: "💭"};
            it += tui_styled_text! { @style: separator_style , @text: " │ "};
            it += tui_styled_text! { @style: new_style!(dim) , @text: "Exit: Ctrl+Q "};
            it += tui_styled_text! { @style: new_style!() , @text: "🖖"};
            it
        };

        let display_width = styled_texts.display_width();
        let col_center = *(size.col_width - display_width) / 2;
        let row_bottom = size.row_height.convert_to_row_index();
        let center = col(col_center) + row_bottom;

        let mut render_ops = render_ops!();
        render_ops.push(RenderOp::MoveCursorPositionAbs(center));
        render_tui_styled_texts_into(&styled_texts, &mut render_ops);
        pipeline.push(ZOrder::Normal, render_ops);
    }
}
