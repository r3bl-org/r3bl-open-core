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

use std::{fmt::Debug, marker::PhantomData};

use r3bl_core::{call_if_true,
                ch,
                col,
                format_as_kilobytes_with_commas,
                glyphs,
                height,
                ok,
                output_device_as_mut,
                row,
                string_storage,
                telemetry::{telemetry_default_constants, Telemetry},
                telemetry_record,
                width,
                Ansi256GradientIndex,
                ColorWheel,
                ColorWheelConfig,
                ColorWheelSpeed,
                CommonResult,
                Dim,
                GCStringExt as _,
                GradientGenerationPolicy,
                InputDevice,
                LockedOutputDevice,
                OutputDevice,
                SufficientSize,
                TelemetryAtomHint,
                TextColorizationPolicy};
use r3bl_macro::tui_style;
use size_of::SizeOf as _;
use smallvec::smallvec;
use tokio::sync::mpsc;

use super::{BoxedSafeApp, Continuation, DefaultInputEventHandler, EventPropagation};
use crate::{render_pipeline,
            ComponentRegistryMap,
            Flush as _,
            FlushKind,
            GlobalData,
            HasFocus,
            InputDeviceExt,
            InputEvent,
            MinSize,
            RawMode,
            RenderOp,
            RenderPipeline,
            TerminalWindowMainThreadSignal,
            ZOrder,
            DEBUG_TUI_MOD,
            DISPLAY_LOG_TELEMETRY};

pub const CHANNEL_WIDTH: usize = 1_000;

/// Don't record response times that are smaller than this amount. This removes a lot of
/// noise from the telemetry data.
pub const FILTER_LOWEST_RESPONSE_TIME_MIN_MICROS: i64 = 100;

