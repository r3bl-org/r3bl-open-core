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
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

use r3bl_rs_utils_core::*;
use r3bl_rs_utils_macro::tui_style;
use r3bl_tui::*;
use tokio::{sync::mpsc::Sender, time::Duration};

use super::*;
use crate::ENABLE_TRACE_EXAMPLES;

#[derive(Default)]
pub struct AppMain {
    pub data: AppData,
}

#[derive(Default)]
pub struct AppData {
    pub color_wheel_rgb: ColorWheel,
    pub color_wheel_ansi_vec: Vec<ColorWheel>,
    pub lolcat_fg: ColorWheel,
    pub lolcat_bg: ColorWheel,
    pub animator: Animator,
}

mod constructor {
    use super::*;

    impl AppMain {
        pub fn new_boxed() -> BoxedSafeApp<State, AppSignal> {
            let it = Self::default();
            Box::new(it)
        }
    }
}

mod animator_task {
    use super::*;

    pub fn start_animator_task(
        main_thread_channel_sender: Sender<TerminalWindowMainThreadSignal<AppSignal>>,
    ) -> Sender<()> {
        const ANIMATION_START_DELAY_MSEC: u64 = 500;
        const ANIMATION_INTERVAL_MSEC: u64 = 500;

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
                        break;
                    }

