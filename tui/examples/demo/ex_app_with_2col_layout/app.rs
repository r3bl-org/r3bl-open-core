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

use std::{fmt::Debug, sync::Arc};

use async_trait::async_trait;
use r3bl_rs_utils_core::*;
use r3bl_rs_utils_macro::style;
use r3bl_tui::*;
use tokio::sync::RwLock;

use super::*;

// Constants for the ids.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Id {
    Container = 1,
    Col1 = 2,
    Col2 = 3,
}

/// Async trait object that implements the [App] trait.
#[derive(Default)]
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

                    perform_layout::ContainerSurfaceRenderer(self)
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
                status_bar::render(&mut surface.render_pipeline, window_size);

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

            // Try to handle left and right arrow key input events & return if handled.
            if let Continuation::Return = self.handle_focus_switch(input_event) {
                return Ok(EventPropagation::ConsumedRender);
            }

            // Route any unhandled event to the component that has focus.
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

        fn init(&mut self) { self.init_component_registry(); }

        fn get_component_registry(&mut self) -> &mut ComponentRegistry<State, Action> {
            &mut self.component_registry
        }
    }
}

mod perform_layout {
    use super::*;

    pub struct ContainerSurfaceRenderer<'a>(pub &'a mut AppWithLayout);

    #[async_trait]
    impl SurfaceRender<State, Action> for ContainerSurfaceRenderer<'_> {
        async fn render_in_surface(
            &mut self,
            args: GlobalScopeArgs<'_, State, Action>,
            surface: &mut Surface,
        ) -> CommonResult<()> {
            let GlobalScopeArgs {
                state,
                shared_store,
                shared_global_data,
                window_size,
            } = args;

            // Layout and render the container.
            throws!({
                // Container.
                box_start!(
                    in: surface,
                    id: Id::Container as u8,
                    dir: LayoutDirection::Horizontal,
                    requested_size_percent: requested_size_percent!(width: 100, height: 100),
                    styles:                 [Id::Container as u8],
                );

                // Col1.
                {
                    box_start!(
                      in:                     surface,
                      id:                     Id::Col1 as u8,
                      dir:                    LayoutDirection::Vertical,
                      requested_size_percent: requested_size_percent!(width: 50, height: 100),
                      styles:                 [Id::Col1 as u8],
                    );
                    render_component_in_current_box!(
                        in:                 surface,
                        component_id:       Id::Col1 as u8,
                        from:               self.0.component_registry,
                        state:              state,
                        shared_store:       shared_store,
                        shared_global_data: shared_global_data,
                        window_size:        window_size
                    );
                    box_end!(in: surface);
                }

                // Col2.
                {
                    box_start!(
                      in:                     surface,
                      id:                     Id::Col2 as u8,
                      dir:                    LayoutDirection::Vertical,
                      requested_size_percent: requested_size_percent!(width: 50, height: 100),
                      styles:                 [Id::Col2 as u8],
                    );
                    render_component_in_current_box!(
                        in:                 surface,
                        component_id:       Id::Col2 as u8,
                        from:               self.0.component_registry,
                        state:              state,
                        shared_store:       shared_store,
                        shared_global_data: shared_global_data,
                        window_size:        window_size
                    );
                    box_end!(in: surface);
                }

                box_end!(in: surface);
            });
        }
    }
}

mod handle_focus {
    use super::*;

    impl AppWithLayout {
        pub fn handle_focus_switch(
            &mut self,
            input_event: &InputEvent,
        ) -> Continuation<String> {
            let mut event_consumed = false;

            // Handle Left, Right to switch focus between columns.
            if let InputEvent::Keyboard(keypress) = input_event {
                match keypress {
                    KeyPress::Plain {
                        key: Key::SpecialKey(SpecialKey::Left),
                    } => {
                        event_consumed = true;
                        self.switch_focus(SpecialKey::Left);
                        debug_log_has_focus(
                            stringify!(AppWithLayout::app_handle_event).into(),
                            &self.component_registry.has_focus,
                        );
                    }
                    KeyPress::Plain {
                        key: Key::SpecialKey(SpecialKey::Right),
                    } => {
                        event_consumed = true;
                        self.switch_focus(SpecialKey::Right);
                        debug_log_has_focus(
                            stringify!(AppWithLayout::app_handle_event).into(),
                            &self.component_registry.has_focus,
                        );
                    }
                    _ => {}
                }
            }

            if event_consumed {
                Continuation::Return
            } else {
                Continuation::Continue
            }
        }

        fn switch_focus(&mut self, special_key: SpecialKey) {
            if let Some(_id) = self.component_registry.has_focus.get_id() {
                if special_key == SpecialKey::Left {
                    self.component_registry.has_focus.set_id(Id::Col1 as u8)
                } else {
                    self.component_registry.has_focus.set_id(Id::Col2 as u8)
                }
            } else {
                log_error("No focus id has been set, and it should be set!".to_string());
            }
        }
    }
}

mod populate_component_registry {
    use super::*;

    impl AppWithLayout {
        pub fn init_component_registry(&mut self) {
            // Construct COL_1_ID.
            let col1_id = Id::Col1 as u8;
            if self.component_registry.does_not_contain(col1_id) {
                let component = ColumnRenderComponent::new(col1_id);
                let shared_component = Arc::new(RwLock::new(component));
                self.component_registry.put(col1_id, shared_component);
            }

            // Construct COL_2_ID.
            let col2_id = Id::Col2 as u8;
            if self.component_registry.does_not_contain(col2_id) {
                let component = ColumnRenderComponent::new(col2_id);
                let shared_component = Arc::new(RwLock::new(component));
                self.component_registry.put(col2_id, shared_component);
            }

            // Init has focus.
            if self.component_registry.has_focus.get_id().is_none() {
                self.component_registry.has_focus.set_id(col1_id);
            }
        }
    }
}

mod pretty_print {
    use super::*;

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
                id: Id::Container as u8
                padding: 1
              },
              style! {
                id: Id::Col1 as u8
                padding: 1
                color_bg: TuiColor::Rgb (RgbValue { red: 55, green: 55, blue: 100 })
              },
              style! {
                id: Id::Col2 as u8
                padding: 1
                color_bg: TuiColor::Rgb (RgbValue { red: 55, green: 55, blue: 248 })
              }
            }
        })
    }
}

mod status_bar {
    use super::*;

    /// Shows helpful messages at the bottom row of the screen.
    pub fn render(pipeline: &mut RenderPipeline, size: &Size) {
        let styled_texts = styled_texts! {
            styled_text! { @style: style!(attrib: [dim]),       @text: "Hints:" },
            styled_text! { @style: style!(attrib: [bold]),      @text: " x : Exit ⛔ " },
            styled_text! { @style: style!(attrib: [dim]),       @text: " … " },
            styled_text! { @style: style!(attrib: [underline]), @text: " ↑ / + : inc " },
            styled_text! { @style: style!(attrib: [dim]),       @text: " … " },
            styled_text! { @style: style!(attrib: [underline]), @text: " ↓ / - : dec " },
            styled_text! { @style: style!(attrib: [dim]),       @text: " … " },
            styled_text! { @style: style!(attrib: [underline]), @text: " ← / → : focus " }
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