pub async fn main_event_loop_impl<S, AS>(
    mut app: BoxedSafeApp<S, AS>,
    exit_keys: &[InputEvent],
    state: S,
    initial_size: Dim,
    mut input_device: InputDevice,
    output_device: OutputDevice,
) -> CommonResult<(
    /* global_data */ GlobalData<S, AS>,
    /* event stream */ InputDevice,
    /* stdout */ OutputDevice,
)>
where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send + 'static,
{
    // mpsc channel to send signals from the app to the main event loop (eg: for exit,
    // re-render, apply action, etc).
    let (main_thread_channel_sender, mut main_thread_channel_receiver) =
        mpsc::channel::<TerminalWindowMainThreadSignal<AS>>(CHANNEL_WIDTH);

    // Initialize the terminal window data struct.
    let mut global_data = GlobalData::try_to_create_instance(
        main_thread_channel_sender.clone(),
        state,
        initial_size,
        output_device.clone(),
    )?;
    let global_data_mut_ref = &mut global_data;

    // Start raw mode.
    RawMode::start(
        global_data_mut_ref.window_size,
        output_device_as_mut!(output_device),
        output_device.is_mock,
    );

    let app = &mut app;

    // This map is used to cache [Component]s that have been created and are meant to be reused between
    // multiple renders.
    // 1. It is entirely up to the [App] on how this [ComponentRegistryMap] is used.
    // 2. The methods provided allow components to be added to the map.
    let component_registry_map = &mut ComponentRegistryMap::default();
    let has_focus = &mut HasFocus::default();

    // Init telemetry recording explicitly (without using the simple constructor).
    let mut telemetry_alt: Telemetry<{ telemetry_default_constants::RING_BUFFER_SIZE }> =
        Telemetry::new((
            telemetry_default_constants::RATE_LIMIT_TIME_THRESHOLD,
            telemetry_default_constants::FILTER_MIN_RESPONSE_TIME,
        ));

    // Init the app, and perform first render.
    telemetry_record!(@telemetry: telemetry_alt, @hint: TelemetryAtomHint::Render, {
        app.app_init(component_registry_map, has_focus);
        AppManager::render_app(
            app,
            global_data_mut_ref,
            component_registry_map,
            has_focus,
            output_device_as_mut!(output_device),
            output_device.is_mock,
        )?;
    });

    call_if_true!(DISPLAY_LOG_TELEMETRY || DEBUG_TUI_MOD, {
        let message = format!(
            "main_event_loop {sp} Startup complete {ch}",
            sp = glyphs::RIGHT_ARROW_GLYPH,
            ch = glyphs::CELEBRATE_GLYPH
        );
        // % is Display, ? is Debug.
        tracing::info!(
            message = message,
            global_data_mut_ref=?global_data_mut_ref
        );
    });

    // Main event loop.
    loop {
        tokio::select! {
            // This branch is cancel safe since recv is cancel safe.
            // Handle signals on the channel.
            maybe_signal = main_thread_channel_receiver.recv() => {
                if let Some(ref signal) = maybe_signal {
                    match signal {
                        TerminalWindowMainThreadSignal::Exit => {
                            // 🐒 Actually exit the main loop!
                            RawMode::end(
                                global_data_mut_ref.window_size,
                                output_device_as_mut!(output_device),
                                output_device.is_mock,
                            );
                            break;
                        },
                        TerminalWindowMainThreadSignal::Render(_) => {
                            telemetry_record!(@telemetry: telemetry_alt, @hint: TelemetryAtomHint::Render, {
                                AppManager::render_app(
                                    app,
                                    global_data_mut_ref,
                                    component_registry_map,
                                    has_focus,
                                    output_device_as_mut!(output_device),
                                    output_device.is_mock,
                                )?;
                            });
                        },
                        TerminalWindowMainThreadSignal::ApplyAppSignal(action) => {
                            telemetry_record!(@telemetry: telemetry_alt, @hint: TelemetryAtomHint::Signal, {
                                let result = app.app_handle_signal(action, global_data_mut_ref, component_registry_map, has_focus);
                                handle_result_generated_by_app_after_handling_action_or_input_event(
                                    result,
                                    None,
                                    exit_keys,
                                    app,
                                    global_data_mut_ref,
                                    component_registry_map,
                                    has_focus,
                                    output_device_as_mut!(output_device),
                                    output_device.is_mock,
                                );
                            });
                        },
                    }
                }
            }

            // Handle input event.
            // This branch is cancel safe because no state is declared inside the
            // future in the following block.
            // - All the state comes from other variables (self.*).
            // - So if this future is dropped, then the item in the
            //   pinned_input_stream isn't used and the state isn't modified.
            maybe_input_event = input_device.next_input_event() => {
                if let Some(input_event) = maybe_input_event {
                    call_if_true!(DISPLAY_LOG_TELEMETRY || DEBUG_TUI_MOD, {
                        if let InputEvent::Keyboard(_)= input_event {
                            let message = format!(
                                "main_event_loop {sp} Tick {ch}",
                                sp = glyphs::RIGHT_ARROW_GLYPH,
                                ch = glyphs::CLOCK_TICK_GLYPH
                            );
                            // % is Display, ? is Debug.
                            tracing::info!(
                                message = message,
                                input_event = ?input_event
                            );
                        }
                    });

                    // Handle resize event here. And then pass it to the app (next).
                    if let InputEvent::Resize(new_size) = input_event {
                        telemetry_record!(@telemetry: telemetry_alt, @hint: TelemetryAtomHint::Resize, {
                            handle_resize(
                                new_size,
                                global_data_mut_ref, app,
                                component_registry_map,
                                has_focus,
                                output_device_as_mut!(output_device),
                                output_device.is_mock,
                            );
                        });
                    }

                    // This includes resize events.
                    telemetry_record!(@telemetry: telemetry_alt, @hint: TelemetryAtomHint::Input, {
                        actually_process_input_event(
                            global_data_mut_ref,
                            app,
                            input_event,
                            exit_keys,
                            component_registry_map,
                            has_focus,
                            output_device_as_mut!(output_device),
                            output_device.is_mock,
                        );
                    });
                } else {
                    // environments with InputDevice::new_mock_with_delay() or
                    // There are no events in the stream, so exit. This happens in test
                    // InputDevice::new_mock().
                    break;
                }
            }
        }

        // Output telemetry report to log.
        call_if_true!(DISPLAY_LOG_TELEMETRY || DEBUG_TUI_MOD, {
            {
                let state = &global_data_mut_ref.state;
                let message =
                    format!("AppManager::render_app() ok {ch}", ch = glyphs::PAINT_GLYPH);
                // % is Display, ? is Debug.
                tracing::info!(
                    message = message,
                    window_size = ?global_data_mut_ref.window_size,
                    state = ?state,
                    report = %telemetry_alt.report()?,
                );

                if let Some(ref offscreen_buffer) =
                    global_data_mut_ref.maybe_saved_offscreen_buffer
                {
                    let message = format!(
                        "AppManager::render_app() offscreen_buffer stats {ch}",
                        ch = glyphs::SCREEN_BUFFER_GLYPH
                    );
                    // % is Display, ? is Debug.
                    tracing::info!(
                        message = message,
                        offscreen_buffer.size = format!(
                            "Memory used: {size}",
                            size = format_as_kilobytes_with_commas(
                                offscreen_buffer.size_of().total_bytes()
                            )
                        )
                    );
                }
            }
        });
    } // End main event loop.

    call_if_true!(DISPLAY_LOG_TELEMETRY || DEBUG_TUI_MOD, {
        let message = format!(
            "main_event_loop {sp} Shutdown {ch}",
            ch = glyphs::BYE_GLYPH,
            sp = glyphs::RIGHT_ARROW_GLYPH,
        );
        // % is Display, ? is Debug.
        tracing::info!(
            message = message,
            session_duration = %telemetry_alt.session_duration()
        );
    });

    ok!((global_data, input_device, output_device))
}

