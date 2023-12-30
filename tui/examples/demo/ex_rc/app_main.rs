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

use chrono::{DateTime, Local};
use r3bl_rs_utils_core::*;
use r3bl_rs_utils_macro::style;
use r3bl_tui::*;
use tokio::{sync::mpsc::{self, Sender},
            time,
            time::Duration};

use super::*;

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
        fn from(id: Id) -> FlexBoxId { FlexBoxId(id as u8) }
    }
}

/// Trait object that implements the [App] trait.
#[derive(Debug)]
pub struct AppMain {
    pub data: AppData,
}

#[derive(Debug)]
pub struct AppData {
    pub lolcat_bg: ColorWheel,
    pub animator: Animator,
}

mod animator_task {
    use super::*;

    /// Note the [Sender] is used to send a signal to the animator to kill it when
    /// [Animator::stop](Animator::stop) is used.
    pub fn start_animator_task<AS>(
        main_thread_channel_sender: Sender<TerminalWindowMainThreadSignal<AS>>,
    ) -> Sender<()>
    where
        AS: Debug + Default + Clone + Sync + Send + 'static,
    {
        const ANIMATION_START_DELAY_MSEC: u64 = 500;
        const ANIMATION_INTERVAL_MSEC: u64 = 30; // 33 FPS.

        let (animator_kill_channel_sender, mut animator_kill_channel_receiver) =
            mpsc::channel::<()>(1);
        let animator_kill_channel_sender_clone = animator_kill_channel_sender.clone();

        tokio::spawn(async move {
            // Give the app some time to actually render to offscreen buffer.
            time::sleep(Duration::from_millis(ANIMATION_START_DELAY_MSEC)).await;

            loop {
                tokio::select! {
                    _ = animator_kill_channel_receiver.recv() => {
                        // Stop the animation.
                        break;
                    }
                    _ = time::sleep(Duration::from_millis(ANIMATION_INTERVAL_MSEC)) => {
                        // Continue the animation.

                        // Wire into the timing telemetry.
                        telemetry_global_static::set_start_ts();

                        // Send a signal to the main thread to render.
                        send_signal!(
                            main_thread_channel_sender,
                            TerminalWindowMainThreadSignal::Render(None)
                        );

                        // Wire into the timing telemetry.
                        telemetry_global_static::set_end_ts();
                    }
                }
            }
        });

        animator_kill_channel_sender_clone
    }
}

mod constructor {
    use super::*;

