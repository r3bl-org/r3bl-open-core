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

use std::fmt::Debug;

use async_trait::async_trait;
use r3bl_redux::*;
use r3bl_rs_utils_core::*;
use r3bl_rs_utils_macro::style;
use r3bl_tui::*;

use super::*;

/// Constants for the ids.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComponentId {
    Editor = 1,
    SimpleDialog = 2,
    AutocompleteDialog = 3,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EditorStyleName {
    Default = 4,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DialogStyleName {
    Border = 5,
    Title = 6,
    Editor = 7,
    ResultsPanel = 8,
}

/// Async trait object that implements the [App] trait.
pub struct AppWithLayout {
    pub component_registry: ComponentRegistry<State, Action>,
}

mod app_with_layout_impl_trait_app {
    use super::*;

    #[async_trait]
    impl App<State, Action> for AppWithLayout {
        async fn app_render(
            &mut self,
            args: GlobalScopeArgs<'_, State, Action>,
        ) -> CommonResult<RenderPipeline> {
            throws_with_return!({
                let GlobalScopeArgs {
                    state,
                    shared_store,
                    shared_global_data,
                    window_size,
                } = args;

                // Create a surface and then run the SurfaceRenderer (ContainerSurfaceRender) on it.
                let mut surface = {
                    let mut it = surface!(stylesheet: stylesheet::create_stylesheet()?);

                    it.surface_start(SurfaceProps {
                        pos: position!(col_index: 0, row_index: 0),
                        size: size!(
                            col_count: window_size.col_count,
                            row_count: window_size.row_count - 1), // Bottom row for for status bar.
                    })?;

                    perform_layout::ContainerSurfaceRender(self)
                        .render_in_surface(
                            GlobalScopeArgs {
                                shared_global_data,
                                shared_store,
                                state,
                                window_size,
                            },
                            &mut it,
                        )
                        .await?;

                    it.surface_end()?;

                    it
                };

                // Render status bar.
                status_bar::render_status_bar(&mut surface.render_pipeline, window_size);

                // Return RenderOps pipeline (which will actually be painted elsewhere).
                surface.render_pipeline
            });
        }

        async fn app_handle_event(
            &mut self,
            args: GlobalScopeArgs<'_, State, Action>,
            input_event: &InputEvent,
        ) -> CommonResult<EventPropagation> {
            let GlobalScopeArgs {
                state,
                shared_store,
                shared_global_data,
                window_size,
            } = args;

            call_if_true!(DEBUG_TUI_MOD, {
                let msg = format!("🐝 focus: {:?}", self.component_registry.has_focus);
                log_debug(msg);
            });

            // Check to see if the modal dialog should be activated.
            if let EventPropagation::Consumed =
                self.try_input_event_activate_modal(args, input_event).await
            {
                call_if_true!(DEBUG_TUI_MOD, {
                    let msg = format!(
                        "🐝 focus move to modal: {:?}",
                        self.component_registry.has_focus
                    );
                    log_debug(msg);
                });
                return Ok(EventPropagation::Consumed);
            }

            // If modal not activated, route the input event to the focused component.
            ComponentRegistry::route_event_to_focused_component(
                &mut self.component_registry,
                input_event,
                state,
                shared_store,
                shared_global_data,
                window_size,
            )
            .await
        }

        fn init(&mut self) { populate_component_registry::init(self); }

        fn get_component_registry(&mut self) -> &mut ComponentRegistry<State, Action> {
            &mut self.component_registry
        }
    }
}

mod detect_modal_dialog_activation_from_input_event {
    use super::*;

    impl AppWithLayout {
        /// If `input_event` matches <kbd>Ctrl+l</kbd> or <kbd>Ctrl+k</kbd>, then toggle the modal
        /// dialog.
        ///
        /// Note that this returns a [EventPropagation::Consumed] and not
        /// [EventPropagation::ConsumedRender] because the [Action::SetDialogBufferTitleAndTextById]
        /// is dispatched to the store & that will cause a rerender.
        pub async fn try_input_event_activate_modal(
            &mut self,
            args: GlobalScopeArgs<'_, State, Action>,
            input_event: &InputEvent,
        ) -> EventPropagation {
            // Ctrl + l => activate Simple.
            if let DialogEvent::ActivateModal = DialogEvent::should_activate_modal(
                input_event,
                KeyPress::WithModifiers {
                    key: Key::Character('l'),
                    mask: ModifierKeysMask::CTRL,
                },
            ) {
                // Reset the dialog component prior to activating / showing it.
                ComponentRegistry::reset_component(
                    &mut self.component_registry,
                    ComponentId::SimpleDialog as u8,
                )
                .await;
                return match activate_simple_modal(self, args) {
                    Ok(_) => EventPropagation::Consumed,
                    Err(err) => {
                        if let Some(CommonError {
                            err_type: _,
                            err_msg: msg,
                        }) = err.downcast_ref::<CommonError>()
                        {
                            log_error(format!("📣 Error activating simple modal: {msg:?}"));
                        }
                        EventPropagation::Propagate
                    }
                };
            };

            // Ctrl + k => activate Autocomplete.
            if let DialogEvent::ActivateModal = DialogEvent::should_activate_modal(
                input_event,
                KeyPress::WithModifiers {
                    key: Key::Character('k'),
                    mask: ModifierKeysMask::CTRL,
                },
            ) {
                // Reset the dialog component prior to activating / showing it.
                ComponentRegistry::reset_component(
                    &mut self.component_registry,
                    ComponentId::AutocompleteDialog as u8,
                )
                .await;
                return match activate_autocomplete_modal(self, args) {
                    Ok(_) => EventPropagation::Consumed,
                    Err(err) => {
                        if let Some(CommonError {
                            err_type: _,
                            err_msg: msg,
                        }) = err.downcast_ref::<CommonError>()
                        {
                            log_error(format!("📣 Error activating autocomplete modal: {msg:?}"));
                        }
                        EventPropagation::Propagate
                    }
                };
            };

            return EventPropagation::Propagate;

            fn activate_simple_modal(
                this: &mut AppWithLayout,
                args: GlobalScopeArgs<State, Action>,
            ) -> CommonResult<()> {
                // Initialize the dialog buffer with title & text.
                let title = "Simple Modal Dialog Title";
                let text = {
                    if let Some(editor_buffer) =
                        args.state.get_editor_buffer(ComponentId::Editor as u8)
                    {
                        editor_buffer.get_as_string()
                    } else {
                        "".to_string()
                    }
                };

                // Setting the has_focus to Id::Dialog will cause the dialog to appear on the next
                // render.
                this.component_registry
                    .has_focus
                    .try_set_modal_id(ComponentId::SimpleDialog as u8)?;

                // Change the state so that it will trigger a render. This will show the title &
                // text on the next render.
                spawn_dispatch_action!(
                    args.shared_store,
                    Action::SimpleDialogComponentInitializeFocused(
                        ComponentId::SimpleDialog as u8,
                        title.to_string(),
                        text.to_string()
                    )
                );

                call_if_true!(DEBUG_TUI_MOD, {
                    let msg = format!(
                        "📣 activate modal simple: {:?}",
                        this.component_registry.has_focus
                    );
                    log_debug(msg);
                });

                Ok(())
            }

            fn activate_autocomplete_modal(
                this: &mut AppWithLayout,
                args: GlobalScopeArgs<State, Action>,
            ) -> CommonResult<()> {
                // Initialize the dialog buffer with title & text.
                let title = "Autocomplete Modal Dialog Title";
                let text = {
                    if let Some(editor_buffer) =
                        args.state.get_editor_buffer(ComponentId::Editor as u8)
                    {
                        editor_buffer.get_as_string()
                    } else {
                        "".to_string()
                    }
                };

                // Setting the has_focus to Id::Dialog will cause the dialog to appear on the next
                // render.
                this.component_registry
                    .has_focus
                    .try_set_modal_id(ComponentId::AutocompleteDialog as u8)?;

                // Change the state so that it will trigger a render. This will show the title &
                // text on the next render.
                spawn_dispatch_action!(
                    args.shared_store,
                    Action::AutocompleteDialogComponentInitializeFocused(
                        ComponentId::AutocompleteDialog as u8,
                        title.to_string(),
                        text.to_string()
                    )
                );

                call_if_true!(DEBUG_TUI_MOD, {
                    let msg = format!(
                        "📣 activate modal autocomplete: {:?}",
                        this.component_registry.has_focus
                    );
                    log_debug(msg);
                });

                Ok(())
            }
        }
    }
}

mod perform_layout {
    use super::*;

    pub struct ContainerSurfaceRender<'a>(pub &'a mut AppWithLayout);

    #[async_trait]
    impl SurfaceRender<State, Action> for ContainerSurfaceRender<'_> {
        async fn render_in_surface(
            &mut self,
            args: GlobalScopeArgs<'_, State, Action>,
            surface: &mut Surface,
        ) -> CommonResult<()> {
            throws!({
                let GlobalScopeArgs {
                    state,
                    shared_store,
                    shared_global_data,
                    window_size,
                } = args;

                // Layout editor component, and render it.
                {
                    box_start! (
                        in:                     surface,
                        id:                     ComponentId::Editor as u8,
                        dir:                    LayoutDirection::Vertical,
                        requested_size_percent: requested_size_percent!(width: 100, height: 100),
                        styles:                 [EditorStyleName::Default as u8]
                    );
                    render_component_in_current_box!(
                        in:                 surface,
                        component_id:       ComponentId::Editor as u8,
                        from:               self.0.component_registry,
                        state:              state,
                        shared_store:       shared_store,
                        shared_global_data: shared_global_data,
                        window_size:        window_size
                    );
                    box_end!(in: surface);
                }

                // Then, render simple modal dialog (if it is active, on top of the editor
                // component).
                if self
                    .0
                    .component_registry
                    .has_focus
                    .is_modal_id(ComponentId::SimpleDialog as u8)
                {
                    render_component_in_given_box! {
                      in:                 surface,
                      box:                FlexBox::default(), /* This is not used as the modal breaks out of its box. */
                      component_id:       ComponentId::SimpleDialog as u8,
                      from:               self.0.component_registry,
                      state:              state,
                      shared_store:       shared_store,
                      shared_global_data: shared_global_data,
                      window_size:        window_size
                    };
                }

                // Or, render autocomplete modal dialog (if it is active, on top of the editor
                // component).
                if self
                    .0
                    .component_registry
                    .has_focus
                    .is_modal_id(ComponentId::AutocompleteDialog as u8)
                {
                    render_component_in_given_box! {
                      in:                 surface,
                      box:                FlexBox::default(), /* This is not used as the modal breaks out of its box. */
                      component_id:       ComponentId::AutocompleteDialog as u8,
                      from:               self.0.component_registry,
                      state:              state,
                      shared_store:       shared_store,
                      shared_global_data: shared_global_data,
                      window_size:        window_size
                    };
                }
            });
        }
    }
}