/// **Telemetry**: This function is not recorded in telemetry but its caller is.
#[allow(clippy::too_many_arguments)]
fn actually_process_input_event<S, AS>(
    global_data_mut_ref: &mut GlobalData<S, AS>,
    app: &mut BoxedSafeApp<S, AS>,
    input_event: InputEvent,
    exit_keys: &[InputEvent],
    component_registry_map: &mut ComponentRegistryMap<S, AS>,
    has_focus: &mut HasFocus,
    locked_output_device: LockedOutputDevice<'_>,
    is_mock: bool,
) where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send + 'static,
{
    let result = app.app_handle_input_event(
        input_event,
        global_data_mut_ref,
        component_registry_map,
        has_focus,
    );

    handle_result_generated_by_app_after_handling_action_or_input_event(
        result,
        Some(input_event),
        exit_keys,
        app,
        global_data_mut_ref,
        component_registry_map,
        has_focus,
        locked_output_device,
        is_mock,
    );
}

/// **Telemetry**: This function is not recorded in telemetry but its caller is.
///
/// This function gets called as a result of:
/// 1. Terminal resize event.
pub fn handle_resize<S, AS>(
    new_size: Dim,
    global_data_mut_ref: &mut GlobalData<S, AS>,
    app: &mut BoxedSafeApp<S, AS>,
    component_registry_map: &mut ComponentRegistryMap<S, AS>,
    has_focus: &mut HasFocus,
    locked_output_device: LockedOutputDevice<'_>,
    is_mock: bool,
) where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
{
    global_data_mut_ref.set_size(new_size);
    global_data_mut_ref.maybe_saved_offscreen_buffer = None;
    let _ = AppManager::render_app(
        app,
        global_data_mut_ref,
        component_registry_map,
        has_focus,
        locked_output_device,
        is_mock,
    );
}

/// **Telemetry**: This function is not recorded in telemetry but its caller is.
///
/// This function gets called as a result of:
/// 1. Input event from the user.
/// 2. Signal from the app.
#[allow(clippy::too_many_arguments)]
fn handle_result_generated_by_app_after_handling_action_or_input_event<S, AS>(
    result: CommonResult<EventPropagation>,
    maybe_input_event: Option<InputEvent>,
    exit_keys: &[InputEvent],
    app: &mut BoxedSafeApp<S, AS>,
    global_data_mut_ref: &mut GlobalData<S, AS>,
    component_registry_map: &mut ComponentRegistryMap<S, AS>,
    has_focus: &mut HasFocus,
    locked_output_device: LockedOutputDevice<'_>,
    is_mock: bool,
) where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send + 'static,
{
    let main_thread_channel_sender =
        global_data_mut_ref.main_thread_channel_sender.clone();

    match result {
        Ok(event_propagation) => match event_propagation {
            EventPropagation::Propagate => {
                if let Some(input_event) = maybe_input_event {
                    let check_if_exit_keys_pressed =
                        DefaultInputEventHandler::no_consume(input_event, exit_keys);
                    if let Continuation::Exit = check_if_exit_keys_pressed {
                        request_exit_by_sending_signal(main_thread_channel_sender);
                    };
                }
            }

            EventPropagation::ConsumedRender => {
                let _ = AppManager::render_app(
                    app,
                    global_data_mut_ref,
                    component_registry_map,
                    has_focus,
                    locked_output_device,
                    is_mock,
                );
            }

            EventPropagation::Consumed => {}

            EventPropagation::ExitMainEventLoop => {
                request_exit_by_sending_signal(main_thread_channel_sender);
            }
        },
        Err(error) => {
            let message = format!(
                "main_event_loop {sp} handle_result_generated_by_app_after_handling_action {ch}",
                ch = glyphs::SUSPICIOUS_GLYPH,
                sp = glyphs::RIGHT_ARROW_GLYPH,
            );
            // % is Display, ? is Debug.
            tracing::error!(
                message = message,
                error =? error
            );
        }
    }
}