    impl Default for AppMain {
        fn default() -> Self {
            call_if_true!(DEBUG_TUI_MOD, {
                let msg = format!("🪙 {}", "construct ex_rc::AppWithLayout");
                log_debug(msg);
            });
            Self {
                data: AppData {
                    lolcat_bg: Default::default(),
                    animator: Default::default(),
                },
            }
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
    use crate::ex_rc::app_main::animator_task::start_animator_task;

    impl App for AppMain {
        type S = State;
        type AS = AppSignal;

        fn app_init(
            &mut self,
            component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
            has_focus: &mut HasFocus,
        ) {
            // Init local data.
            self.data.lolcat_bg = ColorWheel::new(vec![
                ColorWheelConfig::Lolcat(
                    LolcatBuilder::new()
                        .set_background_mode(true)
                        .set_color_change_speed(ColorChangeSpeed::Slow)
                        .set_seed(0.5)
                        .set_seed_delta(0.05),
                ),
                ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::BackgroundDarkGreenToDarkBlue,
                    ColorWheelSpeed::Slow,
                ),
            ]);

            populate_component_registry::create_components(
                component_registry_map,
                has_focus,
            );
        }

        /// Examples are provided of directly manipulating state and returning a request to
        /// re-render or sending a signal via the channel to
        /// [app_apply_action](app_apply_action).
        fn app_handle_input_event(
            &mut self,
            input_event: InputEvent,
            global_data: &mut GlobalData<State, AppSignal>,
            component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
            has_focus: &mut HasFocus,
        ) -> CommonResult<EventPropagation> {
            // Things from the app scope.
            let AppData { animator, .. } = &mut self.data;

            // Things from global scope.
            let GlobalData { state, .. } = global_data;

            // Ctrl + n => next slide.
            if input_event.matches_keypress(KeyPress::WithModifiers {
                key: Key::Character('n'),
                mask: ModifierKeysMask::new().with_ctrl(),
            }) {
                // Update state and re-render.
                state_mutator::next_slide(state);
                return Ok(EventPropagation::ConsumedRender);
            };

            // Ctrl + p => previous slide.
            if input_event.matches_keypress(KeyPress::WithModifiers {
                key: Key::Character('p'),
                mask: ModifierKeysMask::new().with_ctrl(),
            }) {
                // Spawn previous slide action.
                let main_thread_channel_sender_clone =
                    global_data.main_thread_channel_sender.clone();
                send_signal!(
                    main_thread_channel_sender_clone,
                    TerminalWindowMainThreadSignal::ApplyAction(AppSignal::PreviousSlide)
                );
                return Ok(EventPropagation::Consumed);
            };

            // Ctrl + q => Cancel animation & don't consume the event.
            if input_event.matches_keypress(KeyPress::WithModifiers {
                key: Key::Character('q'),
                mask: ModifierKeysMask::new().with_ctrl(),
            }) {
                animator.stop()?;
                return Ok(EventPropagation::ExitMainEventLoop);
            };

            ComponentRegistry::route_event_to_focused_component(
                global_data,
                input_event.clone(),
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
            _component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
            _has_focus: &mut HasFocus,
        ) -> CommonResult<EventPropagation> {
            throws_with_return!({
                let state = &mut global_data.state;
                match action {
                    AppSignal::Noop => {}
                    AppSignal::NextSlide => state_mutator::next_slide(state),
                    AppSignal::PreviousSlide => state_mutator::prev_slide(state),
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
                status_bar::render_status_bar(
                    self,
                    &mut surface.render_pipeline,
                    window_size,
                    &global_data.state,
                );

                // Handle animation.
                if self.data.animator.is_animation_not_started() {
                    self.data.animator.start::<AppSignal>(
                        global_data.main_thread_channel_sender.clone(),
                        start_animator_task,
                    );
                }

                // Return RenderOps pipeline (which will actually be painted elsewhere).
                surface.render_pipeline
            });
        }
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
                let component_id = FlexBoxId::from(Id::Editor);
                let style = Id::EditorStyleNameDefault.into();
                // Layout editor component, and render it.
                {
                    box_start! (
                        in:                     surface,
                        id:                     component_id,
                        dir:                    LayoutDirection::Vertical,
                        requested_size_percent: requested_size_percent!(width: 100, height: 100),
                        styles:                 [style]
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
    use super::*;

    pub fn create_components(
        component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
        has_focus: &mut HasFocus,
    ) {
        let id = FlexBoxId::from(Id::Editor);

        create_and_insert_editor_component_with_id(id, component_registry_map);

        // Switch focus to the editor component if focus is not set.
        has_focus.set_id(id);
        call_if_true!(DEBUG_TUI_MOD, {
            {
                let msg = format!("🪙 {} = {:?}", "init has_focus", has_focus.get_id());
                log_debug(msg);
            }
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
                edit_mode: EditMode::ReadWrite,
                ..Default::default()
            };

            let it = EditorComponent::new_boxed(id, config_options, on_buffer_change);
            it
        };

        ComponentRegistry::put(component_registry_map, id, boxed_editor_component);

        call_if_true!(DEBUG_TUI_MOD, {
            let msg = format!("🪙 {}", "construct EditorComponent { on_buffer_change }");
            log_debug(msg);
        });
    }
}

mod stylesheet {
    use super::*;

    pub fn create_stylesheet() -> CommonResult<Stylesheet> {
        throws_with_return!({
            let id = Id::EditorStyleNameDefault.into();
            stylesheet! {
              style! {
                id: id
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
    pub fn render_status_bar(
        app: &mut AppMain,
        pipeline: &mut RenderPipeline,
        window_size: Size,
        state: &State,
    ) {
        let mut it = styled_texts!();

        let lolcat_st = {
            let date_time: DateTime<Local> = Local::now();
            let time_str = date_time.format("%H:%M:%S").to_string();
            let time_us = UnicodeString::from(format!(" 🦜 {} ", time_str));
            let style = style! {
                color_fg: TuiColor::Basic(ANSIBasicColor::Black)
            };
            app.data.lolcat_bg.colorize_into_styled_texts(
                &time_us,
                GradientGenerationPolicy::ReuseExistingGradientAndIndex,
                TextColorizationPolicy::ColorEachCharacter(Some(style)),
            )
        };

        it += lolcat_st;

        it += styled_text! { @style:style!(attrib: [dim, bold]) ,      @text: " Exit 👋 : "};
        it += styled_text! { @style:style!(attrib: [dim, underline]) , @text: "Ctrl + q"};

        if state.current_slide_index < FILE_CONTENT_ARRAY.len() - 1 {
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
        let col_center: ChUnit = (window_size.col_count - display_width) / 2;
        let row_bottom: ChUnit = window_size.row_count - 1;
        let center: Position = position!(col_index: col_center, row_index: row_bottom);

        let mut render_ops = render_ops!();
        render_ops.push(RenderOp::MoveCursorPositionAbs(center));
        it.render_into(&mut render_ops);
        pipeline.push(ZOrder::Normal, render_ops);
    }
}