mod populate_component_registry {

    use super::*;
    use crate::ex_editor::app::stylesheet::create_stylesheet;

    pub fn init(this: &mut AppWithLayout) {
        insert_editor_component(this);
        insert_dialog_component_simple(this);
        insert_dialog_component_autocomplete(this);

        // Switch focus to the editor component if focus is not set.
        this.component_registry
            .has_focus
            .set_id(ComponentId::Editor as u8);
        call_if_true!(DEBUG_TUI_MOD, {
            {
                let msg = format!(
                    "🪙 {} = {:?}",
                    "init component_registry.has_focus",
                    this.component_registry.has_focus.get_id()
                );
                log_debug(msg);
            }
        });
    }

    /// Insert autocomplete dialog component into registry if it's not already there.
    fn insert_dialog_component_autocomplete(this: &mut AppWithLayout) {
        let result_stylesheet = create_stylesheet();

        let dialog_options = DialogEngineConfigOptions {
            mode: DialogEngineMode::ModalAutocomplete,
            maybe_style_border: get_style! { @from_result: result_stylesheet , DialogStyleName::Border as u8 },
            maybe_style_title: get_style! { @from_result: result_stylesheet , DialogStyleName::Title as u8 },
            maybe_style_editor: get_style! { @from_result: result_stylesheet , DialogStyleName::Editor as u8 },
            maybe_style_results_panel: get_style! { @from_result: result_stylesheet , DialogStyleName::ResultsPanel as u8 },
            ..Default::default()
        };

        let editor_options = EditorEngineConfigOptions {
            multiline_mode: EditorLineMode::SingleLine,
            syntax_highlight: SyntaxHighlightConfig::Disable,
        };

        let shared_dialog_component = {
            let it = DialogComponent::new_shared(
                ComponentId::AutocompleteDialog as u8,
                dialog_options,
                editor_options,
                on_dialog_press_handler,
                on_dialog_editor_change_handler,
            );

            fn on_dialog_press_handler(
                dialog_choice: DialogChoice,
                shared_store: &SharedStore<State, Action>,
            ) {
                match dialog_choice {
                    DialogChoice::Yes(text) => {
                        spawn_dispatch_action!(
                            shared_store,
                            Action::AutocompleteDialogComponentInitializeFocused(
                                ComponentId::AutocompleteDialog as u8,
                                "Yes".to_string(),
                                text
                            )
                        );
                    }
                    DialogChoice::No => {
                        spawn_dispatch_action!(
                            shared_store,
                            Action::AutocompleteDialogComponentInitializeFocused(
                                ComponentId::AutocompleteDialog as u8,
                                "No".to_string(),
                                "".to_string()
                            )
                        );
                    }
                }
            }

            fn on_dialog_editor_change_handler(
                editor_buffer: EditorBuffer,
                shared_store: &SharedStore<State, Action>,
            ) {
                spawn_dispatch_action!(
                    shared_store,
                    Action::AutocompleteDialogComponentUpdateContent(
                        ComponentId::AutocompleteDialog as u8,
                        editor_buffer
                    )
                );
            }

            it
        };

        this.component_registry.put(
            ComponentId::AutocompleteDialog as u8,
            shared_dialog_component,
        );

        call_if_true!(DEBUG_TUI_MOD, {
            let msg = format!(
                "🪙 {}",
                "construct DialogComponent (autocomplete) { on_dialog_press }"
            );
            log_debug(msg);
        });
    }