fn request_exit_by_sending_signal<AS>(
    channel_sender: mpsc::Sender<TerminalWindowMainThreadSignal<AS>>,
) where
    AS: Debug + Default + Clone + Sync + Send + 'static,
{
    // Exit keys were pressed.
    // Note: make sure to wrap the call to `send` in a `tokio::spawn()` so that it doesn't
    // block the calling thread. More info: <https://tokio.rs/tokio/tutorial/channels>.
    tokio::spawn(async move {
        let _ = channel_sender
            .send(TerminalWindowMainThreadSignal::Exit)
            .await;
    });
}

struct AppManager<S, AS>
where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
{
    _phantom: PhantomData<(S, AS)>,
}

impl<S, AS> AppManager<S, AS>
where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
{
    /// **Telemetry**: This function is not recorded in telemetry but its caller(s) are.
    pub fn render_app(
        app: &mut BoxedSafeApp<S, AS>,
        global_data_mut_ref: &mut GlobalData<S, AS>,
        component_registry_map: &mut ComponentRegistryMap<S, AS>,
        has_focus: &mut HasFocus,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) -> CommonResult<()> {
        let window_size = global_data_mut_ref.window_size;

        // Check to see if the window_size is large enough to render.
        let render_result = match window_size
            .fits_min_size(width(MinSize::Col as u8) + height(MinSize::Row as u8))
        {
            SufficientSize::IsLargeEnough => {
                app.app_render(global_data_mut_ref, component_registry_map, has_focus)
            }
            SufficientSize::IsTooSmall => {
                global_data_mut_ref.maybe_saved_offscreen_buffer = None;
                Ok(render_window_too_small_error(window_size))
            }
        };

        match render_result {
            Err(error) => {
                RenderOp::default().flush(locked_output_device);

                // Print debug message w/ error.
                call_if_true!(DEBUG_TUI_MOD, {
                    let message = format!(
                        "AppManager::render_app() error {ch}",
                        ch = glyphs::SUSPICIOUS_GLYPH
                    );
                    // % is Display, ? is Debug.
                    tracing::error!(
                        message = message,
                        details = ?error
                    );
                });
            }

            Ok(render_pipeline) => {
                render_pipeline.paint(
                    FlushKind::ClearBeforeFlush,
                    global_data_mut_ref,
                    locked_output_device,
                    is_mock,
                );
            }
        }

        ok!()
    }
}

fn render_window_too_small_error(window_size: Dim) -> RenderPipeline {
    // Show warning message that window_size is too small.
    let msg = string_storage!(
        "Window size is too small. Minimum size is {} cols x {} rows",
        MinSize::Col as u8,
        MinSize::Row as u8
    );
    let msg_gcs = msg.grapheme_string();
    let trunc_msg = msg_gcs.trunc_end_to_fit(window_size);

    let trunc_msg_gcs = trunc_msg.grapheme_string();
    let trunc_msg_width = trunc_msg_gcs.display_width;

    let row_pos = row({
        let it = window_size.row_height / ch(2);
        *it
    });
    let col_pos = col({
        let it = (window_size.col_width - trunc_msg_width) / ch(2);
        *it
    });

    let mut pipeline = render_pipeline!();

    let style_bold = tui_style!(attrib: [bold]);

    render_pipeline! {
        @push_into pipeline
        at ZOrder::Normal
        =>
            RenderOp::ResetColor,
            // RenderOp::MoveCursorPositionAbs(position! {col_index: col_pos, row_index: row_pos})
            RenderOp::MoveCursorPositionAbs(col_pos + row_pos)
    }

    render_pipeline! {
        @push_styled_texts_into pipeline
        at ZOrder::Normal
        =>
            ColorWheel::new(smallvec![
                ColorWheelConfig::RgbRandom(ColorWheelSpeed::Fast),
                ColorWheelConfig::Ansi256(Ansi256GradientIndex::DarkRedToDarkMagenta, ColorWheelSpeed::Medium),
            ])
                .colorize_into_styled_texts(
                    &trunc_msg_gcs,
                    GradientGenerationPolicy::RegenerateGradientAndIndexBasedOnTextLength,
                    TextColorizationPolicy::ColorEachCharacter(Some(style_bold)),
                )
    }

    pipeline
}

