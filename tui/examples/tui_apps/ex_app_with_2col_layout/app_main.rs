// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::ops::AddAssign;
use super::{AppSignal, ColumnComponent, State};
use r3bl_tui::{App, BoxedSafeApp, CommonResult, ComponentRegistry, ComponentRegistryMap,
               ContainsResult, Continuation, EventPropagation, FlexBoxId, GlobalData,
               HasFocus, InputEvent, Key, KeyPress, LayoutDirection, LayoutManagement,
               LengthOps, PerformPositioningAndSizing, RenderOpCommon, RenderOpIR,
               RenderOpIRVec, RenderPipeline, SPACER_GLYPH, Size, SpecialKey, Surface,
               SurfaceProps, SurfaceRender, TuiStylesheet, ZOrder, box_end, box_start,
               col, glyphs, height, inline_string, new_style,
               render_component_in_current_box, render_tui_styled_texts_into,
               req_size_pc, row, surface, throws, throws_with_return, tui_color,
               tui_styled_text, tui_styled_texts, tui_stylesheet};

// Constants for the ids.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Id {
    Container = 1,
    Column1 = 2,
    Column2 = 3,
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
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl AppMain {
        pub fn new_boxed() -> BoxedSafeApp<State, AppSignal> {
            let it = Self::default();
            Box::new(it)
        }
    }
}

mod app_main_impl_app_trait {
    #[allow(clippy::wildcard_imports)]
    use super::*;

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
            // Try to handle left and right arrow key input events & return if handled.
            if let Continuation::Return =
                handle_focus::handle_focus_switch(input_event.clone(), has_focus)
            {
                return Ok(EventPropagation::ConsumedRender);
            }

            // Route any unhandled event to the component that has focus.
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

                // Create a surface and then run the SurfaceRenderer.
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

                    perform_layout::ContainerSurfaceRenderer { _app: self }
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
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub struct ContainerSurfaceRenderer<'a> {
        pub _app: &'a mut AppMain,
    }

    impl SurfaceRender<State, AppSignal> for ContainerSurfaceRenderer<'_> {
        fn render_in_surface(
            &mut self,
            surface: &mut Surface,
            global_data: &mut GlobalData<State, AppSignal>,
            component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
            has_focus: &mut HasFocus,
        ) -> CommonResult<()> {
            // Layout and render the container.
            throws!({
                // Container - start.
                let id_container = FlexBoxId::from(Id::Container);
                box_start!(
                    in: surface,
                    id: id_container,
                    dir: LayoutDirection::Horizontal,
                    requested_size_percent: req_size_pc!(width: 100, height: 100),
                    styles:                 [id_container],
                );

                // Col1.
                let id_column_1 = FlexBoxId::from(Id::Column1);
                {
                    box_start!(
                      in:                     surface,
                      id:                     id_column_1,
                      dir:                    LayoutDirection::Vertical,
                      requested_size_percent: req_size_pc!(width: 50, height: 100),
                      styles:                 [id_column_1],
                    );
                    render_component_in_current_box!(
                    in:                 surface,
                    component_id:       id_column_1,
                    from:               component_registry_map,
                    global_data:        global_data,
                    has_focus:          has_focus
                    );
                    box_end!(in: surface);
                }

                // Col2.
                let id_column_2 = FlexBoxId::from(Id::Column2);
                {
                    box_start!(
                      in:                     surface,
                      id:                     id_column_2,
                      dir:                    LayoutDirection::Vertical,
                      requested_size_percent: req_size_pc!(width: 50, height: 100),
                      styles:                 [id_column_2],
                    );
                    render_component_in_current_box!(
                    in:                 surface,
                    component_id:       id_column_2,
                    from:               component_registry_map,
                    global_data:        global_data,
                    has_focus:          has_focus
                    );
                    box_end!(in: surface);
                }

                // Container - end.
                box_end!(in: surface);
            });
        }
    }
}

