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
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EditorStyleName {
    Default = 4,
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
                status_bar::render_status_bar(
                    &mut surface.render_pipeline,
                    window_size,
                    state,
                );

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

            // Ctrl + n => next slide.
            if input_event.matches_keypress(KeyPress::WithModifiers {
                key: Key::Character('n'),
                mask: ModifierKeysMask::CTRL,
            }) {
                // Spawn next slide action.
                spawn_dispatch_action!(args.shared_store, Action::SlideControlNextSlide);
                return Ok(EventPropagation::Consumed);
            };

            // Ctrl + p => previous slide.
            if input_event.matches_keypress(KeyPress::WithModifiers {
                key: Key::Character('p'),
                mask: ModifierKeysMask::CTRL,
            }) {
                // Spawn previous slide action.
                spawn_dispatch_action!(
                    args.shared_store,
                    Action::SlideControlPreviousSlide
                );
                return Ok(EventPropagation::Consumed);
            };

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
            });
        }
    }
}

mod populate_component_registry {
    use super::*;

    pub fn init(this: &mut AppWithLayout) {
        insert_editor_component(this);

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

            let config_options = EditorEngineConfig {
                edit_mode: EditMode::ReadOnly,
                ..Default::default()
            };

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
                    "construct ex_pitch::AppWithLayout { ComponentRegistry }"
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
            }
        })
    }
}

mod status_bar {
    use super::*;

    /// Shows helpful messages at the bottom row of the screen.
    pub fn render_status_bar(pipeline: &mut RenderPipeline, size: &Size, state: &State) {
        let mut it = styled_texts! {
            styled_text! { @style:style!(attrib: [dim, bold]) ,      @text: "Exit 👋 : "},
            styled_text! { @style:style!(attrib: [dim, underline]) , @text: "Ctrl + x"},
        };

        if state.current_slide_index < LINES_ARRAY.len() - 1 {
            it += styled_text! { @style: style!(attrib: [dim, bold]) ,      @text: " ┊ "};
            it += styled_text! { @style: style!(attrib: [dim, bold]) ,      @text: "Next 👉 : "};
            it += styled_text! { @style: style!(attrib: [dim, underline]) , @text: "Ctrl + n"};
        }

        if state.current_slide_index > 0 {
            it += styled_text! { @style: style!(attrib: [dim, bold]) ,      @text: " ┊ "};
            it += styled_text! { @style: style!(attrib: [dim, bold]) ,      @text: "Prev 👈 : "};
            it += styled_text! { @style: style!(attrib: [dim, underline]) , @text: "Ctrl + p"};
        }

        let display_width = it.display_width();
        let col_center: ChUnit = (size.col_count - display_width) / 2;
        let row_bottom: ChUnit = size.row_count - 1;
        let center: Position = position!(col_index: col_center, row_index: row_bottom);

        let mut render_ops = render_ops!();
        render_ops.push(RenderOp::MoveCursorPositionAbs(center));
        it.render_into(&mut render_ops);
        pipeline.push(ZOrder::Normal, render_ops);
    }
}