#[cfg(test)]
mod tests {
    use std::{fmt::{Debug, Formatter},
              time::Duration};

    use r3bl_ansi_color::{is_fully_uninteractive_terminal, TTYResult};
    use r3bl_core::{assert_eq2,
                    ch,
                    col,
                    color,
                    defaults::get_default_gradient_stops,
                    height,
                    ok,
                    send_signal,
                    string_storage,
                    throws_with_return,
                    tui_styled_text,
                    tui_styled_texts,
                    width,
                    ColorWheel,
                    ColorWheelConfig,
                    ColorWheelSpeed,
                    CommonResult,
                    CrosstermEventResult,
                    Dim,
                    GradientGenerationPolicy,
                    GradientLengthKind,
                    InputDevice,
                    OutputDevice,
                    TextColorizationPolicy,
                    TuiStyle,
                    VecArray};
    use r3bl_macro::tui_style;
    use r3bl_test_fixtures::{output_device_ext::OutputDeviceExt as _, InputDeviceExt};
    use smallvec::smallvec;
    use test_fixture_app::AppMainTest;
    use test_fixture_state::{AppSignal, State};

    use crate::{keypress,
                main_event_loop_impl,
                render_ops,
                render_pipeline,
                render_tui_styled_texts_into,
                App,
                ComponentRegistryMap,
                EventPropagation,
                GlobalData,
                HasFocus,
                InputEvent,
                Key,
                KeyPress,
                PixelChar,
                RenderOp,
                RenderPipeline,
                SpecialKey,
                TerminalWindowMainThreadSignal,
                ZOrder};

