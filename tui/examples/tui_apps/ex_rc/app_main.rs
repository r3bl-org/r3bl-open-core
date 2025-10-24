// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{AppSignal, FILE_CONTENT_ARRAY, State, state_mutator};
use crate::ex_rc::app_main::animator_task::start_animator_task;
use chrono::{DateTime, Local};
use r3bl_tui::{Animator, Ansi256GradientIndex, App, BoxedSafeApp, ColorChangeSpeed,
               ColorWheel, ColorWheelConfig, ColorWheelSpeed, Colorize, CommonResult,
               ComponentRegistry, ComponentRegistryMap, DEBUG_TUI_MOD, EditMode,
               EditorComponent, EditorEngineConfig, EventPropagation, FlexBoxId,
               GCStringOwned, GlobalData, GradientGenerationPolicy, HasFocus,
               InputEvent, Key, KeyPress, LayoutDirection, LayoutManagement, LengthOps,
               LolcatBuilder, ModifierKeysMask, PerformPositioningAndSizing, RenderOpCommon,
               RenderOpIR, RenderOpsIR, RenderPipeline, SPACER_GLYPH, Size, Surface,
               SurfaceProps, SurfaceRender, TerminalWindowMainThreadSignal,
               TextColorizationPolicy, TuiStyledTexts, TuiStylesheet, ZOrder, box_end,
               box_start, col, glyphs, height, inline_string, new_style,
               render_component_in_current_box, render_tui_styled_texts_into, req_size_pc,
               row, send_signal, surface, throws, throws_with_return, tui_color,
               tui_styled_text, tui_styled_texts, tui_stylesheet};
use smallvec::smallvec;
use std::fmt::Debug;
use tokio::{sync::mpsc::Sender, time::Duration};

/// Constants for the ids.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Id {
    Editor = 1,
    EditorStyleNameDefault = 4,
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
    #[allow(clippy::wildcard_imports)]
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
            tokio::sync::mpsc::channel::<()>(1);
        let animator_kill_channel_sender_clone = animator_kill_channel_sender.clone();

        tokio::spawn(async move {
            // Give the app some time to actually render to offscreen buffer.
            tokio::time::sleep(Duration::from_millis(ANIMATION_START_DELAY_MSEC)).await;

            // Use an tokio::time::interval instead of tokio::time::sleep because we need
            // to be able to re-use it, and call tick on it repeatedly.
            let mut interval =
                tokio::time::interval(Duration::from_millis(ANIMATION_INTERVAL_MSEC));

            loop {
                tokio::select! {
                    // Stop the animation.
                    // This branch is cancel safe because recv is cancel safe.
                    _ = animator_kill_channel_receiver.recv() => {
                        // Stop the animation.
                        break;
                    }

                    // Trigger the animation by sending a signal (that mutates state, and
                    // causes rerender).
                    // This branch is cancel safe because tick is cancel safe.
                    _ = interval.tick() => {
                        // Continue the animation. Send a signal to the main thread to
                        // render.
                        send_signal!(
                            main_thread_channel_sender,
                            TerminalWindowMainThreadSignal::Render(None)
                        );
                    }
                }
            }
        });

        animator_kill_channel_sender_clone
    }
}

