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

use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use r3bl_redux::*;
use r3bl_rs_utils_core::*;
use r3bl_rs_utils_macro::style;
use r3bl_tui::*;
use tokio::{task::JoinHandle, time, time::Duration};

use super::*;
use crate::DEBUG;

/// Async trait object that implements the [Render] trait.
#[derive(Default)]
pub struct AppNoLayout {
    pub color_wheel_rgb: ColorWheel,
    pub color_wheel_ansi_vec: Vec<ColorWheel>,
    pub lolcat_fg: ColorWheel,
    pub lolcat_bg: ColorWheel,
    pub component_registry: ComponentRegistry<State, Action>,
    pub animation_started: AtomicBool,
    pub animation_task_handle: Option<JoinHandle<()>>,
}

macro_rules! fire {
    ($arg_event_consumed: ident, $arg_shared_store: ident, $arg_action: expr) => {
        spawn_and_consume_event!($arg_event_consumed, $arg_shared_store, $arg_action);
        call_if_true!(DEBUG, {
            let msg = format!(
                "⛵ AppNoLayout::handle_event -> dispatch_spawn: {}",
                $arg_action
            );
            log_info(msg)
        });
    };
}

mod handle_animation {

    use super::*;

    const ANIMATION_START_DELAY_SEC: u64 = 1;
    const ANIMATION_INTERVAL_SEC: u64 = 1;

    impl AppNoLayout {
        pub fn handle_animation(&mut self, shared_store: &SharedStore<State, Action>) {
            let is_animation_started = self.animation_started.load(Ordering::SeqCst);
            if is_animation_started {
                return;
            }

            let my_store_copy = shared_store.clone();

            // Save the handle so it can be aborted later.
            self.animation_task_handle = Some(tokio::spawn(async move {
                // Give the app some time to actually render to offscreen buffer.
                time::sleep(Duration::from_secs(ANIMATION_START_DELAY_SEC)).await;

                loop {
                    // Wire into the timing telemetry.
                    telemetry_global_static::set_start_ts();

                    // Dispatch the action.
                    my_store_copy
                        .write()
                        .await
                        .dispatch_action(Action::Add)
                        .await;

                    // Wire into the timing telemetry.
                    telemetry_global_static::set_end_ts();

                    // Wait for the next interval.
                    time::sleep(Duration::from_secs(ANIMATION_INTERVAL_SEC)).await;
                }
            }));

            self.animation_started.store(true, Ordering::SeqCst);
        }
    }
}

mod app_no_layout_impl_trait_app {
    use super::*;

