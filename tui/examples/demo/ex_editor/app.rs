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
use int_enum::IntEnum;
use r3bl_redux::*;
use r3bl_rs_utils_core::*;
use r3bl_rs_utils_macro::style;
use r3bl_tui::*;
use strum_macros::AsRefStr;

use super::*;

/// Constants for the ids.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, IntEnum)]
enum Id {
    Editor = 1,
    Dialog = 2,
}

#[derive(Debug, Eq, PartialEq, AsRefStr)]
pub enum DialogStyleId {
    Border,
    Title,
    Editor,
}

/// Async trait object that implements the [App] trait.
pub struct AppWithLayout {
    pub component_registry: ComponentRegistry<State, Action>,
}

mod app_trait_impl {
    use super::*;

    #[async_trait]
    impl App<State, Action> for AppWithLayout {
        // ┏━━━━━━━━━━━━━━━━━━━━━━━━┓
        // ┃ get_component_registry ┃
        // ┛                        ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        fn get_component_registry(&mut self) -> &mut ComponentRegistry<State, Action> {
            &mut self.component_registry
        }

        // ┏━━━━━━┓
        // ┃ init ┃
        // ┛      ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        fn init(&mut self) { populate_component_registry::init(self); }

        // ┏━━━━━━━━━━━━━━━━━━┓
        // ┃ app_handle_event ┃
        // ┛                  ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
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
                let msg_1 = format!("🐝 focus: {:?}", self.component_registry.has_focus);
                let msg_2 = format!("💾 user_data: {:?}", self.component_registry.user_data);
                log_debug(msg_1);
                log_debug(msg_2);
            });

            // Check to see if the modal dialog should be activated.
            if let EventPropagation::Consumed =
                self.try_input_event_activate_modal(args, input_event)
            {
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

        // ┏━━━━━━━━━━━━┓
        // ┃ app_render ┃
        // ┛            ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
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
                    let mut it = surface!(stylesheet: style_helpers::create_stylesheet()?);

                    it.surface_start(SurfaceProps {
                        pos: position!(col_index: 0, row_index: 0),
                        size: size!(
                            col_count: window_size.col_count,
                            row_count: window_size.row_count - 1), // Bottom row for for status bar.
                    })?;

                    layout_container::ContainerSurfaceRender(self)
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
                status_bar_helpers::render_status_bar(&mut surface.render_pipeline, window_size);

                // Return RenderOps pipeline (which will actually be painted elsewhere).
                surface.render_pipeline
            });
        }
    }
}

mod detect_modal_dialog_activation_from_input_event {
    use super::*;

    impl AppWithLayout {
        /// If `input_event` matches <kbd>Ctrl+l</kbd>, then toggle the modal dialog. Note that this
        /// returns a [EventPropagation::Consumed] and not [EventPropagation::ConsumedRerender]
        /// because the [Action::SetDialogBufferTitleAndText] is dispatched to the store & that will
        /// cause a rerender.
        pub fn try_input_event_activate_modal(
            &mut self,
            args: GlobalScopeArgs<'_, State, Action>,
            input_event: &InputEvent,
        ) -> EventPropagation {
            if let DialogEvent::ActivateModal = DialogEvent::should_activate_modal(
                input_event,
                KeyPress::WithModifiers {
                    key: Key::Character('l'),
                    mask: ModifierKeysMask::CTRL,
                },
            ) {
                activate_modal(self, args);
                return EventPropagation::Consumed;
            } else {
                return EventPropagation::Propagate;
            }

            fn activate_modal(this: &mut AppWithLayout, args: GlobalScopeArgs<State, Action>) {
                let title = "Modal Dialog Title";
                let text = {
                    if let Some(editor_buffer) =
                        args.state.get_editor_buffer(Id::Editor.int_value())
                    {
                        editor_buffer.get_as_string()
                    } else {
                        "Press <Esc> to close, or <Enter> to accept".to_string()
                    }
                };

                // Setting the has_focus to Id::Dialog will cause the dialog to appear on the next
                // render.
                this.component_registry
                    .has_focus
                    .set_modal_id(Id::Dialog.int_value());

                // Change the state so that it will trigger a render. This will show the title &
                // text on the next render.
                spawn_dispatch_action!(
                    args.shared_store,
                    Action::SetDialogBufferTitleAndTextById(
                        Id::Dialog.int_value(),
                        title.to_string(),
                        text.to_string()
                    )
                );

                call_if_true!(DEBUG_TUI_MOD, {
                    let msg = format!("📣 activate modal: {:?}", this.component_registry.has_focus);
                    log_debug(msg);
                });
            }
        }
    }
}

mod layout_container {
    use super::*;