mod constructor {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Default for AppMain {
        fn default() -> Self {
            DEBUG_TUI_MOD.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(message = "🪙 construct ex_rc::AppWithLayout");
            });
            Self {
                data: AppData {
                    lolcat_bg: ColorWheel::default(),
                    animator: Animator::default(),
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
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl App for AppMain {
        type S = State;
        type AS = AppSignal;

        fn app_init(
            &mut self,
            component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
            has_focus: &mut HasFocus,
        ) {
            // Init local data.
            self.data.lolcat_bg = ColorWheel::new(smallvec![
                ColorWheelConfig::Lolcat(
                    LolcatBuilder::new()
                        .set_background_mode(Colorize::BothBackgroundAndForeground)
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
            // Things from the app scope.
            let AppData { animator, .. } = &mut self.data;

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
            }

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
                return Ok(EventPropagation::Consumed);
            }

            // Ctrl + q => Cancel animation & don't consume the event.
            if input_event.matches_keypress(KeyPress::WithModifiers {
                key: Key::Character('q'),
                mask: ModifierKeysMask::new().with_ctrl(),
            }) {
                animator.stop()?;
                return Ok(EventPropagation::ExitMainEventLoop);
            }

            ComponentRegistry::route_event_to_focused_component(
                global_data,
                input_event,
                component_registry_map,
                has_focus,
            )
        }

        /// Examples are provided of directly manipulating the state in
        /// [`Self::app_handle_input_event()`].
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
                        state_mutator::next_slide(state, component_registry_map);
                    }
                    AppSignal::PrevSlide => {
                        state_mutator::prev_slide(state, component_registry_map);
                    }
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
    #[allow(clippy::wildcard_imports)]
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
    #[allow(clippy::wildcard_imports)]
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
                edit_mode: EditMode::ReadWrite,
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
    #[allow(clippy::wildcard_imports)]
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

        let mut render_ops = RenderOpsIR::new();
        render_ops.push(RenderOpIR::Common(RenderOpCommon::MoveCursorPositionAbs(col(0) + row_idx)));
        render_ops.push(RenderOpIR::Common(RenderOpCommon::ResetColor));
        render_ops.push(RenderOpIR::Common(RenderOpCommon::SetBgColor(color_bg)));
        render_ops.push(RenderOpIR::PaintTextWithAttributes(
            SPACER_GLYPH.repeat(size.col_width.as_usize()).into(),
            None,
        ));
        render_ops.push(RenderOpIR::Common(RenderOpCommon::ResetColor));
        render_ops.push(RenderOpIR::Common(RenderOpCommon::MoveCursorPositionAbs(cursor)));
        render_tui_styled_texts_into(&styled_texts, &mut render_ops);
        pipeline.push(ZOrder::Normal, render_ops);
    }
}

mod status_bar {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Shows helpful messages at the bottom row of the screen.
    pub fn render_status_bar(
        app: &mut AppMain,
        pipeline: &mut RenderPipeline,
        size: Size,
        state: &State,
    ) {
        let color_bg = tui_color!(hex "#076DEB");
        let color_fg = tui_color!(hex "#E9C940");

        let lolcat_st = {
            let now: DateTime<Local> = Local::now();
            let time_string = inline_string!(" 🦜 {} ", now.format("%H:%M:%S"));
            let time_string_gcs = GCStringOwned::from(time_string);
            let style = new_style!(color_fg: {tui_color!(black)});
            app.data.lolcat_bg.colorize_into_styled_texts(
                &time_string_gcs,
                GradientGenerationPolicy::ReuseExistingGradientAndIndex,
                TextColorizationPolicy::ColorEachCharacter(Some(style)),
            )
        };

        let mut styled_texts = TuiStyledTexts::default();

        styled_texts += lolcat_st;

        styled_texts += tui_styled_text! {
            @style: new_style!(dim bold color_fg: {color_fg} color_bg: {color_bg}),
            @text: " Exit 👋 : "
        };
        styled_texts += tui_styled_text! {
            @style: new_style!(dim underline color_fg: {color_fg} color_bg: {color_bg}),
            @text: "Ctrl + q"
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
        let row_idx = size.row_height.convert_to_index(); /* Bottom row */
        let cursor = col_idx + row_idx;

        let mut render_ops = RenderOpsIR::new();
        render_ops.push(RenderOpIR::Common(RenderOpCommon::MoveCursorPositionAbs(col(0) + row_idx)));
        render_ops.push(RenderOpIR::Common(RenderOpCommon::ResetColor));
        render_ops.push(RenderOpIR::Common(RenderOpCommon::SetBgColor(color_bg)));
        render_ops.push(RenderOpIR::PaintTextWithAttributes(
            SPACER_GLYPH.repeat(size.col_width.as_usize()).into(),
            None,
        ));
        render_ops.push(RenderOpIR::Common(RenderOpCommon::ResetColor));
        render_ops.push(RenderOpIR::Common(RenderOpCommon::MoveCursorPositionAbs(cursor)));
        render_tui_styled_texts_into(&styled_texts, &mut render_ops);
        pipeline.push(ZOrder::Normal, render_ops);
    }
}