    #[tokio::test]
    #[allow(clippy::needless_return)]
    async fn test_main_event_loop_impl() -> CommonResult<()> {
        // Enable tracing to debug this test.
        // let _guard = TracingConfig {
        //     writer_config: tracing_logging::WriterConfig::Display(
        //         DisplayPreference::Stdout,
        //     ),
        //     level_filter: tracing::Level::DEBUG.into(),
        // }
        // .install_thread_local()?;

        // Create an App (renders & responds to user input).
        let app = Box::<AppMainTest>::default();

        // Exit if these keys are pressed.
        let exit_keys: VecArray<InputEvent> =
            smallvec![InputEvent::Keyboard(keypress! { @char 'x' })];

        // Simulated key inputs.
        let generator_vec: VecArray<CrosstermEventResult> = smallvec![
            Ok(crossterm::event::Event::Key(
                crossterm::event::KeyEvent::new(
                    crossterm::event::KeyCode::Up,
                    crossterm::event::KeyModifiers::empty(),
                ),
            )),
            Ok(crossterm::event::Event::Key(
                crossterm::event::KeyEvent::new(
                    crossterm::event::KeyCode::Up,
                    crossterm::event::KeyModifiers::empty(),
                ),
            )),
            Ok(crossterm::event::Event::Key(
                crossterm::event::KeyEvent::new(
                    crossterm::event::KeyCode::Char('x'),
                    crossterm::event::KeyModifiers::empty(),
                ),
            )),
        ];

        // Create a window.
        let initial_size = width(65) + height(11);
        let input_device =
            InputDevice::new_mock_with_delay(generator_vec, Duration::from_millis(10));
        let (output_device, stdout_mock) = OutputDevice::new_mock();
        let state = State::default();

        let (global_data, _, _) = main_event_loop_impl(
            app,
            &exit_keys,
            state,
            initial_size,
            input_device,
            output_device,
        )
        .await?;

        // Make assertions.

        // console_log!(global_data.state);
        // console_log!(stdout_mock.get_copy_of_buffer_as_string_strip_ansi());

        assert_eq!(global_data.state.counter, 2);
        assert!(stdout_mock
            .get_copy_of_buffer_as_string_strip_ansi()
            .contains("State{counter:2}"));

        // println!(
        //     "global_data.offscreen_buffer: {:?}",
        //     global_data.maybe_saved_offscreen_buffer
        // );

        let my_offscreen_buffer = global_data.maybe_saved_offscreen_buffer.unwrap();

        // This is for CI/CD environment. It does not support truecolor, and degrades to ANSI 256 colors
        if let TTYResult::IsNotInteractive = is_fully_uninteractive_terminal() {
            // Check pixel char at 4 x 7.
            {
                let PixelChar::PlainText {
                    text,
                    maybe_style: _,
                } = my_offscreen_buffer.buffer[4][7].clone()
                else {
                    panic!(
                        "Expected PixelChar::PlainText, got: {:?}",
                        my_offscreen_buffer.buffer[4][7]
                    );
                };
                assert_eq2!(text, "S");
            }

            // Check pixel char at 10 x 7.
            {
                let PixelChar::PlainText {
                    text,
                    maybe_style: _,
                } = my_offscreen_buffer.buffer[10][7].clone()
                else {
                    panic!(
                        "Expected PixelChar::PlainText, got: {:?}",
                        my_offscreen_buffer.buffer[10][7]
                    );
                };
                assert_eq2!(text, "H");
            }
        }
        // This is for local development environment. It supports truecolor.
        else {
            // Check pixel char at 4 x 7.
            {
                assert_eq2!(
                    PixelChar::PlainText {
                        text: "S".into(),
                        maybe_style: Some(TuiStyle {
                            color_fg: Some(color!(102, 0, 255)),
                            ..Default::default()
                        }),
                    },
                    my_offscreen_buffer.buffer[4][7].clone()
                );
            }

            // Check pixel char at 10 x 7.
            {
                assert_eq2!(
                    PixelChar::PlainText {
                        text: "H".into(),
                        maybe_style: Some(TuiStyle {
                            id: u8::MAX,
                            dim: true,
                            ..Default::default()
                        }),
                    },
                    my_offscreen_buffer.buffer[10][7].clone()
                );
            }
        }

        ok!()
    }

    mod test_fixture_state {
        use super::*;

        /// Action.
        #[derive(Default, Clone, Debug)]
        #[non_exhaustive]
        #[allow(dead_code)]
        pub enum AppSignal {
            Add,
            Sub,
            Clear,
            #[default]
            Noop,
        }

        /// State.
        #[derive(Clone, PartialEq, Eq, Default)]
        pub struct State {
            pub counter: isize,
        }