    /// Insert simple dialog component into registry if it's not already there.
    fn insert_dialog_component_simple(this: &mut AppWithLayout) {
        let result_stylesheet = create_stylesheet();

        let dialog_options = DialogEngineConfigOptions {
            mode: DialogEngineMode::ModalSimple,
            maybe_style_border: get_style! { @from_result: result_stylesheet , DialogStyleName::Border as u8 },
            maybe_style_title: get_style! { @from_result: result_stylesheet , DialogStyleName::Title as u8 },
            maybe_style_editor: get_style! { @from_result: result_stylesheet , DialogStyleName::Editor as u8 },
            maybe_style_results_panel: get_style! { @from_result: result_stylesheet , DialogStyleName::ResultsPanel as u8 },
            ..Default::default()
        };

        let editor_options = EditorEngineConfigOptions {
            multiline_mode: EditorLineMode::SingleLine,
            syntax_highlight: SyntaxHighlightConfig::Disable,
        };

        let shared_dialog_component = {
            let it = DialogComponent::new_shared(
                ComponentId::SimpleDialog as u8,
                dialog_options,
                editor_options,
                on_dialog_press_handler,
                on_dialog_editor_change_handler,
            );

            fn on_dialog_press_handler(
                dialog_choice: DialogChoice,
                shared_store: &SharedStore<State, Action>,
            ) {
                match dialog_choice {
                    DialogChoice::Yes(text) => {
                        spawn_dispatch_action!(
                            shared_store,
                            Action::SimpleDialogComponentInitializeFocused(
                                ComponentId::SimpleDialog as u8,
                                "Yes".to_string(),
                                text
                            )
                        );
                    }
                    DialogChoice::No => {
                        spawn_dispatch_action!(
                            shared_store,
                            Action::SimpleDialogComponentInitializeFocused(
                                ComponentId::SimpleDialog as u8,
                                "No".to_string(),
                                "".to_string()
                            )
                        );
                    }
                }
            }

            fn on_dialog_editor_change_handler(
                editor_buffer: EditorBuffer,
                shared_store: &SharedStore<State, Action>,
            ) {
                spawn_dispatch_action!(
                    shared_store,
                    Action::SimpleDialogComponentUpdateContent(
                        ComponentId::SimpleDialog as u8,
                        editor_buffer
                    )
                );
            }

            it
        };

        this.component_registry
            .put(ComponentId::SimpleDialog as u8, shared_dialog_component);

        call_if_true!(DEBUG_TUI_MOD, {
            let msg = format!(
                "🪙 {}",
                "construct DialogComponent (simple) { on_dialog_press }"
            );
            log_debug(msg);
        });
    }