                    // Trigger the animation by sending a signal (that mutates state, and
                    // causes rerender).
                    // This branch is cancel safe because tick is cancel safe.
                    _ = interval.tick() => {
                        // Continue the animation.

                        // Wire into the timing telemetry.
                        telemetry_global_static::set_start_ts();

                        // Send a signal to the main thread to render.
                        send_signal!(
                            main_thread_channel_sender,
                            TerminalWindowMainThreadSignal::ApplyAction(AppSignal::Add)
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

mod app_main_impl_trait_app {
    use super::{animator_task::start_animator_task, *};

    impl App for AppMain {
        type S = State;
        type AS = AppSignal;

        fn app_render(
            &mut self,
            global_data: &mut GlobalData<State, AppSignal>,
            _component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
            _has_focus: &mut HasFocus,
        ) -> CommonResult<RenderPipeline> {
            throws_with_return!({
                let state_str = format!("{}", global_data.state);
                let data = &mut self.data;

                let sample_line_of_text =
                    format!("{state_str}, gradient: [index: X, len: Y]");
                let content_size_col = ChUnit::from(sample_line_of_text.len());
                let window_size = global_data.window_size;

                let col = (window_size.col_count - content_size_col) / 2;
                let mut row = (window_size.row_count
                    - ch!(2)
                    - ch!(data.color_wheel_ansi_vec.len()))
                    / 2;

                let mut pipeline = render_pipeline!();

                pipeline.push(ZOrder::Normal, {
                    let mut it = render_ops! {
                        @new
                        RenderOp::ResetColor,
                    };

                    // Render using color_wheel_ansi_vec.
                    for color_wheel_index in 0..data.color_wheel_ansi_vec.len() {
                        let color_wheel =
                            &mut data.color_wheel_ansi_vec[color_wheel_index];

                        let unicode_string = {
                            let index = color_wheel.get_index();
                            let len = match color_wheel.get_gradient_len() {
                                GradientLengthKind::ColorWheel(len) => len,
                                _ => 0,
                            };
                            UnicodeString::from(format!(
                                "{state_str}, gradient: [index: {index}, len: {len}]"
                            ))
                        };

                        it += RenderOp::MoveCursorPositionAbs(position!(
                            col_index: col,
                            row_index: row
                        ));

                        render_ops! {
                            @render_styled_texts_into it
                            =>
                            color_wheel.colorize_into_styled_texts(
                                &unicode_string,
                                GradientGenerationPolicy::ReuseExistingGradientAndIndex,
                                TextColorizationPolicy::ColorEachWord(None),
                            )
                        }

                        row += 1;
                    }

                    // Render using color_wheel_rgb.
                    {
                        it += RenderOp::MoveCursorPositionAbs(position!(
                            col_index: col,
                            row_index: row
                        ));

                        let unicode_string = {
                            let index = data.color_wheel_rgb.get_index();
                            let len = match data.color_wheel_rgb.get_gradient_len() {
                                GradientLengthKind::ColorWheel(len) => len,
                                _ => 0,
                            };
                            UnicodeString::from(format!(
                                "{state_str}, gradient: [index: {index}, len: {len}]"
                            ))
                        };

                        render_ops! {
                            @render_styled_texts_into it
                            =>
                            data.color_wheel_rgb.colorize_into_styled_texts(
                                &unicode_string,
                                GradientGenerationPolicy::ReuseExistingGradientAndIndex,
                                TextColorizationPolicy::ColorEachWord(None),
                            )
                        }

                        row += 1;
                    }

                    // Render using lolcat_fg.
                    {
                        it += RenderOp::MoveCursorPositionAbs(position!(
                            col_index: col,
                            row_index: row
                        ));

                        let unicode_string = {
                            UnicodeString::from(format!(
                                "{state_str}, gradient: [index: _, len: _]"
                            ))
                        };

                        let texts = data.lolcat_fg.colorize_into_styled_texts(
                            &unicode_string,
                            GradientGenerationPolicy::ReuseExistingGradientAndIndex,
                            TextColorizationPolicy::ColorEachCharacter(None),
                        );
                        render_tui_styled_texts_into(&texts, &mut it);

                        row += 1;
                    }

                    // Render using lolcat_bg.
                    {
                        it += RenderOp::MoveCursorPositionAbs(position!(
                            col_index: col,
                            row_index: row
                        ));

                        let unicode_string = {
                            UnicodeString::from(format!(
                                "{state_str}, gradient: [index: _, len: _]"
                            ))
                        };

                        let texts = data.lolcat_bg.colorize_into_styled_texts(
                            &unicode_string,
                            GradientGenerationPolicy::ReuseExistingGradientAndIndex,
                            TextColorizationPolicy::ColorEachCharacter(None),
                        );
                        render_tui_styled_texts_into(&texts, &mut it);

                        row += 1;
                    }

                    it
                });

                status_bar::create_status_bar_message(&mut pipeline, window_size);

                // Handle animation.
                if data.animator.is_animation_not_started() {
                    data.animator.start(
                        global_data.main_thread_channel_sender.clone(),
                        start_animator_task,
                    )
                }

                pipeline
            });
        }

        fn app_handle_input_event(
            &mut self,
            input_event: InputEvent,
            global_data: &mut GlobalData<State, AppSignal>,
            _component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
            _has_focus: &mut HasFocus,
        ) -> CommonResult<EventPropagation> {
            // Things from the app scope.
            let AppData { animator, .. } = &mut self.data;

            throws_with_return!({
                call_if_true!(ENABLE_TRACE_EXAMPLES, {
                    let msg = format!(
                        "⛵ AppNoLayout::handle_event -> input_event: {input_event}"
                    );
                    log_info(msg)
                });

                let mut event_consumed = false;

                if let InputEvent::Keyboard(KeyPress::Plain { key }) = input_event {
                    // Check for + or - key.
                    if let Key::Character(typed_char) = key {
                        match typed_char {
                            '+' => {
                                event_consumed = true;
                                send_signal!(
                                    global_data.main_thread_channel_sender,
                                    TerminalWindowMainThreadSignal::ApplyAction(
                                        AppSignal::Add,
                                    )
                                );
                            }
                            '-' => {
                                event_consumed = true;
                                send_signal!(
                                    global_data.main_thread_channel_sender,
                                    TerminalWindowMainThreadSignal::ApplyAction(
                                        AppSignal::Sub,
                                    )
                                );
                            }
                            // Override default behavior of 'x' key.
                            'x' => {
                                event_consumed = false;
                                let _ = animator.stop();
                            }
                            _ => {}
                        }
                    }

                    // Check for up or down arrow key.
                    if let Key::SpecialKey(special_key) = key {
                        match special_key {
                            SpecialKey::Up => {
                                event_consumed = true;
                                send_signal!(
                                    global_data.main_thread_channel_sender,
                                    TerminalWindowMainThreadSignal::ApplyAction(
                                        AppSignal::Add,
                                    )
                                );
                            }
                            SpecialKey::Down => {
                                event_consumed = true;
                                send_signal!(
                                    global_data.main_thread_channel_sender,
                                    TerminalWindowMainThreadSignal::ApplyAction(
                                        AppSignal::Sub,
                                    )
                                );
                            }
                            _ => {}
                        }
                    }
                }

                if event_consumed {
                    EventPropagation::ConsumedRender
                } else {
                    EventPropagation::Propagate
                }
            });
        }

        fn app_handle_signal(
            &mut self,
            action: &AppSignal,
            global_data: &mut GlobalData<State, AppSignal>,
            _component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
            _has_focus: &mut HasFocus,
        ) -> CommonResult<EventPropagation> {
            throws_with_return!({
                let GlobalData { state, .. } = global_data;

                match action {
                    AppSignal::Add => {
                        state.counter += 1;
                    }

                    AppSignal::Sub => {
                        state.counter -= 1;
                    }

                    AppSignal::Clear => {
                        state.counter = 0;
                    }

                    _ => {}
                }

                EventPropagation::ConsumedRender
            });
        }

        fn app_init(
            &mut self,
            _component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
            _has_focus: &mut HasFocus,
        ) {
            let data = &mut self.data;

            data.color_wheel_ansi_vec = vec![
                ColorWheel::new(vec![ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::GrayscaleMediumGrayToWhite,
                    ColorWheelSpeed::Fast,
                )]),
                ColorWheel::new(vec![ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::DarkRedToDarkMagenta,
                    ColorWheelSpeed::Fast,
                )]),
                ColorWheel::new(vec![ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::RedToBrightPink,
                    ColorWheelSpeed::Fast,
                )]),
                ColorWheel::new(vec![ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::OrangeToNeonPink,
                    ColorWheelSpeed::Fast,
                )]),
                ColorWheel::new(vec![ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::LightYellowToWhite,
                    ColorWheelSpeed::Fast,
                )]),
                ColorWheel::new(vec![ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::MediumGreenToMediumBlue,
                    ColorWheelSpeed::Fast,
                )]),
                ColorWheel::new(vec![ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::GreenToBlue,
                    ColorWheelSpeed::Fast,
                )]),
                ColorWheel::new(vec![ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::LightGreenToLightBlue,
                    ColorWheelSpeed::Fast,
                )]),
                ColorWheel::new(vec![ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::LightLimeToLightMint,
                    ColorWheelSpeed::Fast,
                )]),
                ColorWheel::new(vec![ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::RustToPurple,
                    ColorWheelSpeed::Fast,
                )]),
                ColorWheel::new(vec![ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::OrangeToPink,
                    ColorWheelSpeed::Fast,
                )]),
                ColorWheel::new(vec![ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::LightOrangeToLightPurple,
                    ColorWheelSpeed::Fast,
                )]),
                ColorWheel::new(vec![ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::DarkOliveGreenToDarkLavender,
                    ColorWheelSpeed::Fast,
                )]),
                ColorWheel::new(vec![ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::OliveGreenToLightLavender,
                    ColorWheelSpeed::Fast,
                )]),
                ColorWheel::new(vec![ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::BackgroundDarkGreenToDarkBlue,
                    ColorWheelSpeed::Fast,
                )]),
            ];

            data.color_wheel_rgb = ColorWheel::new(vec![ColorWheelConfig::Rgb(
                Vec::from(DEFAULT_GRADIENT_STOPS.map(String::from)),
                ColorWheelSpeed::Fast,
                25,
            )]);

            data.lolcat_fg = ColorWheel::new(vec![
                ColorWheelConfig::Lolcat(
                    LolcatBuilder::new()
                        .set_color_change_speed(ColorChangeSpeed::Rapid)
                        .set_seed(0.5)
                        .set_seed_delta(1.0),
                ),
                ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::LightOrangeToLightPurple,
                    ColorWheelSpeed::Slow,
                ),
            ]);

            data.lolcat_bg = ColorWheel::new(vec![
                ColorWheelConfig::Lolcat(
                    LolcatBuilder::new()
                        .set_background_mode(true)
                        .set_color_change_speed(ColorChangeSpeed::Rapid)
                        .set_seed(0.5)
                        .set_seed_delta(1.0),
                ),
                ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::BackgroundDarkGreenToDarkBlue,
                    ColorWheelSpeed::Slow,
                ),
            ]);
        }
    }
}

