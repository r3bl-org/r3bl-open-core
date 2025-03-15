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

use r3bl_core::{call_if_true,
                ch,
                glyphs,
                position,
                send_signal,
                throws_with_return,
                Ansi256GradientIndex,
                ColorWheel,
                ColorWheelConfig,
                ColorWheelSpeed,
                CommonResult,
                GradientGenerationPolicy,
                TextColorizationPolicy,
                UnicodeString,
                UnicodeStringExt};
use r3bl_tui::{render_ops,
               render_pipeline,
               BoxedSafeComponent,
               Component,
               EventPropagation,
               FlexBox,
               FlexBoxId,
               GlobalData,
               HasFocus,
               InputEvent,
               Key,
               KeyPress,
               RenderOp,
               RenderPipeline,
               SpecialKey,
               SurfaceBounds,
               TerminalWindowMainThreadSignal,
               ZOrder,
               DEBUG_TUI_MOD};

use super::{AppSignal, State};

#[derive(Debug, Clone, Default)]
pub struct ColumnComponent {
    pub data: ColumnComponentData,
}

#[derive(Debug, Clone, Default)]
pub struct ColumnComponentData {
    pub color_wheel: ColorWheel,
    pub id: FlexBoxId,
}

mod constructor {
    use super::*;

    impl ColumnComponent {
        pub fn new_boxed(id: FlexBoxId) -> BoxedSafeComponent<State, AppSignal> {
            let it = Self {
                data: ColumnComponentData {
                    id,
                    color_wheel: ColorWheel::new(vec![
                        ColorWheelConfig::RgbRandom(ColorWheelSpeed::Fast),
                        ColorWheelConfig::Ansi256(
                            Ansi256GradientIndex::LightGreenToLightBlue,
                            ColorWheelSpeed::Fast,
                        ),
                    ]),
                },
            };
            Box::new(it)
        }
    }
}

mod column_render_component_impl_component_trait {
    use super::*;

    impl Component<State, AppSignal> for ColumnComponent {
        fn reset(&mut self) {}

        fn get_id(&self) -> FlexBoxId { self.data.id }

        /// Handle following input events (and consume them):
        /// - Up,   `+` : fire `AddPop(1)`
        /// - Down, `-` : fire `SubPop(1)`
        fn handle_event(
            &mut self,
            global_data: &mut GlobalData<State, AppSignal>,
            input_event: InputEvent,
            _has_focus: &mut HasFocus,
        ) -> CommonResult<EventPropagation> {
            throws_with_return!({
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
                                        AppSignal::AddPop(1),
                                    )
                                );
                            }
                            '-' => {
                                event_consumed = true;
                                // Note: make sure to wrap the call to `send` in a `tokio::spawn()` so
                                // that it doesn't block the calling thread. More info:
                                // <https://tokio.rs/tokio/tutorial/channels>.
                                send_signal!(
                                    global_data.main_thread_channel_sender,
                                    TerminalWindowMainThreadSignal::ApplyAction(
                                        AppSignal::SubPop(1),
                                    )
                                );
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
                                        AppSignal::AddPop(1),
                                    )
                                );
                            }
                            SpecialKey::Down => {
                                event_consumed = true;
                                send_signal!(
                                    global_data.main_thread_channel_sender,
                                    TerminalWindowMainThreadSignal::ApplyAction(
                                        AppSignal::SubPop(1),
                                    )
                                );
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

        fn render(
            &mut self,
            _global_data: &mut GlobalData<State, AppSignal>,
            current_box: FlexBox,
            _surface_bounds: SurfaceBounds, /* Ignore this. */
            has_focus: &mut HasFocus,
        ) -> CommonResult<RenderPipeline> {
            throws_with_return!({
                // Things from component scope.
                let ColumnComponentData { color_wheel, .. } = &mut self.data;

                // Fixed strings.
                let line_1 = format!("box.id:{} - Hello", current_box.id);
                let line_2 = format!("box.id:{} - World", current_box.id);

                // Setup intermediate vars.
                let box_origin_pos = current_box.style_adjusted_origin_pos; // Adjusted for style margin (if any).
                let box_bounds_size = current_box.style_adjusted_bounds_size; // Adjusted for style margin (if any).

                let mut row = ch!(0);
                let mut col = ch!(0);

                let mut render_ops = render_ops!();

                let line_1_us = UnicodeString::from(line_1);
                let line_1_trunc = line_1_us.truncate_to_fit_size(box_bounds_size);

                let line_2_us = UnicodeString::from(line_2);
                let line_2_trunc = line_2_us.truncate_to_fit_size(box_bounds_size);

                // Line 1.
                {
                    render_ops! {
                      @add_to render_ops
                      =>
                        RenderOp::ResetColor,
                        RenderOp::MoveCursorPositionRelTo(box_origin_pos, position!(col_index: col, row_index: row)),
                        RenderOp::ApplyColors(current_box.get_computed_style()),
                    };
                    render_ops! {
                        @render_styled_texts_into render_ops
                        =>
                        color_wheel.colorize_into_styled_texts(
                            &UnicodeString::from(line_1_trunc),
                            GradientGenerationPolicy::ReuseExistingGradientAndIndex,
                            TextColorizationPolicy::ColorEachCharacter(current_box.get_computed_style()),
                        )
                    }
                    render_ops += RenderOp::ResetColor;
                }

                // Line 2.
                {
                    row += 1;
                    render_ops! {
                      @add_to render_ops
                      =>
                        RenderOp::MoveCursorPositionRelTo(box_origin_pos, position!(col_index: col, row_index: row)),
                        RenderOp::ApplyColors(current_box.get_computed_style()),
                    };
                    render_ops! {
                        @render_styled_texts_into render_ops
                        =>
                        color_wheel.colorize_into_styled_texts(
                            &UnicodeString::from(line_2_trunc),
                            GradientGenerationPolicy::ReuseExistingGradientAndIndex,
                            TextColorizationPolicy::ColorEachCharacter(current_box.get_computed_style()),
                        )
                    }
                    render_ops += RenderOp::ResetColor;
                }

                // Paint is_focused.
                {
                    row += 1;
                    col = line_2_trunc.unicode_string().display_width / 2 - 1;
                    render_ops! {
                      @add_to render_ops
                      =>
                        RenderOp::ResetColor,
                        RenderOp::MoveCursorPositionRelTo(box_origin_pos, position!(col_index: col, row_index: row)),
                        if has_focus.does_current_box_have_focus(current_box) {
                          RenderOp::PaintTextWithAttributes("👀".into(), None)
                        }
                        else {
                          RenderOp::PaintTextWithAttributes(" ".into(), None)
                        }
                    };
                }

                // Add render_ops to pipeline.
                let mut pipeline = render_pipeline!();
                pipeline.push(ZOrder::Normal, render_ops);

                // Log pipeline.
                // 00: [x] clean up log
                call_if_true!(DEBUG_TUI_MOD, {
                    let message = format!(
                        "ColumnComponent::render {ch}",
                        ch = glyphs::RENDER_GLYPH
                    );
                    // % is Display, ? is Debug.
                    tracing::info!(
                        message = message,
                        current_box = ?current_box,
                        box_origin_pos = ?box_origin_pos,
                        box_bounds_size = ?box_bounds_size,
                        content_pos = ?position!(col_index: col, row_index: row),
                        render_pipeline = ?pipeline,
                    );
                });

                // Return the pipeline.
                pipeline
            });
        }
    }
}