    pub struct ContainerSurfaceRender<'a>(pub &'a mut AppWithLayout);

    #[async_trait]
    impl SurfaceRenderer<State, Action> for ContainerSurfaceRender<'_> {
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
                        id:                     Id::Editor.int_value(),
                        dir:                    Direction::Vertical,
                        requested_size_percent: requested_size_percent!(width: 100, height: 100),
                        styles:                 [&Id::Editor.int_value().to_string()]
                    );
                    render_component_in_current_box!(
                        in:                 surface,
                        component_id:       Id::Editor.int_value(),
                        from:               self.0.component_registry,
                        state:              state,
                        shared_store:       shared_store,
                        shared_global_data: shared_global_data,
                        window_size:        window_size
                    );
                    box_end!(in: surface);
                }

                // Then, render modal dialog (if it is active, on top of the editor component).
                if self
                    .0
                    .component_registry
                    .has_focus
                    .is_modal_id(Id::Dialog.int_value())
                {
                    render_component_in_given_box! {
                      in:                 surface,
                      box:                DialogEngineApi::make_flex_box_for_dialog(Id::Dialog.int_value(), surface, window_size)?,
                      component_id:       Id::Dialog.int_value(),
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
    use crate::ex_editor::app::style_helpers::create_stylesheet;

    pub fn init(this: &mut AppWithLayout) {
        let editor_id = Id::Editor.int_value();
        let dialog_id = Id::Dialog.int_value();

        insert_editor_component(this, editor_id);
        insert_dialog_component(this, dialog_id);
        init_has_focus(this, editor_id);
    }

    /// Switch focus to the editor component if focus is not set.
    fn init_has_focus(this: &mut AppWithLayout, id: FlexBoxId) {
        this.component_registry.has_focus.set_id(id);
        call_if_true!(DEBUG_TUI_MOD, {
            {
                let msg = format!("🪙 {} = {}", "init component_registry.has_focus", id);
                log_debug(msg);
            }
        });
    }

    /// Insert dialog component into registry if it's not already there.
    fn insert_dialog_component(this: &mut AppWithLayout, id: FlexBoxId) {
        let result_stylesheet = create_stylesheet();

        let shared_dialog_component = {
            let it = DialogComponent::new_shared(
                id,
                on_dialog_press,
                on_dialog_editor_change_handler,
                get_style! { @from_result: result_stylesheet , DialogStyleId::Border.as_ref() },
                get_style! { @from_result: result_stylesheet , DialogStyleId::Title.as_ref() },
                get_style! { @from_result: result_stylesheet , DialogStyleId::Editor.as_ref() },
                EditorEngineConfigOptions {
                    multiline_mode: EditorLineMode::SingleLine,
                    syntax_highlight: SyntaxHighlightConfig::Disable,
                },
            );

            fn on_dialog_press(
                dialog_choice: DialogChoice,
                shared_store: &SharedStore<State, Action>,
            ) {
                match dialog_choice {
                    DialogChoice::Yes(text) => {
                        spawn_dispatch_action!(
                            shared_store,
                            Action::SetDialogBufferTitleAndTextById(
                                Id::Dialog.int_value(),
                                "Yes".to_string(),
                                text
                            )
                        );
                    }
                    DialogChoice::No => {
                        spawn_dispatch_action!(
                            shared_store,
                            Action::SetDialogBufferTitleAndTextById(
                                Id::Dialog.int_value(),
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
                    Action::UpdateDialogBufferById(Id::Dialog.int_value(), editor_buffer)
                );
            }

            it
        };

        this.component_registry.put(id, shared_dialog_component);

        call_if_true!(DEBUG_TUI_MOD, {
            let msg = format!("🪙 {}", "construct DialogComponent { on_dialog_press }");
            log_debug(msg);
        });
    }

    /// Insert editor component into registry if it's not already there.
    fn insert_editor_component(this: &mut AppWithLayout, id: FlexBoxId) {
        let shared_editor_component = {
            fn on_buffer_change(
                shared_store: &SharedStore<State, Action>,
                my_id: FlexBoxId,
                buffer: EditorBuffer,
            ) {
                spawn_dispatch_action!(shared_store, Action::UpdateEditorBufferById(my_id, buffer));
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

mod debug_helpers {
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

mod style_helpers {
    use super::*;

    pub fn create_stylesheet() -> CommonResult<Stylesheet> {
        throws_with_return!({
            stylesheet! {
              style! {
                id: Id::Editor.int_value().to_string()
                attrib: [bold]
                padding: 1
                color_fg: TuiColor::Blue
              },
              style! {
                id: DialogStyleId::Title.as_ref()
                attrib: [bold]
                color_fg: TuiColor::Yellow
                lolcat: true
              },
              style! {
                id: DialogStyleId::Border.as_ref()
                attrib: [dim]
                color_fg: TuiColor::Green
                lolcat: true
              },
              style! {
                id: DialogStyleId::Editor.as_ref()
                attrib: [bold]
                color_fg: TuiColor::Magenta
              }
            }
        })
    }
}

mod status_bar_helpers {
    use super::*;

    /// Shows helpful messages at the bottom row of the screen.
    pub fn render_status_bar(pipeline: &mut RenderPipeline, size: &Size) {
        let styled_texts = styled_texts! {
          styled_text! { "Hints:",                       style!(attrib: [dim])  },
          styled_text! { " Ctrl + x : Exit ⛔ ",         style!(attrib: [bold]) },
          styled_text! { " … ",                          style!(attrib: [dim])  },
          styled_text! { " Type content 🖖 ",            style!(attrib: [bold]) },
          styled_text! { " … ",                          style!(attrib: [dim])  },
          styled_text! { " Ctrl + l : Modal dialog 📣 ", style!(attrib: [bold]) }
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
