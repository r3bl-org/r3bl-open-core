// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.
use super::{AppSignal, State};
use r3bl_tui::{Ansi256GradientIndex, BoxedSafeComponent, ColorWheel, ColorWheelConfig,
               ColorWheelSpeed, CommonResult, Component, DEBUG_TUI_MOD,
               EventPropagation, FlexBox, FlexBoxId, GCStringOwned, GlobalData,
               GradientGenerationPolicy, HasFocus, InputEvent, Key, KeyPress,
               RenderOpCommon, RenderOpIR, RenderOpsIR, RenderPipeline, SpecialKey,
               SurfaceBounds, TerminalWindowMainThreadSignal, TextColorizationPolicy,
               ZOrder, ch, col, glyphs, inline_string, render_pipeline, row, send_signal,
               throws_with_return};
use smallvec::smallvec;

#[derive(Debug, Clone, Default)]
pub struct SingleColumnComponent {
    pub data: SingleColumnComponentData,
}

#[derive(Debug, Clone, Default)]
pub struct SingleColumnComponentData {
    pub color_wheel: ColorWheel,
    pub id: FlexBoxId,
}

mod constructor {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl SingleColumnComponent {
        pub fn new_boxed(id: FlexBoxId) -> BoxedSafeComponent<State, AppSignal> {
            let it = Self {
                data: SingleColumnComponentData {
                    id,
                    color_wheel: ColorWheel::new(smallvec![
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

mod single_column_component_impl_component_trait {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Component<State, AppSignal> for SingleColumnComponent {
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
                                    TerminalWindowMainThreadSignal::ApplyAppSignal(
                                        AppSignal::AddPop(1),
                                    )
                                );
                            }
                            '-' => {
                                event_consumed = true;
                                send_signal!(
                                    global_data.main_thread_channel_sender,
                                    TerminalWindowMainThreadSignal::ApplyAppSignal(
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
                                    TerminalWindowMainThreadSignal::ApplyAppSignal(
                                        AppSignal::AddPop(1),
                                    )
                                );
                            }
                            SpecialKey::Down => {
                                event_consumed = true;
                                send_signal!(
                                    global_data.main_thread_channel_sender,
                                    TerminalWindowMainThreadSignal::ApplyAppSignal(
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
                let SingleColumnComponentData { color_wheel, .. } = &mut self.data;

                // Fixed strings.
                let line_1 = inline_string!("box.id: {a:?} - Hello", a = current_box.id);
                let line_2 = inline_string!("box.id: {b:?} - World", b = current_box.id);

                // Setup intermediate vars.
                let box_origin_pos = current_box.style_adjusted_origin_pos; // Adjusted for style margin (if any).
                let box_bounds_size = current_box.style_adjusted_bounds_size; // Adjusted for style margin (if any).
                let mut content_cursor_pos = col(0) + row(0);

                let mut render_ops = RenderOpsIR::new();

                // Line 1.
                {
                    let line_1_gcs = GCStringOwned::from(line_1);
                    let line_1_trunc = line_1_gcs.trunc_end_to_fit(box_bounds_size);

                    render_ops.push(RenderOpIR::Common(RenderOpCommon::MoveCursorPositionRelTo(box_origin_pos, content_cursor_pos)));
                    render_ops.push(RenderOpIR::Common(RenderOpCommon::ApplyColors(current_box.get_computed_style())));
                    render_ops.push(RenderOpIR::PaintTextWithAttributes(
                        line_1_trunc.into(),
                        current_box.get_computed_style(),
                    ));
                    render_ops.push(RenderOpIR::Common(RenderOpCommon::ResetColor));
                }

                // Line 2.
                {
                    let line_2_gcs = GCStringOwned::from(line_2);
                    let line_2_trunc_str = line_2_gcs.trunc_end_to_fit(box_bounds_size);
                    let line_2_trunc_gcs = GCStringOwned::from(line_2_trunc_str);
                    content_cursor_pos
                        .add_row_with_bounds(ch(1), box_bounds_size.row_height);

                    render_ops.push(RenderOpIR::Common(RenderOpCommon::MoveCursorPositionRelTo(
                        box_origin_pos,
                        content_cursor_pos,
                    )));
                    render_ops.push(RenderOpIR::Common(RenderOpCommon::ApplyColors(current_box.get_computed_style())));

                    let styled_texts = color_wheel.colorize_into_styled_texts(
                        &line_2_trunc_gcs,
                        GradientGenerationPolicy::ReuseExistingGradientAndIndex,
                        TextColorizationPolicy::ColorEachCharacter(current_box.get_computed_style()),
                    );
                    r3bl_tui::render_tui_styled_texts_into(&styled_texts, &mut render_ops);

                    render_ops.push(RenderOpIR::Common(RenderOpCommon::ResetColor));
                }

                // Paint is_focused.
                content_cursor_pos.add_row_with_bounds(ch(1), box_bounds_size.row_height);
                render_ops.push(RenderOpIR::Common(RenderOpCommon::MoveCursorPositionRelTo(
                    box_origin_pos,
                    content_cursor_pos,
                )));
                if has_focus.does_current_box_have_focus(current_box) {
                    render_ops.push(RenderOpIR::PaintTextWithAttributes("👀".into(), None));
                } else {
                    render_ops.push(RenderOpIR::PaintTextWithAttributes(" ".into(), None));
                }

                // Add render_ops to pipeline.
                let mut pipeline = render_pipeline!();
                pipeline.push(ZOrder::Normal, render_ops);

                // Log pipeline.
                DEBUG_TUI_MOD.then(|| {
                    // % is Display, ? is Debug.
                    tracing::info!(
                        message = %inline_string!(
                            "ColumnComponent::render {ch}",
                            ch = glyphs::RENDER_GLYPH
                        ),
                        current_box = ?current_box,
                        box_origin_pos = ?box_origin_pos,
                        box_bounds_size = ?box_bounds_size,
                        content_pos = ?content_cursor_pos,
                        render_pipeline = ?pipeline,
                    );
                });

                // Return the pipeline.
                pipeline
            });
        }
    }
}