mod status_bar {
    use super::*;

    /// Shows helpful messages at the bottom row of the screen.
    pub fn create_status_bar_message(pipeline: &mut RenderPipeline, size: Size) {
        let styled_texts = tui_styled_texts! {
            tui_styled_text!{ @style: tui_style!(attrib: [dim])       , @text: "Hints:"},
            tui_styled_text!{ @style: tui_style!(attrib: [bold])      , @text: " x : Exit ⛔ "},
            tui_styled_text!{ @style: tui_style!(attrib: [dim])       , @text: " … "},
            tui_styled_text!{ @style: tui_style!(attrib: [underline]) , @text: " ↑ / + : inc "},
            tui_styled_text!{ @style: tui_style!(attrib: [dim])       , @text: " … "},
            tui_styled_text!{ @style: tui_style!(attrib: [underline]) , @text: " ↓ / - : dec "},
        };

        let display_width = styled_texts.display_width();
        let col_center: ChUnit = (size.col_count - display_width) / 2;
        let row_bottom: ChUnit = size.row_count - 1;
        let center: Position = position!(col_index: col_center, row_index: row_bottom);

        let mut render_ops = render_ops!();
        render_ops.push(RenderOp::MoveCursorPositionAbs(center));
        render_tui_styled_texts_into(&styled_texts, &mut render_ops);
        pipeline.push(ZOrder::Normal, render_ops);
    }
}
