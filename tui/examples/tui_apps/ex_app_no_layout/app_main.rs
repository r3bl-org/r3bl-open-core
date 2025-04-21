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
use r3bl_tui::{ch,
               col,
               defaults::get_default_gradient_stops,
               glyphs,
               inline_string,
               new_style,
               render_ops,
               render_pipeline,
               render_tui_styled_texts_into,
               row,
               send_signal,
               tui_styled_text,
               width,
               Animator,
               Ansi256GradientIndex,
               App,
               BoxedSafeApp,
               ColorChangeSpeed,
               ColorWheel,
               ColorWheelConfig,
               ColorWheelSpeed,
               ComponentRegistryMap,
               EventPropagation,
               GlobalData,
               GradientGenerationPolicy,
               GradientLengthKind,
               HasFocus,
               InlineVec,
               InputEvent,
               Key,
               KeyPress,
               LolcatBuilder,
               RenderOp,
               RenderPipeline,
               SpecialKey,
               TerminalWindowMainThreadSignal,
               TextColorizationPolicy,
               ZOrder};
use smallvec::smallvec;
use tokio::{sync::mpsc::Sender, time::Duration};

use super::{AppSignal, State};
use crate::ENABLE_TRACE_EXAMPLES;

#[derive(Default)]
pub struct AppMain {
    pub data: AppData,
}

#[derive(Default)]
pub struct AppData {
    pub color_wheel_rgb: ColorWheel,
    pub color_wheel_ansi_vec: InlineVec<ColorWheel>,
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
                        // Continue the animation. Send a signal to the main thread to
                        // render.
                        send_signal!(
                            main_thread_channel_sender,
                            TerminalWindowMainThreadSignal::ApplyAppSignal(AppSignal::Add)
                        );
                    }
                }
            }
        });

        animator_kill_channel_sender_clone
    }
}

mod app_main_impl_trait_app {
    use r3bl_tui::{throws_with_return, Colorize, CommonResult, GCStringExt};

    use super::{animator_task::start_animator_task, *};

    impl App for AppMain {
        type S = State;
        type AS = AppSignal;