    #[async_trait]
    impl App<State, Action> for AppNoLayout {
        async fn app_render(
            &mut self,
            args: GlobalScopeArgs<'_, State, Action>,
        ) -> CommonResult<RenderPipeline> {
            throws_with_return!({
                let GlobalScopeArgs {
                    state,
                    shared_global_data,
                    shared_store,
                    ..
                } = args;

                let sample_line_of_text = format!("{state}, gradient: [index: X, len: Y]",);
                let content_size_col: ChUnit = sample_line_of_text.len().into();
                let window_size: Size = shared_global_data.read().await.get_size();

                let col: ChUnit = (window_size.col_count - content_size_col) / 2;
                let mut row: ChUnit =
                    (window_size.row_count - ch!(2) - ch!(self.color_wheel_ansi_vec.len())) / 2;

                let mut pipeline = render_pipeline!();

                pipeline.push(ZOrder::Normal, {
                    let mut it = render_ops! {
                        @new
                        RenderOp::ResetColor,
                    };

                    // Render using color_wheel_ansi_vec.
                    for color_wheel_index in 0..self.color_wheel_ansi_vec.len() {
                        let color_wheel = &mut self.color_wheel_ansi_vec[color_wheel_index];

                        let unicode_string = {
                            let index = color_wheel.get_index();
                            let len = match color_wheel.get_gradient_len() {
                                GradientLengthKind::ColorWheel(len) => len,
                                _ => 0,
                            };
                            UnicodeString::from(format!(
                                "{state}, gradient: [index: {index}, len: {len}]"
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
                            let index = self.color_wheel_rgb.get_index();
                            let len = match self.color_wheel_rgb.get_gradient_len() {
                                GradientLengthKind::ColorWheel(len) => len,
                                _ => 0,
                            };
                            UnicodeString::from(format!(
                                "{state}, gradient: [index: {index}, len: {len}]"
                            ))
                        };

                        render_ops! {
                            @render_styled_texts_into it
                            =>
                            self.color_wheel_rgb.colorize_into_styled_texts(
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
                            UnicodeString::from(format!("{state}, gradient: [index: _, len: _]"))
                        };

                        let st = self.lolcat_fg.colorize_into_styled_texts(
                            &unicode_string,
                            GradientGenerationPolicy::ReuseExistingGradientAndIndex,
                            TextColorizationPolicy::ColorEachCharacter(None),
                        );
                        st.render_into(&mut it);

                        row += 1;
                    }

                    // Render using lolcat_bg.
                    {
                        it += RenderOp::MoveCursorPositionAbs(position!(
                            col_index: col,
                            row_index: row
                        ));

                        let unicode_string = {
                            UnicodeString::from(format!("{state}, gradient: [index: _, len: _]"))
                        };

                        let st = self.lolcat_bg.colorize_into_styled_texts(
                            &unicode_string,
                            GradientGenerationPolicy::ReuseExistingGradientAndIndex,
                            TextColorizationPolicy::ColorEachCharacter(None),
                        );
                        st.render_into(&mut it);

                        row += 1;
                    }

                    it
                });

                status_bar::create_status_bar_message(&mut pipeline, window_size);

                // Handle animation.
                self.handle_animation(shared_store);

                pipeline
            });
        }

        async fn app_handle_event(
            &mut self,
            args: GlobalScopeArgs<'_, State, Action>,
            input_event: &InputEvent,
        ) -> CommonResult<EventPropagation> {
            throws_with_return!({
                let GlobalScopeArgs { shared_store, .. } = args;

                call_if_true!(DEBUG, {
                    let msg = format!("⛵ AppNoLayout::handle_event -> input_event: {input_event}");
                    log_info(msg)
                });

                let mut event_consumed = false;

                if let InputEvent::Keyboard(KeyPress::Plain { key }) = input_event {
                    // Check for + or - key.
                    if let Key::Character(typed_char) = key {
                        match typed_char {
                            '+' => {
                                fire!(event_consumed, shared_store, Action::Add);
                            }
                            '-' => {
                                fire!(event_consumed, shared_store, Action::Sub);
                            }
                            // Override default behavior of 'x' key.
                            'x' => {
                                if let Some(handle) = &self.animation_task_handle {
                                    handle.abort();
                                }
                                event_consumed = false;
                            }
                            _ => {}
                        }
                    }

                    // Check for up or down arrow key.
                    if let Key::SpecialKey(special_key) = key {
                        match special_key {
                            SpecialKey::Up => {
                                fire!(event_consumed, shared_store, Action::Add);
                            }
                            SpecialKey::Down => {
                                fire!(event_consumed, shared_store, Action::Sub);
                            }
                            _ => {}
                        }
                    }
                }

                if event_consumed {
                    EventPropagation::Consumed
                } else {
                    EventPropagation::Propagate
                }
            });
        }

        fn init(&mut self) {
            self.color_wheel_ansi_vec = vec![
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

            self.color_wheel_rgb = ColorWheel::new(vec![ColorWheelConfig::Rgb(
                Vec::from(DEFAULT_GRADIENT_STOPS.map(String::from)),
                ColorWheelSpeed::Fast,
                25,
            )]);

            self.lolcat_fg = ColorWheel::new(vec![ColorWheelConfig::Lolcat(
                LolcatBuilder::new()
                    .set_color_change_speed(ColorChangeSpeed::Rapid)
                    .set_seed(0.5)
                    .set_seed_delta(1.0),
            )]);

            self.lolcat_bg = ColorWheel::new(vec![
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

        /// No-op.
        fn get_component_registry(&mut self) -> &mut ComponentRegistry<State, Action> {
            &mut self.component_registry
        }
    }
}

mod status_bar {
    use super::*;

    /// Shows helpful messages at the bottom row of the screen.
    pub fn create_status_bar_message(pipeline: &mut RenderPipeline, size: Size) {
        let styled_texts = styled_texts! {
          styled_text! { "Hints:",        style!(attrib: [dim])       },
          styled_text! { " x : Exit ⛔ ",  style!(attrib: [bold])      },
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
