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

use std::fmt::Debug;

use r3bl_core::{col,
                new_style,
                req_size_pc,
                row,
                send_signal,
                throws,
                throws_with_return,
                tui_color,
                tui_styled_text,
                tui_styled_texts,
                tui_stylesheet,
                CommonResult,
                Size,
                TuiStylesheet,
                SPACER_GLYPH};
use r3bl_tui::{box_end,
               box_props,
               box_start,
               render_component_in_current_box,
               render_ops,
               render_tui_styled_texts_into,
               surface,
               App,
               BoxedSafeApp,
               ComponentRegistry,
               ComponentRegistryMap,
               EditMode,
               EditorComponent,
               EditorEngineConfig,
               EventPropagation,
               FlexBoxId,
               GlobalData,
               HasFocus,
               InputEvent,
               Key,
               KeyPress,
               LayoutDirection,
               LayoutManagement,
               ModifierKeysMask,
               PerformPositioningAndSizing,
               RenderOp,
               RenderPipeline,
               Surface,
               SurfaceProps,
               SurfaceRender,
               TerminalWindowMainThreadSignal,
               ZOrder,
               DEBUG_TUI_MOD};
use tokio::sync::mpsc::Sender;

use crate::ex_pitch::state::{state_mutator, AppSignal, State, FILE_CONTENT_ARRAY};

/// Constants for the ids.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Id {
    Editor = 1,
    EditorStyleNameDefault = 4,
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

/// Trait object that implements the [App] trait.
pub struct AppMain;

mod constructor {
    use super::*;

    impl Default for AppMain {
        fn default() -> Self {
            DEBUG_TUI_MOD.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(message = "🪙 construct ex_pitch::AppWithLayout");
            });
            Self
        }
    }

    impl AppMain {
        /// Note that this needs to be initialized before it can be used.
        pub fn new_boxed() -> BoxedSafeApp<State, AppSignal> {
            let it = Self;
            Box::new(it)
        }
    }
}

mod app_main_impl_app_trait {
    use r3bl_core::height;

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

        /// Examples are provided of directly manipulating state and returning a request
        /// to re-render or sending a signal via the channel to
        /// [app_apply_action](app_apply_action).
        fn app_handle_input_event(
            &mut self,
            input_event: InputEvent,
            global_data: &mut GlobalData<State, AppSignal>,
            component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
            has_focus: &mut HasFocus,
        ) -> CommonResult<EventPropagation> {
            // Ctrl + n => next slide.
            if input_event.matches_keypress(KeyPress::WithModifiers {
                key: Key::Character('n'),
                mask: ModifierKeysMask::new().with_ctrl(),
            }) {
                // Spawn next slide action.
                send_signal!(
                    global_data.main_thread_channel_sender,
                    TerminalWindowMainThreadSignal::ApplyAppSignal(AppSignal::NextSlide)
                );
                return Ok(EventPropagation::ConsumedRender);
            };

            // Ctrl + p => previous slide.
            if input_event.matches_keypress(KeyPress::WithModifiers {
                key: Key::Character('p'),
                mask: ModifierKeysMask::new().with_ctrl(),
            }) {
                // Spawn previous slide action.
                send_signal!(
                    global_data.main_thread_channel_sender,
                    TerminalWindowMainThreadSignal::ApplyAppSignal(AppSignal::PrevSlide)
                );
                return Ok(EventPropagation::ConsumedRender);
            };

            // If modal not activated, route the input event to the focused component.
            ComponentRegistry::route_event_to_focused_component(
                global_data,
                input_event,
                component_registry_map,
                has_focus,
            )
        }

        /// Examples are provided of directly manipulating the state in the
        /// [app_handle_input_event](app_handle_input_event) method.
        fn app_handle_signal(
            &mut self,
            action: &AppSignal,
            global_data: &mut GlobalData<State, AppSignal>,
            component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
            _has_focus: &mut HasFocus,
        ) -> CommonResult<EventPropagation> {
            throws_with_return!({
                let state = &mut global_data.state;
                match action {
                    AppSignal::Noop => {}
                    AppSignal::NextSlide => {
                        state_mutator::next_slide(state, component_registry_map)
                    }
                    AppSignal::PrevSlide => {
                        state_mutator::prev_slide(state, component_registry_map)
                    }
                };
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

                // Create a surface and then run the SurfaceRenderer (ContainerSurfaceRender) on it.
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
                status_bar::render_status_bar(
                    &mut surface.render_pipeline,
                    window_size,
                    &global_data.state,
                );

                // Return RenderOps pipeline (which will actually be painted elsewhere).
                surface.render_pipeline
            });
        }
    }
}

