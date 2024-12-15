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

use std::{fmt::Debug, marker::PhantomData};

use r3bl_core::{call_if_true,
                ch,
                ok,
                output_device_as_mut,
                position,
                throws,
                Ansi256GradientIndex,
                ColorWheel,
                ColorWheelConfig,
                ColorWheelSpeed,
                CommonResult,
                GradientGenerationPolicy,
                InputDevice,
                LockedOutputDevice,
                OutputDevice,
                Size,
                TextColorizationPolicy,
                TooSmallToDisplayResult,
                UnicodeString};
use r3bl_macro::tui_style;
use size_of::SizeOf as _;
use tokio::sync::mpsc;

use super::{BoxedSafeApp, Continuation, DefaultInputEventHandler, EventPropagation};
use crate::{render_pipeline,
            telemetry_global_static,
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
            DEBUG_TUI_MOD};

pub const CHANNEL_WIDTH: usize = 1_000;

pub async fn main_event_loop_impl<S, AS>(
    mut app: BoxedSafeApp<S, AS>,
    exit_keys: Vec<InputEvent>,
    state: S,
    initial_size: Size,
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
    let global_data_ref = &mut global_data;

    // Start raw mode.
    RawMode::start(
        global_data_ref.window_size,
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

    // Init the app, and perform first render.
    app.app_init(component_registry_map, has_focus);
    AppManager::render_app(
        app,
        global_data_ref,
        component_registry_map,
        has_focus,
        output_device_as_mut!(output_device),
        output_device.is_mock,
    )?;

    global_data_ref.dump_to_log("main_event_loop -> Startup 🚀");

    // Main event loop.
    loop {
        tokio::select! {
            // Handle signals on the channel.
            // This branch is cancel safe since recv is cancel safe.
            maybe_signal = main_thread_channel_receiver.recv() => {
                if let Some(ref signal) = maybe_signal {
                    match signal {
                        TerminalWindowMainThreadSignal::Exit => {
                            // 🐒 Actually exit the main loop!
                            RawMode::end(
                                global_data_ref.window_size,
                                output_device_as_mut!(output_device),
                                output_device.is_mock,
                            );
                            break;
                        },
                        TerminalWindowMainThreadSignal::Render(_) => {
                            AppManager::render_app(
                                app,
                                global_data_ref,
                                component_registry_map,
                                has_focus,
                                output_device_as_mut!(output_device),
                                output_device.is_mock,
                            )?;
                        },
                        TerminalWindowMainThreadSignal::ApplyAction(action) => {
                            let result = app.app_handle_signal(action, global_data_ref, component_registry_map, has_focus);
                            handle_result_generated_by_app_after_handling_action_or_input_event(
                                result,
                                None,
                                &exit_keys,
                                app,
                                global_data_ref,
                                component_registry_map,
                                has_focus,
                                output_device_as_mut!(output_device),
                                output_device.is_mock,
                            );
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
                    telemetry_global_static::set_start_ts();

                    call_if_true!(DEBUG_TUI_MOD, {
                        if let InputEvent::Keyboard(_)= input_event {
                            tracing::info!("main_event_loop -> Tick: 🌄 {input_event}");
                        }
                    });

                    handle_resize_if_applicable(input_event,
                        global_data_ref, app,
                        component_registry_map,
                        has_focus,
                        output_device_as_mut!(output_device),
                        output_device.is_mock,
                    );

                    actually_process_input_event(
                        global_data_ref,
                        app,
                        input_event,
                        &exit_keys,
                        component_registry_map,
                        has_focus,
                        output_device_as_mut!(output_device),
                        output_device.is_mock,
                    );
                } else {
                    // There are no events in the stream, so exit. This happens in test
                    // environments with InputDevice::new_mock_with_delay() or
                    // InputDevice::new_mock().
                    break;
                }
            }
        }
    } // End loop.

    call_if_true!(DEBUG_TUI_MOD, {
        tracing::info!("main_event_loop -> Shutdown 🛑");
    });

    ok!((global_data, input_device, output_device))
}

#[allow(clippy::too_many_arguments)]
fn actually_process_input_event<S, AS>(
    global_data: &mut GlobalData<S, AS>,
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
        global_data,
        component_registry_map,
        has_focus,
    );

    handle_result_generated_by_app_after_handling_action_or_input_event(
        result,
        Some(input_event),
        exit_keys,
        app,
        global_data,
        component_registry_map,
        has_focus,
        locked_output_device,
        is_mock,
    );
}

/// Before any app gets to process the `input_event`, perform special handling in case
/// it is a resize event.
pub fn handle_resize_if_applicable<S, AS>(
    input_event: InputEvent,
    global_data: &mut GlobalData<S, AS>,
    app: &mut BoxedSafeApp<S, AS>,
    component_registry_map: &mut ComponentRegistryMap<S, AS>,
    has_focus: &mut HasFocus,
    locked_output_device: LockedOutputDevice<'_>,
    is_mock: bool,
) where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
{
    if let InputEvent::Resize(new_size) = input_event {
        global_data.set_size(new_size);
        global_data.maybe_saved_offscreen_buffer = None;
        let _ = AppManager::render_app(
            app,
            global_data,
            component_registry_map,
            has_focus,
            locked_output_device,
            is_mock,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_result_generated_by_app_after_handling_action_or_input_event<S, AS>(
    result: CommonResult<EventPropagation>,
    maybe_input_event: Option<InputEvent>,
    exit_keys: &[InputEvent],
    app: &mut BoxedSafeApp<S, AS>,
    global_data: &mut GlobalData<S, AS>,
    component_registry_map: &mut ComponentRegistryMap<S, AS>,
    has_focus: &mut HasFocus,
    locked_output_device: LockedOutputDevice<'_>,
    is_mock: bool,
) where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send + 'static,
{
    let main_thread_channel_sender = global_data.main_thread_channel_sender.clone();

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
                    global_data,
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
            tracing::error!("main_event_loop -> handle_result_generated_by_app_after_handling_action. Error: {error}");
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
    pub fn render_app(
        app: &mut BoxedSafeApp<S, AS>,
        global_data: &mut GlobalData<S, AS>,
        component_registry_map: &mut ComponentRegistryMap<S, AS>,
        has_focus: &mut HasFocus,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) -> CommonResult<()> {
        throws!({
            let window_size = global_data.window_size;

            // Check to see if the window_size is large enough to render.
            let render_result =
                match window_size.fits_min_size(MinSize::Col as u8, MinSize::Row as u8) {
                    TooSmallToDisplayResult::IsLargeEnough => {
                        app.app_render(global_data, component_registry_map, has_focus)
                    }
                    TooSmallToDisplayResult::IsTooSmall => {
                        global_data.maybe_saved_offscreen_buffer = None;
                        Ok(render_window_too_small_error(window_size))
                    }
                };

            match render_result {
                Err(error) => {
                    RenderOp::default().flush(locked_output_device);

                    telemetry_global_static::set_end_ts();

                    call_if_true!(DEBUG_TUI_MOD, {
                        tracing::error!("MySubscriber::render() error ❌: {error}");
                    });
                }
                Ok(render_pipeline) => {
                    render_pipeline.paint(
                        FlushKind::ClearBeforeFlush,
                        global_data,
                        locked_output_device,
                        is_mock,
                    );

                    telemetry_global_static::set_end_ts();

                    // Print debug message w/ memory utilization, etc.
                    call_if_true!(DEBUG_TUI_MOD, {
                        {
                            let state = &global_data.state;
                            tracing::info!("🎨 MySubscriber::paint() ok 🟢: \n window_size: {window_size:?}\n state: {state:?}");
                            tracing::info!(
                                "🏁 SPEED: {:?}",
                                telemetry_global_static::get_avg_response_time_micros(),
                            );

                            if let Some(ref offscreen_buffer) =
                                global_data.maybe_saved_offscreen_buffer
                            {
                                tracing::info!(
                                    "offscreen_buffer: {0:.3} kb",
                                    offscreen_buffer.size_of().total_bytes() as f64
                                        / 1000_f64
                                );
                            }
                        }
                    });
                }
            }
        });
    }
}

fn render_window_too_small_error(window_size: Size) -> RenderPipeline {
    // Show warning message that window_size is too small.
    let display_msg = UnicodeString::from(format!(
        "Window size is too small. Minimum size is {} cols x {} rows",
        MinSize::Col as u8,
        MinSize::Row as u8
    ));
    let trunc_display_msg =
        UnicodeString::from(display_msg.truncate_to_fit_size(window_size));
    let trunc_display_msg_len = ch!(trunc_display_msg.len());

    let row_pos = window_size.row_count / 2;
    let col_pos = (window_size.col_count - trunc_display_msg_len) / 2;

    let mut pipeline = render_pipeline!();

    let style_bold = tui_style!(attrib: [bold]);

    render_pipeline! {
        @push_into pipeline
        at ZOrder::Normal
        =>
            RenderOp::ResetColor,
            RenderOp::MoveCursorPositionAbs(position! {col_index: col_pos, row_index: row_pos})
    }

    render_pipeline! {
        @push_styled_texts_into pipeline
        at ZOrder::Normal
        =>
            ColorWheel::new(vec![
                ColorWheelConfig::RgbRandom(ColorWheelSpeed::Fast),
                ColorWheelConfig::Ansi256(Ansi256GradientIndex::DarkRedToDarkMagenta, ColorWheelSpeed::Medium),
            ])
                .colorize_into_styled_texts(
                    &trunc_display_msg,
                    GradientGenerationPolicy::RegenerateGradientAndIndexBasedOnTextLength,
                    TextColorizationPolicy::ColorEachCharacter(Some(style_bold)),
                )
    }

    pipeline
}

#[cfg(test)]
mod tests {
    use std::{fmt::{Display, Formatter},
              time::Duration};

    use position::Position;
    use r3bl_ansi_color::{is_fully_uninteractive_terminal, TTYResult};
    use r3bl_core::{assert_eq2,
                    ch,
                    color,
                    ok,
                    position,
                    send_signal,
                    size,
                    throws_with_return,
                    tui_styled_text,
                    tui_styled_texts,
                    ChUnit,
                    ColorWheel,
                    ColorWheelConfig,
                    ColorWheelSpeed,
                    CommonResult,
                    CrosstermEventResult,
                    GradientGenerationPolicy,
                    GradientLengthKind,
                    GraphemeClusterSegment,
                    InputDevice,
                    OutputDevice,
                    TextColorizationPolicy,
                    TuiStyle,
                    UnicodeString,
                    DEFAULT_GRADIENT_STOPS};
    use r3bl_macro::tui_style;
    use r3bl_test_fixtures::{output_device_ext::OutputDeviceExt as _, InputDeviceExt};
    use size::Size;
    use state::{AppSignal, State};

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
        let app = Box::<AppMain>::default();

        // Exit if these keys are pressed.
        let exit_keys: Vec<InputEvent> =
            vec![InputEvent::Keyboard(keypress! { @char 'x' })];

        // Simulated key inputs.
        let generator_vec: Vec<CrosstermEventResult> = vec![
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
        let initial_size = size!(col_count: 65, row_count: 11);
        let input_device =
            InputDevice::new_mock_with_delay(generator_vec, Duration::from_millis(10));
        let (output_device, stdout_mock) = OutputDevice::new_mock();
        let state = State::default();

        let (global_data, _, _) = main_event_loop_impl(
            app,
            exit_keys,
            state,
            initial_size,
            input_device,
            output_device,
        )
        .await?;

        // Make assertions.

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
                    content,
                    maybe_style: _,
                } = my_offscreen_buffer.buffer[4][7].clone()
                else {
                    panic!(
                        "Expected PixelChar::PlainText, got: {:?}",
                        my_offscreen_buffer.buffer[4][7]
                    );
                };
                assert_eq2!(content, GraphemeClusterSegment::from("S"));
            }

            // Check pixel char at 10 x 7.
            {
                let PixelChar::PlainText {
                    content,
                    maybe_style: _,
                } = my_offscreen_buffer.buffer[10][7].clone()
                else {
                    panic!(
                        "Expected PixelChar::PlainText, got: {:?}",
                        my_offscreen_buffer.buffer[10][7]
                    );
                };
                assert_eq2!(content, GraphemeClusterSegment::from("H"));
            }
        }
        // This is for local development environment. It supports truecolor.
        else {
            // Check pixel char at 4 x 7.
            {
                assert_eq2!(
                    PixelChar::PlainText {
                        content: GraphemeClusterSegment::from("S"),
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
                        content: GraphemeClusterSegment::from("H"),
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

    mod state {
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

        impl Display for AppSignal {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{self:?}")
            }
        }

        /// State.
        #[derive(Clone, PartialEq, Eq, Debug, Default)]
        pub struct State {
            pub counter: isize,
        }

        impl Display for State {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "State {{ counter: {:?} }}", self.counter)
            }
        }
    }

    #[derive(Default)]
    pub struct AppMain {
        pub data: AppData,
    }

    #[derive(Default)]
    pub struct AppData {
        pub color_wheel_rgb: ColorWheel,
    }

    mod app_main_impl_trait_app {
        use super::*;

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
                    let mut row = (window_size.row_count - ch!(2)) / 2;

                    let mut pipeline = render_pipeline!();

                    pipeline.push(ZOrder::Normal, {
                        let mut it = render_ops! {
                            @new
                            RenderOp::ResetColor,
                        };

                        // Render using color_wheel_rgb.
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

                        render_ops!(
                            @render_styled_texts_into it
                            =>
                            data.color_wheel_rgb.colorize_into_styled_texts(
                                &unicode_string,
                                GradientGenerationPolicy::ReuseExistingGradientAndIndex,
                                TextColorizationPolicy::ColorEachWord(None),
                            )
                        );

                        row += 1;

                        it
                    });

                    status_bar::create_status_bar_message(&mut pipeline, window_size);

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

                data.color_wheel_rgb = ColorWheel::new(vec![ColorWheelConfig::Rgb(
                    Vec::from(DEFAULT_GRADIENT_STOPS.map(String::from)),
                    ColorWheelSpeed::Fast,
                    25,
                )]);
            }
        }
    }

    mod status_bar {
        use super::*;

        /// Shows helpful messages at the bottom row of the screen.
        pub fn create_status_bar_message(pipeline: &mut RenderPipeline, size: Size) {
            let styled_texts = tui_styled_texts! {
                tui_styled_text!{ @style: tui_style!(attrib: [dim])       , @text: "Hints:"},
                tui_styled_text!{ @style: tui_style!(attrib: [bold])      , @text: " x : Exit 🖖 "},
                tui_styled_text!{ @style: tui_style!(attrib: [dim])       , @text: " … "},
                tui_styled_text!{ @style: tui_style!(attrib: [underline]) , @text: " ↑ / + : inc "},
                tui_styled_text!{ @style: tui_style!(attrib: [dim])       , @text: " … "},
                tui_styled_text!{ @style: tui_style!(attrib: [underline]) , @text: " ↓ / - : dec "},
            };

            let display_width = styled_texts.display_width();
            let col_center: ChUnit = (size.col_count - display_width) / 2;
            let row_bottom: ChUnit = size.row_count - 1;
            let center: Position =
                position!(col_index: col_center, row_index: row_bottom);

            let mut render_ops = render_ops!();
            render_ops.push(RenderOp::MoveCursorPositionAbs(center));
            render_tui_styled_texts_into(&styled_texts, &mut render_ops);
            pipeline.push(ZOrder::Normal, render_ops);
        }
    }
}