    /// Insert editor component into registry if it's not already there.
    fn insert_editor_component(this: &mut AppWithLayout) {
        let id = ComponentId::Editor as u8;
        let shared_editor_component = {
            fn on_buffer_change(
                shared_store: &SharedStore<State, Action>,
                my_id: FlexBoxId,
                buffer: EditorBuffer,
            ) {
                spawn_dispatch_action!(
                    shared_store,
                    Action::EditorComponentUpdateContent(my_id, buffer)
                );
            }

            let config_options = EditorEngineConfigOptions::default();
            EditorComponent::new_shared(id, config_options, on_buffer_change)
        };

        this.component_registry.put(id, shared_editor_component);

        call_if_true!(DEBUG_TUI_MOD, {
            let msg = format!("🪙 {}", "construct EditorComponent { on_buffer_change }");
            log_debug(msg);
        });
    }
}

mod pretty_print {
    use super::*;

    impl Default for AppWithLayout {
        fn default() -> Self {
            // Potentially do any other initialization here.
            call_if_true!(DEBUG_TUI_MOD, {
                let msg = format!(
                    "🪙 {}",
                    "construct ex_editor::AppWithLayout { ComponentRegistry }"
                );
                log_debug(msg);
            });

            Self {
                component_registry: Default::default(),
            }
        }
    }