mod handle_focus {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub fn handle_focus_switch(
        input_event: InputEvent,
        has_focus: &mut HasFocus,
    ) -> Continuation<String> {
        let mut event_consumed = false;

        // Handle Left, Right to switch focus between columns.
        if let InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(key),
        }) = input_event
        {
            match key {
                SpecialKey::Left => {
                    event_consumed = true;
                    handle_key(SpecialKey::Left, has_focus);
                }
                SpecialKey::Right => {
                    event_consumed = true;
                    handle_key(SpecialKey::Right, has_focus);
                }
                _ => {}
            }

            // % is Display, ? is Debug.
            tracing::info!(
                message = %inline_string!(
                    "AppWithLayout::app_handle_event -> switch focus {ch}",
                    ch = glyphs::FOCUS_GLYPH
                ),
                has_focus = ?has_focus
            );
        }

        if event_consumed {
            Continuation::Return
        } else {
            Continuation::Continue
        }
    }

    fn handle_key(special_key: SpecialKey, has_focus: &mut HasFocus) {
        if let Some(_id) = has_focus.get_id() {
            if special_key == SpecialKey::Left {
                has_focus.set_id(FlexBoxId::from(Id::Column1 as u8));
            } else {
                has_focus.set_id(FlexBoxId::from(Id::Column2 as u8));
            }
        } else {
            // % is Display, ? is Debug.
            tracing::error!(message = "No focus id has been set, and it should be set!");
        }
    }
}

mod populate_component_registry {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl AppMain {
        pub fn init_component_registry(
            map: &mut ComponentRegistryMap<State, AppSignal>,
            has_focus: &mut HasFocus,
        ) {
            // Construct Column1.
            let col1_id = FlexBoxId::from(Id::Column1 as u8);
            if let ContainsResult::DoesNotContain =
                ComponentRegistry::contains(map, col1_id)
            {
                let component = ColumnComponent::new_boxed(col1_id);
                ComponentRegistry::put(map, col1_id, component);
            }

            // Construct Column2.
            let col2_id = FlexBoxId::from(Id::Column2 as u8);
            if let ContainsResult::DoesNotContain =
                ComponentRegistry::contains(map, col2_id)
            {
                let boxed_component = ColumnComponent::new_boxed(col2_id);
                ComponentRegistry::put(map, col2_id, boxed_component);
            }

            // Init has focus.
            if has_focus.get_id().is_none() {
                has_focus.set_id(col1_id);
            }
        }
    }
}

mod stylesheet {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub fn create_stylesheet() -> CommonResult<TuiStylesheet> {
        throws_with_return!({
            tui_stylesheet! {
                new_style!(
                    id: {Id::Container}
                    padding: {1}
                ),
                new_style!(
                    id: {Id::Column1}
                    padding: {1}
                    color_bg: {tui_color!(55, 55, 100)}
                ),
                new_style!(
                    id: {Id::Column2}
                    padding: {1}
                    color_bg: {tui_color!(55, 55, 248)}
                )
            }
        })
    }
}

mod hud {
    #[allow(clippy::wildcard_imports)]
    use super::*;

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
        let row_idx = size.row_height.index_from_end(height(1)); /* 1 row above bottom */
        let cursor = col_idx + row_idx;

        let mut render_ops = RenderOpIRVec::new();
        render_ops  += (RenderOpCommon::MoveCursorPositionAbs(col(0) + row_idx));
        render_ops  += (RenderOpCommon::ResetColor);
        render_ops  += (RenderOpCommon::SetBgColor(color_bg));
        render_ops  += (RenderOpIR::PaintTextWithAttributes(
            SPACER_GLYPH.repeat(size.col_width.as_usize()).into(),
            None,
        ));
        render_ops  += (RenderOpCommon::ResetColor);
        render_ops  += (RenderOpCommon::MoveCursorPositionAbs(cursor));
        render_tui_styled_texts_into(&styled_texts, &mut render_ops);
        pipeline.push(ZOrder::Normal, render_ops);
    }
}

mod status_bar {
    #[allow(clippy::wildcard_imports)]
    use super::*;

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
        let row_idx = size.row_height.convert_to_index(); /* Bottom row */
        let cursor = col_idx + row_idx;

        let mut render_ops = RenderOpIRVec::new();
        render_ops  += (RenderOpCommon::MoveCursorPositionAbs(col(0) + row_idx));
        render_ops  += (RenderOpCommon::ResetColor);
        render_ops  += (RenderOpCommon::SetBgColor(color_bg));
        render_ops  += (RenderOpIR::PaintTextWithAttributes(
            SPACER_GLYPH.repeat(size.col_width.as_usize()).into(),
            None,
        ));
        render_ops  += (RenderOpCommon::ResetColor);
        render_ops  += (RenderOpCommon::MoveCursorPositionAbs(cursor));
        render_tui_styled_texts_into(&styled_texts, &mut render_ops);
        pipeline.push(ZOrder::Normal, render_ops);
    }
}