        fn app_init(
            &mut self,
            _component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
            _has_focus: &mut HasFocus,
        ) {
            let data = &mut self.data;

            data.color_wheel_ansi_vec = smallvec![
                ColorWheel::new(smallvec![ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::GrayscaleMediumGrayToWhite,
                    ColorWheelSpeed::Fast,
                )]),
                ColorWheel::new(smallvec![ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::DarkRedToDarkMagenta,
                    ColorWheelSpeed::Fast,
                )]),
                ColorWheel::new(smallvec![ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::RedToBrightPink,
                    ColorWheelSpeed::Fast,
                )]),
                ColorWheel::new(smallvec![ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::OrangeToNeonPink,
                    ColorWheelSpeed::Fast,
                )]),
                ColorWheel::new(smallvec![ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::LightYellowToWhite,
                    ColorWheelSpeed::Fast,
                )]),
                ColorWheel::new(smallvec![ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::MediumGreenToMediumBlue,
                    ColorWheelSpeed::Fast,
                )]),
                ColorWheel::new(smallvec![ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::GreenToBlue,
                    ColorWheelSpeed::Fast,
                )]),
                ColorWheel::new(smallvec![ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::LightGreenToLightBlue,
                    ColorWheelSpeed::Fast,
                )]),
                ColorWheel::new(smallvec![ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::LightLimeToLightMint,
                    ColorWheelSpeed::Fast,
                )]),
                ColorWheel::new(smallvec![ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::RustToPurple,
                    ColorWheelSpeed::Fast,
                )]),
                ColorWheel::new(smallvec![ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::OrangeToPink,
                    ColorWheelSpeed::Fast,
                )]),
                ColorWheel::new(smallvec![ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::LightOrangeToLightPurple,
                    ColorWheelSpeed::Fast,
                )]),
                ColorWheel::new(smallvec![ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::DarkOliveGreenToDarkLavender,
                    ColorWheelSpeed::Fast,
                )]),
                ColorWheel::new(smallvec![ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::OliveGreenToLightLavender,
                    ColorWheelSpeed::Fast,
                )]),
                ColorWheel::new(smallvec![ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::BackgroundDarkGreenToDarkBlue,
                    ColorWheelSpeed::Fast,
                )]),
            ];

            data.color_wheel_rgb = ColorWheel::new(smallvec![ColorWheelConfig::Rgb(
                get_default_gradient_stops(),
                ColorWheelSpeed::Fast,
                25,
            )]);

            data.lolcat_fg = ColorWheel::new(smallvec![
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

            data.lolcat_bg = ColorWheel::new(smallvec![
                ColorWheelConfig::Lolcat(
                    LolcatBuilder::new()
                        .set_background_mode(Colorize::BothBackgroundAndForeground)
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
                ENABLE_TRACE_EXAMPLES.then(|| {
                    // % is Display, ? is Debug.
                    tracing::info! {
                        message = "AppNoLayout::handle_event",
                        input_event = %inline_string!(
                            "{a} {b:?}",
                            a = glyphs::USER_INPUT_GLYPH,
                            b = input_event
                        )
                    };
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
                                    TerminalWindowMainThreadSignal::ApplyAppSignal(
                                        AppSignal::Add,
                                    )
                                );
                            }
                            '-' => {
                                event_consumed = true;
                                send_signal!(
                                    global_data.main_thread_channel_sender,
                                    TerminalWindowMainThreadSignal::ApplyAppSignal(
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
                                    TerminalWindowMainThreadSignal::ApplyAppSignal(
                                        AppSignal::Add,
                                    )
                                );
                            }
                            SpecialKey::Down => {
                                event_consumed = true;
                                send_signal!(
                                    global_data.main_thread_channel_sender,
                                    TerminalWindowMainThreadSignal::ApplyAppSignal(
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

        fn app_render(
            &mut self,
            global_data: &mut GlobalData<State, AppSignal>,
            _component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
            _has_focus: &mut HasFocus,
        ) -> CommonResult<RenderPipeline> {
            throws_with_return!({
                let state_string = inline_string!("{a:?}", a = global_data.state);

                let sample_line_of_text =
                    format!("{state_string}, gradient: [index: X, len: Y]");
                let content_size_col = width(sample_line_of_text.len());
                let window_size = global_data.window_size;
                let data = &mut self.data;

                // Horizontal centering.
                let col_idx = col({
                    let it = *window_size.col_width - *content_size_col;
                    it / 2
                });

                // Vertical centering.
                let mut row_idx = row({
                    // Shift up by 2 rows (adjust for extra status & HUD rows).
                    let vertical_offset = 2;
                    let content_height = ch(data.color_wheel_ansi_vec.len());
                    (*window_size.row_height - content_height) / 2 - vertical_offset
                });
                let mut pipeline = render_pipeline!();

                pipeline.push(ZOrder::Normal, {
                    let mut acc_render_ops = render_ops! {
                        @new
                        RenderOp::ResetColor,
                    };

                    // Render many rows using color_wheel_ansi_vec.
                    for color_wheel_index in 0..data.color_wheel_ansi_vec.len() {
                        let color_wheel =
                            &mut data.color_wheel_ansi_vec[color_wheel_index];

                        let text = {
                            let index = color_wheel.get_index();
                            let len = match color_wheel.get_gradient_len() {
                                GradientLengthKind::ColorWheel(len) => len,
                                _ => 0,
                            };
                            inline_string!(
                                "{state_string}, gradient: [index: {index:?}, len: {len}]"
                            )
                        };

                        let text_gcs = text.grapheme_string();

                        acc_render_ops +=
                            RenderOp::MoveCursorPositionAbs(col_idx + row_idx);

                        let texts = color_wheel.colorize_into_styled_texts(
                            &text_gcs,
                            GradientGenerationPolicy::ReuseExistingGradientAndIndex,
                            TextColorizationPolicy::ColorEachWord(None),
                        );
                        render_tui_styled_texts_into(&texts, &mut acc_render_ops);

                        row_idx += row(1);
                    }

                    // Render 1 row using color_wheel_rgb.
                    {
                        acc_render_ops +=
                            RenderOp::MoveCursorPositionAbs(col_idx + row_idx);

                        let text = {
                            let index = data.color_wheel_rgb.get_index();
                            let len = match data.color_wheel_rgb.get_gradient_len() {
                                GradientLengthKind::ColorWheel(len) => len,
                                _ => 0,
                            };
                            inline_string!(
                                "{state_string}, gradient: [index: {index:?}, len: {len}]"
                            )
                        };

                        let text_gcs = text.grapheme_string();

                        let texts = data.color_wheel_rgb.colorize_into_styled_texts(
                            &text_gcs,
                            GradientGenerationPolicy::ReuseExistingGradientAndIndex,
                            TextColorizationPolicy::ColorEachWord(None),
                        );
                        render_tui_styled_texts_into(&texts, &mut acc_render_ops);

                        row_idx += row(1);
                    }

                    // Render 1 row using lolcat_fg.
                    {
                        acc_render_ops +=
                            RenderOp::MoveCursorPositionAbs(col_idx + row_idx);

                        let text = {
                            inline_string!("{state_string}, gradient: [index: _, len: _]")
                        };

                        let text_gcs = text.grapheme_string();

                        let texts = data.lolcat_fg.colorize_into_styled_texts(
                            &text_gcs,
                            GradientGenerationPolicy::ReuseExistingGradientAndIndex,
                            TextColorizationPolicy::ColorEachCharacter(None),
                        );
                        render_tui_styled_texts_into(&texts, &mut acc_render_ops);

                        row_idx += row(1);
                    }

                    // Render 1 row using lolcat_bg.
                    {
                        acc_render_ops +=
                            RenderOp::MoveCursorPositionAbs(col_idx + row_idx);

                        let text = {
                            inline_string!("{state_string}, gradient: [index: _, len: _]")
                        };

                        let text_gcs = text.grapheme_string();

                        let texts = data.lolcat_bg.colorize_into_styled_texts(
                            &text_gcs,
                            GradientGenerationPolicy::ReuseExistingGradientAndIndex,
                            TextColorizationPolicy::ColorEachCharacter(None),
                        );
                        render_tui_styled_texts_into(&texts, &mut acc_render_ops);

                        row_idx += row(1);
                    }

                    acc_render_ops
                });

                status_bar::render_status_bar(&mut pipeline, window_size);

                hud::create_hud(
                    &mut pipeline,
                    window_size,
                    global_data.get_hud_report_with_spinner(),
                );

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
    }
}

mod hud {
    use r3bl_tui::{col, row, tui_color, tui_styled_texts, Size, SPACER_GLYPH};

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
    use r3bl_tui::{col, tui_color, tui_styled_texts, Size, SPACER_GLYPH};

    use super::*;

    /// Shows helpful messages at the bottom row of the screen.
    pub fn render_status_bar(pipeline: &mut RenderPipeline, size: Size) {
        let color_bg = tui_color!(hex "#076DEB");
        let color_fg = tui_color!(hex "#E9C940");
        let styled_texts = tui_styled_texts! {
            tui_styled_text! {
                @style: new_style!(dim color_fg:{color_fg} color_bg:{color_bg}),
                @text: "Hints:"
            },
            tui_styled_text! {
                @style: new_style!(bold color_fg:{color_fg} color_bg:{color_bg}),
                @text: " x : Exit 🖖 "
            },
            tui_styled_text! {
                @style: new_style!(dim color_fg:{color_fg} color_bg:{color_bg}),
                @text: " … "
            },
            tui_styled_text! {
                @style: new_style!(underline color_fg:{color_fg} color_bg:{color_bg}),
                @text: " ↑ / + : inc "
            },
            tui_styled_text! {
                @style: new_style!(dim color_fg:{color_fg} color_bg:{color_bg}),
                @text: " … "
            },
            tui_styled_text! {
                @style: new_style!(underline color_fg:{color_fg} color_bg:{color_bg}),
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