mod perform_layout {
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
                let component_id = FlexBoxId::from(Id::Editor);
                // Layout editor component, and render it.
                {
                    box_start! (
                        in:                     surface,
                        id:                     component_id,
                        dir:                    LayoutDirection::Vertical,
                        requested_size_percent: req_size_pc!(width: 100, height: 100),
                        styles:                 [Id::EditorStyleNameDefault]
                    );
                    render_component_in_current_box!(
                        in:                 surface,
                        component_id:       component_id,
                        from:               component_registry_map,
                        global_data:        global_data,
                        has_focus:          has_focus
                    );
                    box_end!(in: surface);
                }
            });
        }
    }
}

mod populate_component_registry {
    use r3bl_core::{glyphs, inline_string};

    use super::*;

    pub fn create_components(
        component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
        has_focus: &mut HasFocus,
    ) {
        let id = FlexBoxId::from(Id::Editor);
        create_and_insert_editor_component_with_id(id, component_registry_map);

        // Switch focus to the editor component if focus is not set.
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
    fn create_and_insert_editor_component_with_id(
        id: FlexBoxId,
        component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
    ) {
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

            let config_options = EditorEngineConfig {
                edit_mode: EditMode::ReadOnly,
                ..Default::default()
            };

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
    use super::*;

    pub fn create_stylesheet() -> CommonResult<TuiStylesheet> {
        throws_with_return!({
            tui_stylesheet! {
                new_style! {
                    id: {Id::EditorStyleNameDefault}
                    padding: {1}
                    // These are ignored due to syntax highlighting.
                    // bold
                    // color_fg: {TuiColor::Basic(crate::ANSIBasicColor::Blue)}
                },
            }
        })
    }
}

mod hud {
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
    use super::*;

    /// Shows helpful messages at the bottom row of the screen.
    pub fn render_status_bar(pipeline: &mut RenderPipeline, size: Size, state: &State) {
        let color_bg = tui_color!(hex "#076DEB");
        let color_fg = tui_color!(hex "#E9C940");

        let mut styled_texts = tui_styled_texts! {
            tui_styled_text! {
                @style: new_style!(dim bold color_fg: {color_fg} color_bg: {color_bg}),
                @text: "Exit 👋 : "
            },
            tui_styled_text! {
                @style: new_style!(dim underline color_fg: {color_fg} color_bg: {color_bg}),
                @text: "Ctrl + q"
            },
        };

        if state.current_slide_index < FILE_CONTENT_ARRAY.len() - 1 {
            styled_texts += tui_styled_text! {
                @style: new_style!(dim bold color_fg: {color_fg} color_bg: {color_bg}),
                @text: " ┊ "
            };
            styled_texts += tui_styled_text! {
                @style: new_style!(dim bold color_fg: {color_fg} color_bg: {color_bg}),
                @text: "Next 👉 : "
            };
            styled_texts += tui_styled_text! {
                @style: new_style!(dim underline color_fg: {color_fg} color_bg: {color_bg}),
                @text: "Ctrl + n"
            };
        }

        if state.current_slide_index > 0 {
            styled_texts += tui_styled_text! {
                @style: new_style!(dim bold color_fg: {color_fg} color_bg: {color_bg}),
                @text: " ┊ "
            };
            styled_texts += tui_styled_text! {
                @style: new_style!(dim bold color_fg: {color_fg} color_bg: {color_bg}),
                @text: "Prev 👈 : "
            };
            styled_texts += tui_styled_text! {
                @style: new_style!(dim underline color_fg: {color_fg} color_bg: {color_bg}),
                @text: "Ctrl + p"
            };
        }

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