    impl Debug for AppWithLayout {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("AppWithLayout")
                .field("component_registry", &self.component_registry)
                .finish()
        }
    }
}

mod stylesheet {
    use super::*;

    pub fn create_stylesheet() -> CommonResult<Stylesheet> {
        throws_with_return!({
            stylesheet! {
              style! {
                id: EditorStyleName::Default as u8
                padding: 1
                // These are ignored due to syntax highlighting.
                // attrib: [bold]
                // color_fg: TuiColor::Blue
              },
              style! {
                id: DialogStyleName::Title as u8
                lolcat: true
                // These are ignored due to lolcat: true.
                // attrib: [bold]
                // color_fg: TuiColor::Yellow
              },
              style! {
                id: DialogStyleName::Border as u8
                lolcat: true
                // These are ignored due to lolcat: true.
                // attrib: [dim]
                // color_fg: TuiColor::Green
              },
              style! {
                id: DialogStyleName::Editor as u8
                attrib: [bold]
                color_fg: TuiColor::Basic(ANSIBasicColor::Magenta)
              },
              style! {
                id: DialogStyleName::ResultsPanel as u8
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
    pub fn render_status_bar(pipeline: &mut RenderPipeline, size: &Size) {
        let styled_texts = styled_texts! {
            styled_text! { @style: style!(attrib: [bold, dim]) ,      @text: "Hints: "},
            styled_text! { @style: style!(attrib: [dim, underline]) , @text: "Ctrl + x"},
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
