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

use async_trait::async_trait;
use r3bl_rs_utils_core::*;
use r3bl_tui::*;

use super::*;

#[derive(Debug, Clone, Default)]
pub struct ColumnRenderComponent {
    pub color_wheel: ColorWheel,
    pub id: FlexBoxId,
}

impl ColumnRenderComponent {
    pub fn new(id: FlexBoxId) -> Self {
        Self {
            id,
            color_wheel: ColorWheel::new(vec![
                ColorWheelConfig::RgbRandom(ColorWheelSpeed::Fast),
                ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::LightGreenToLightBlue,
                    ColorWheelSpeed::Fast,
                ),
            ]),
        }
    }
}

macro_rules! fire {
    (@add_pop => $arg_event_consumed: ident, $arg_shared_store: ident, $arg_action: expr) => {
        spawn_and_consume_event!($arg_event_consumed, $arg_shared_store, $arg_action);

        debug_log_action(
            "ColumnRenderComponent::handle_event".to_string(),
            $arg_action,
        );

        call_if_true!(DEBUG_TUI_MOD, {
            let msg = format!(
                "⛵ ColumnRenderComponent::handle_event -> + -> dispatch_spawn: {}",
                $arg_action
            );
            log_info(msg)
        });
    };

    (@sub_pop => $arg_event_consumed: ident, $arg_shared_store: ident, $arg_action: expr) => {
        spawn_and_consume_event!($arg_event_consumed, $arg_shared_store, $arg_action);
        call_if_true!(DEBUG_TUI_MOD, {
            let msg = format!(
                "⛵ ColumnRenderComponent::handle_event -> - -> dispatch_spawn: {}",
                $arg_action
            );
            log_info(msg)
        });
    };
}

#[async_trait]
impl Component<State, Action> for ColumnRenderComponent {
    fn reset(&mut self) {}

    fn get_id(&self) -> FlexBoxId { self.id }

    /// Handle following input events (and consume them):
    /// - Up,   `+` : fire `AddPop(1)`
    /// - Down, `-` : fire `SubPop(1)`
    async fn handle_event(
        &mut self,
        args: ComponentScopeArgs<'_, State, Action>,
        input_event: &InputEvent,
    ) -> CommonResult<EventPropagation> {
        throws_with_return!({
            let ComponentScopeArgs { shared_store, .. } = args;

            let mut event_consumed = false;

            if let InputEvent::Keyboard(KeyPress::Plain { key }) = input_event {
                // Check for + or - key.
                if let Key::Character(typed_char) = key {
                    match typed_char {
                        '+' => {
                            fire!(@add_pop => event_consumed, shared_store, Action::AddPop(1));
                        }
                        '-' => {
                            fire!(@sub_pop => event_consumed, shared_store, Action::SubPop(1));
                        }
                        _ => {}
                    }
                }

                // Check for up or down arrow key.
                if let Key::SpecialKey(special_key) = key {
                    match special_key {
                        SpecialKey::Up => {
                            fire!(@add_pop => event_consumed, shared_store, Action::AddPop(1));
                        }
                        SpecialKey::Down => {
                            fire!(@sub_pop => event_consumed, shared_store, Action::SubPop(1));
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

    async fn render(
        &mut self,
        args: ComponentScopeArgs<'_, State, Action>,
        current_box: &FlexBox,
        _surface_bounds: SurfaceBounds, /* Ignore this. */
    ) -> CommonResult<RenderPipeline> {
        throws_with_return!({
            let ComponentScopeArgs {
                component_registry, ..
            } = args;

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
                    self.color_wheel.colorize_into_styled_texts(
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
                    self.color_wheel.colorize_into_styled_texts(
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
                    if component_registry.has_focus.does_current_box_have_focus(current_box) {
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
            call_if_true!(DEBUG_TUI_MOD, {
                let msg = format!(
                    "\
                🦜 ColumnComponent::render ->
                  - current_box: {:?},
                  - box_origin_pos: {:?},
                  - box_bounds_size: {:?},
                  - content_pos: {:?},
                  - render_pipeline: {:?}",
                    current_box,
                    box_origin_pos,
                    box_bounds_size,
                    position!(col_index: col, row_index: row),
                    pipeline
                );
                log_info(msg);
            });

            // Return the pipeline.
            pipeline
        });
    }
}