        impl Debug for State {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "State {{ counter: {:?} }}", self.counter)
            }
        }
    }

    mod test_fixture_app {
        use super::*;

        #[derive(Default)]
        pub struct AppMainTest {
            pub data: AppDataTest,
        }

        #[derive(Default)]
        pub struct AppDataTest {
            pub color_wheel_rgb: ColorWheel,
        }
    }

    mod test_fixture_app_main_impl_trait_app {
        use r3bl_core::{row, GCStringExt as _, Pos};

        use super::*;

        impl App for AppMainTest {
            type S = State;
            type AS = AppSignal;

            fn app_render(
                &mut self,
                global_data_mut_ref: &mut GlobalData<State, AppSignal>,
                _component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
                _has_focus: &mut HasFocus,
            ) -> CommonResult<RenderPipeline> {
                throws_with_return!({
                    let state_string =
                        string_storage!("{a:?}", a = global_data_mut_ref.state);
                    let data = &mut self.data;

                    let sample_line_of_text =
                        format!("{state_string}, gradient: [index: X, len: Y]");
                    let content_size_col = width(sample_line_of_text.len());
                    let window_size = global_data_mut_ref.window_size;

                    let col_idx = col({
                        let it = window_size.col_width - content_size_col;
                        *it / ch(2)
                    });
                    let mut row_idx = row({
                        let it = window_size.row_height - height(2);
                        *it / ch(2)
                    });

                    let mut pipeline = render_pipeline!();

                    pipeline.push(ZOrder::Normal, {
                        let mut acc_render_op = render_ops! {
                            @new
                            RenderOp::ResetColor,
                        };

                        // Render using color_wheel_rgb.
                        acc_render_op += RenderOp::MoveCursorPositionAbs(Pos {
                            col_index: col_idx,
                            row_index: row_idx,
                        });

                        let index = data.color_wheel_rgb.get_index();
                        let len = match data.color_wheel_rgb.get_gradient_len() {
                            GradientLengthKind::ColorWheel(len) => len,
                            _ => 0,
                        };

                        let string = string_storage!(
                            "{state_string}, gradient: [index: {a:?}, len: {b}]",
                            a = index,
                            b = len
                        );

                        let string_gcs = string.grapheme_string();

                        render_ops!(
                            @render_styled_texts_into acc_render_op
                            =>
                            data.color_wheel_rgb.colorize_into_styled_texts(
                                &string_gcs,
                                GradientGenerationPolicy::ReuseExistingGradientAndIndex,
                                TextColorizationPolicy::ColorEachWord(None),
                            )
                        );

                        *row_idx += 1;

                        acc_render_op
                    });

                    text_fixture_status_bar::create_status_bar_message(
                        &mut pipeline,
                        window_size,
                    );

                    pipeline
                });
            }

            fn app_handle_input_event(
                &mut self,
                input_event: InputEvent,
                global_data_mut_ref: &mut GlobalData<State, AppSignal>,
                _component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
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
                                        global_data_mut_ref.main_thread_channel_sender,
                                        TerminalWindowMainThreadSignal::ApplyAppSignal(
                                            AppSignal::Add,
                                        )
                                    );
                                }
                                '-' => {
                                    event_consumed = true;
                                    send_signal!(
                                        global_data_mut_ref.main_thread_channel_sender,
                                        TerminalWindowMainThreadSignal::ApplyAppSignal(
                                            AppSignal::Sub,
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
                                        global_data_mut_ref.main_thread_channel_sender,
                                        TerminalWindowMainThreadSignal::ApplyAppSignal(
                                            AppSignal::Add,
                                        )
                                    );
                                }
                                SpecialKey::Down => {
                                    event_consumed = true;
                                    send_signal!(
                                        global_data_mut_ref.main_thread_channel_sender,
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
                global_data_mut_ref: &mut GlobalData<State, AppSignal>,
                _component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
                _has_focus: &mut HasFocus,
            ) -> CommonResult<EventPropagation> {
                throws_with_return!({
                    let GlobalData { state, .. } = global_data_mut_ref;

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

                data.color_wheel_rgb = ColorWheel::new(smallvec![ColorWheelConfig::Rgb(
                    get_default_gradient_stops(),
                    ColorWheelSpeed::Fast,
                    25,
                )]);
            }
        }
    }

    mod text_fixture_status_bar {
        use super::*;

        /// Shows helpful messages at the bottom row of the screen.
        pub fn create_status_bar_message(pipeline: &mut RenderPipeline, size: Dim) {
            let styled_texts = tui_styled_texts! {
                tui_styled_text!{ @style: tui_style!(attrib: [dim])       , @text: "Hints:"},
                tui_styled_text!{ @style: tui_style!(attrib: [bold])      , @text: " x : Exit 🖖 "},
                tui_styled_text!{ @style: tui_style!(attrib: [dim])       , @text: " … "},
                tui_styled_text!{ @style: tui_style!(attrib: [underline]) , @text: " ↑ / + : inc "},
                tui_styled_text!{ @style: tui_style!(attrib: [dim])       , @text: " … "},
                tui_styled_text!{ @style: tui_style!(attrib: [underline]) , @text: " ↓ / - : dec "},
            };

            let display_width = styled_texts.display_width();
            let col_center = *(size.col_width - display_width) / ch(2);
            let row_bottom = size.row_height.convert_to_row_index();
            let center = col(col_center) + row_bottom;

            let mut render_ops = render_ops!();
            render_ops.push(RenderOp::MoveCursorPositionAbs(center));
            render_tui_styled_texts_into(&styled_texts, &mut render_ops);
            pipeline.push(ZOrder::Normal, render_ops);
        }
    }
}
