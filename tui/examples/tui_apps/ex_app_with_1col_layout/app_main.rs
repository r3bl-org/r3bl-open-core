/*
 *   Copyright (c) 2022-2025 R3BL LLC
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
use r3bl_tui::{box_end, box_start, ch, col, height, new_style,
               render_component_in_current_box, render_ops,
               render_tui_styled_texts_into, req_size_pc, row, surface, throws,
               throws_with_return, tui_color, tui_styled_text, tui_styled_texts,
               tui_stylesheet, App, BoxedSafeApp, CommonResult, ComponentRegistry,
               ComponentRegistryMap, ContainsResult, EventPropagation, FlexBoxId,
               GlobalData, HasFocus, InputEvent, LayoutDirection, LayoutManagement,
               PerformPositioningAndSizing, RenderOp, RenderPipeline, Size, Surface,
               SurfaceProps, SurfaceRender, TuiStylesheet, ZOrder, SPACER_GLYPH};

use super::{AppSignal, SingleColumnComponent, State};

// Constants for the ids.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Id {
    Container = 1,
    Column = 2,
}

mod id_impl {
    use super::{FlexBoxId, Id};

    impl From<Id> for u8 {
        fn from(id: Id) -> u8 { id as u8 }
    }

    impl From<Id> for FlexBoxId {
        fn from(id: Id) -> FlexBoxId { FlexBoxId::new(id) }
    }
}

#[derive(Default)]
pub struct AppMain {
    _phantom: std::marker::PhantomData<(State, AppSignal)>,
}

mod constructor {
    use super::{AppMain, AppSignal, BoxedSafeApp, State};

    impl AppMain {
        pub fn new_boxed() -> BoxedSafeApp<State, AppSignal> {
            let it = Self::default();
            Box::new(it)
        }
    }
}

mod app_main_impl_app_trait {
    use super::{col, height, hud, perform_layout, row, status_bar, stylesheet, surface,
                throws_with_return, App, AppMain, AppSignal, CommonResult,
                ComponentRegistry, ComponentRegistryMap, EventPropagation, GlobalData,
                HasFocus, InputEvent, LayoutManagement, RenderPipeline, State,
                SurfaceProps, SurfaceRender};

    impl App for AppMain {
        type S = State;
        type AS = AppSignal;

        fn app_init(
            &mut self,
            component_registry_map: &mut ComponentRegistryMap<Self::S, Self::AS>,
            has_focus: &mut HasFocus,
        ) {
            Self::init_component_registry(component_registry_map, has_focus);
        }

        fn app_handle_input_event(
            &mut self,
            input_event: InputEvent,
            global_data: &mut GlobalData<State, AppSignal>,
            component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
            has_focus: &mut HasFocus,
        ) -> CommonResult<EventPropagation> {
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
            _component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
            _has_focus: &mut HasFocus,
        ) -> CommonResult<EventPropagation> {
            throws_with_return!({
                let stack = &mut global_data.state.stack;
                match action {
                    AppSignal::AddPop(arg) => {
                        if stack.is_empty() {
                            stack.push(*arg);
                        } else if let Some(top) = stack.pop() {
                            stack.push(top + arg);
                        }
                    }

                    AppSignal::SubPop(arg) => {
                        if stack.is_empty() {
                            stack.push(*arg);
                        } else if let Some(top) = stack.pop() {
                            stack.push(top - arg);
                        }
                    }

                    AppSignal::Clear => stack.clear(),

                    _ => {}
                }
                EventPropagation::ConsumedRender
            });
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
                        pos: col(0) + row(0),
                        size: {
                            let col_count = window_size.col_width;
                            let row_count = window_size.row_height -
                                height(2) /* Bottom row for for status bar & HUD. */;
                            col_count + row_count
                        },
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

                // Render HUD.
                hud::create_hud(
                    &mut surface.render_pipeline,
                    window_size,
                    global_data.get_hud_report_with_spinner(),
                );

                // Render status bar.
                status_bar::render_status_bar(&mut surface.render_pipeline, window_size);

                // Return RenderOps pipeline (which will actually be painted elsewhere).
                surface.render_pipeline
            });
        }
    }
}

mod perform_layout {
    use super::{box_end, box_start, render_component_in_current_box, req_size_pc,
                throws, AppMain, AppSignal, CommonResult, ComponentRegistryMap,
                FlexBoxId, GlobalData, HasFocus, Id, LayoutDirection, LayoutManagement,
                PerformPositioningAndSizing, State, Surface, SurfaceRender};

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
            // Layout ColumnRenderComponent and render it.
            throws!({
                let component_id = FlexBoxId::from(Id::Column);
                // Layout column component, and render it.
                box_start! (
                    in:                     surface,
                    id:                     component_id,
                    dir:                    LayoutDirection::Vertical,
                    requested_size_percent: req_size_pc!(width: 100, height: 100),
                    styles:                 [component_id]
                );
                render_component_in_current_box!(
                    in:                 surface,
                    component_id:       component_id,
                    from:               component_registry_map,
                    global_data:        global_data,
                    has_focus:          has_focus
                );
                box_end!(in: surface);
            })
        }
    }
}

mod populate_component_registry {
    use super::{AppMain, AppSignal, ComponentRegistry, ComponentRegistryMap,
                ContainsResult, FlexBoxId, HasFocus, Id, SingleColumnComponent, State};

