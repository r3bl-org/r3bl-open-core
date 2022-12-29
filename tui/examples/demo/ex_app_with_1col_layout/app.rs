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
use int_enum::IntEnum;
use r3bl_rs_utils_core::*;
use r3bl_rs_utils_macro::style;
use r3bl_tui::*;
use tokio::sync::RwLock;

use super::*;

// Constants for the ids.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, IntEnum)]
pub enum Id {
    Container = 1,
    Col = 2,
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
        fn get_component_registry(&mut self) -> &mut ComponentRegistry<State, Action> {
            &mut self.component_registry
        }

        fn init(&mut self) { self.init_component_registry(); }

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
            let GlobalScopeArgs {
                state,
                shared_store,
                shared_global_data,
                window_size,
            } = args;

            // Layout ColumnRenderComponent and render it.
            throws!({
                let col_id = Id::Col.int_value();
                box_start! (
                    in:                     surface,
                    id:                     col_id,
                    dir:                    Direction::Vertical,
                    requested_size_percent: requested_size_percent!(width: 100, height: 100),
                    styles:                 [col_id]
                );
                render_component_in_current_box!(
                    in:                 surface,
                    component_id:       col_id,
                    from:               self.0.component_registry,
                    state:              state,
                    shared_store:       shared_store,
                    shared_global_data: shared_global_data,
                    window_size:        window_size
                );
                box_end!(in: surface);
            })
        }
    }
}

mod populate_component_registry {
    use super::*;

    impl AppWithLayout {
        pub fn init_component_registry(&mut self) {
            // Construct Col.
            let col_id = Id::Col.int_value();
            if self.component_registry.does_not_contain(col_id) {
                let _component = ColumnRenderComponent::new(col_id);
                let shared_component_r1 = Arc::new(RwLock::new(_component));
                self.component_registry.put(col_id, shared_component_r1);
            }

            // Init has focus.
            if self.component_registry.has_focus.get_id().is_none() {
                self.component_registry.has_focus.set_id(col_id);
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
                id: Id::Container.int_value()
                padding: 1
              },
              style! {
                id: Id::Col.int_value()
                padding: 1
                color_bg: TuiColor::Rgb { r: 55, g: 55, b: 100 }
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
          styled_text! { "Hints:",        style!(attrib: [dim])       },
          styled_text! { " x : Exit ⛔ ", style!(attrib: [bold])      },
          styled_text! { " … ",           style!(attrib: [dim])       },
          styled_text! { " ↑ / + : inc ", style!(attrib: [underline]) },
          styled_text! { " … ",           style!(attrib: [dim])       },
          styled_text! { " ↓ / - : dec ", style!(attrib: [underline]) }
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