    impl AppMain {
        pub fn init_component_registry(
            map: &mut ComponentRegistryMap<State, AppSignal>,
            has_focus: &mut HasFocus,
        ) {
            // Construct column component.
            let id = FlexBoxId::from(Id::Column);
            if let ContainsResult::DoesNotContain = ComponentRegistry::contains(map, id) {
                let component = SingleColumnComponent::new_boxed(id);
                ComponentRegistry::put(map, id, component);
            }
            // Init has focus.
            if has_focus.get_id().is_none() {
                has_focus.set_id(id);
            }
        }
    }
}

mod stylesheet {
    use super::{ch, new_style, throws_with_return, tui_color, tui_stylesheet,
                CommonResult, Id, TuiStylesheet};

    pub fn create_stylesheet() -> CommonResult<TuiStylesheet> {
        throws_with_return!({
            tui_stylesheet! {
              new_style!(id: {Id::Container} padding: {ch(1)}),
              new_style!(id: {Id::Column} padding: {ch(1)} color_bg: {
                tui_color!(55, 55, 100)

            })
            }
        })
    }
}

mod hud {
    use super::{col, new_style, render_ops, render_tui_styled_texts_into, row,
                tui_color, tui_styled_text, tui_styled_texts, RenderOp, RenderPipeline,
                Size, ZOrder, SPACER_GLYPH};

    pub fn create_hud(pipeline: &mut RenderPipeline, size: Size, hud_report_str: &str) {
        let color_bg = tui_color!(hex "#fdb6fd");
        let color_fg = tui_color!(hex "#942997");
        let styled_texts = tui_styled_texts! {
            tui_styled_text! {
                @style: new_style!(dim color_fg: {color_fg} color_bg: {color_bg}),
                @text: hud_report_str
            },
        };
        let display_width = styled_texts.display_width();
        let col_idx = col(*(size.col_width - display_width) / 2);
        let row_idx = size.row_height.convert_to_row_index() - row(1); /* 1 row above bottom */
        let cursor = col_idx + row_idx;

        let mut render_ops = render_ops!();
        render_ops.push(RenderOp::MoveCursorPositionAbs(col(0) + row_idx));
        render_ops.push(RenderOp::ResetColor);
        render_ops.push(RenderOp::SetBgColor(color_bg));
        render_ops.push(RenderOp::PaintTextWithAttributes(
            SPACER_GLYPH.repeat(size.col_width.as_usize()).into(),
            None,
        ));
        render_ops.push(RenderOp::ResetColor);
        render_ops.push(RenderOp::MoveCursorPositionAbs(cursor));
        render_tui_styled_texts_into(&styled_texts, &mut render_ops);
        pipeline.push(ZOrder::Normal, render_ops);
    }
}

mod status_bar {
    use super::{col, new_style, render_ops, render_tui_styled_texts_into, tui_color,
                tui_styled_text, tui_styled_texts, RenderOp, RenderPipeline, Size,
                ZOrder, SPACER_GLYPH};

    /// Shows helpful messages at the bottom row of the screen.
    pub fn render_status_bar(pipeline: &mut RenderPipeline, size: Size) {
        let color_bg = tui_color!(hex "#076DEB");
        let color_fg = tui_color!(hex "#E9C940");
        let styled_texts = tui_styled_texts! {
            tui_styled_text!{
                @style: new_style!(dim color_fg: {color_fg} color_bg: {color_bg}),
                @text: "Hints:"
            },
            tui_styled_text!{
                @style: new_style!(bold color_fg: {color_fg} color_bg: {color_bg}),
                @text: " x : Exit 🖖 "
            },
            tui_styled_text!{
                @style: new_style!(dim color_fg: {color_fg} color_bg: {color_bg}),
                @text: " … "
            },
            tui_styled_text!{
                @style: new_style!(underline color_fg: {color_fg} color_bg: {color_bg}),
                @text: " ↑ / + : inc "
            },
            tui_styled_text!{
                @style: new_style!(dim color_fg: {color_fg} color_bg: {color_bg}),
                @text: " … "
            },
            tui_styled_text!{
                @style: new_style!(underline color_fg: {color_fg} color_bg: {color_bg}),
                @text: " ↓ / - : dec "
            },
        };

        let display_width = styled_texts.display_width();
        let col_idx = col(*(size.col_width - display_width) / 2);
        let row_idx = size.row_height.convert_to_row_index(); /* Bottom row */
        let cursor = col_idx + row_idx;

        let mut render_ops = render_ops!();
        render_ops.push(RenderOp::MoveCursorPositionAbs(col(0) + row_idx));
        render_ops.push(RenderOp::ResetColor);
        render_ops.push(RenderOp::SetBgColor(color_bg));
        render_ops.push(RenderOp::PaintTextWithAttributes(
            SPACER_GLYPH.repeat(size.col_width.as_usize()).into(),
            None,
        ));
        render_ops.push(RenderOp::ResetColor);
        render_ops.push(RenderOp::MoveCursorPositionAbs(cursor));
        render_tui_styled_texts_into(&styled_texts, &mut render_ops);
        pipeline.push(ZOrder::Normal, render_ops);
    }
}
