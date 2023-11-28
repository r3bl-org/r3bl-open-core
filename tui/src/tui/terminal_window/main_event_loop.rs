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

use get_size::GetSize;
use r3bl_rs_utils_core::*;
use r3bl_rs_utils_macro::*;
use tokio::sync::mpsc;

use crate::*;

pub struct TerminalWindow;

pub const CHANNEL_WIDTH: usize = 1_000;

#[derive(Debug)]
pub enum TerminalWindowMainThreadSignal<A>
where
    A: Debug + Default + Clone + Sync + Send,
{
    /// Exit the main event loop.
    Exit,
    /// Render the app.
    Render(Option<FlexBoxId>),
    /// Apply an action to the app.
    ApplyAction(A),
}

impl TerminalWindow {
    /// This is the main event loop for the entire application. It is responsible for
    /// handling all input events, and dispatching them to the [App] for processing. It is
    /// also responsible for rendering the [App] after each input event. It is also
    /// responsible for handling all signals sent from the [App] to the main event loop
    /// (eg: exit, re-render, apply action, etc).
    pub async fn main_event_loop<S, A>(
        mut app: BoxedSafeApp<S, A>,
        exit_keys: Vec<InputEvent>,
    ) -> CommonResult<()>
    where
        S: Debug + Default + Clone + Sync + Send,
        A: Debug + Default + Clone + Sync + Send + 'static,
    {
        throws!({
            // mpsc channel to send signals from the app to the main event loop (eg: for exit,
            // re-render, apply action, etc).
            let (main_thread_channel_sender, mut main_thread_channel_receiver) =
                mpsc::channel::<TerminalWindowMainThreadSignal<A>>(CHANNEL_WIDTH);

            // Initialize the terminal window data struct.
            let mut global_data = &mut GlobalData::try_to_create_instance(
                main_thread_channel_sender.clone(),
            )?;

            // Start raw mode.
            RawMode::start(global_data.window_size);

            // Create a new event stream (async).
            let async_event_stream = &mut AsyncEventStream::default();

            let app = &mut app;

            // This map is used to cache [Component]s that have been created and are meant to be reused between
            // multiple renders.
            // 1. It is entirely up to the [App] on how this [ComponentRegistryMap] is used.
            // 2. The methods provided allow components to be added to the map.
            let mut component_registry_map = &mut ComponentRegistryMap::default();
            let mut has_focus = &mut HasFocus::default();

            // Init the app, and perform first render.
            app.app_init(&mut component_registry_map, &mut has_focus);
            AppManager::render_app(
                app,
                &mut global_data,
                component_registry_map,
                has_focus,
            )?;

            global_data.dump_to_log("main_event_loop -> Startup 🚀");

            // Main event loop.
            loop {
                tokio::select! {
                    // Handle signals on the channel.
                    maybe_signal = main_thread_channel_receiver.recv() => {
                        if let Some(ref signal) = maybe_signal {
                            match signal {
                                TerminalWindowMainThreadSignal::Exit => {
                                    // 🐒 Actually exit the main loop!
                                    RawMode::end(global_data.window_size);
                                    break;
                                },
                                TerminalWindowMainThreadSignal::Render(_) => {
                                    AppManager::render_app(
                                        app,
                                        global_data,
                                        component_registry_map,
                                        has_focus,
                                    )?;
                                },
                                TerminalWindowMainThreadSignal::ApplyAction(action) => {
                                    let result = app.app_handle_signal(action, &mut global_data)?;
                                    handle_result_generated_by_app_after_handling_action_or_input_event(
                                        Ok(result),
                                        None,
                                        &exit_keys,
                                        app,
                                        &mut global_data,
                                        &mut component_registry_map,
                                        &mut has_focus,
                                    );
                                },
                            }
                        }
                    }

                    // Handle input event.
                    maybe_input_event = AsyncEventStream::try_to_get_input_event(async_event_stream) => {
                        if let Some(input_event) = maybe_input_event {
                            telemetry_global_static::set_start_ts();

                            call_if_true!(DEBUG_TUI_MOD, {
                                match input_event {
                                    InputEvent::Keyboard(_) => {
                                        let msg = format!("main_event_loop -> Tick: ⏰ {input_event}");
                                        log_info(msg);
                                    }
                                    _ => {}
                                }
                            });

                            Self::handle_resize_if_applicable(input_event,
                                &mut global_data, app,
                                component_registry_map,
                                has_focus);

                            Self::actually_process_input_event(
                                &mut global_data,
                                app,
                                input_event,
                                &exit_keys,
                                component_registry_map,
                                has_focus,
                            );
                        }
                    }
                }
            } // End loop.

            call_if_true!(DEBUG_TUI_MOD, {
                let msg = format!("\nmain_event_loop -> Shutdown 🛑");
                log_info(msg);
            });
        });
    }

    fn actually_process_input_event<S, A>(
        global_data: &mut GlobalData<S, A>,
        app: &mut BoxedSafeApp<S, A>,
        input_event: InputEvent,
        exit_keys: &[InputEvent],
        component_registry_map: &mut ComponentRegistryMap<S, A>,
        has_focus: &mut HasFocus,
    ) where
        S: Debug + Default + Clone + Sync + Send,
        A: Debug + Default + Clone + Sync + Send + 'static,
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
            &exit_keys,
            app,
            global_data,
            component_registry_map,
            has_focus,
        );
    }

    /// Before any app gets to process the `input_event`, perform special handling in case
    /// it is a resize event.
    pub fn handle_resize_if_applicable<S, A>(
        input_event: InputEvent,
        global_data: &mut GlobalData<S, A>,
        app: &mut BoxedSafeApp<S, A>,
        component_registry_map: &mut ComponentRegistryMap<S, A>,
        has_focus: &mut HasFocus,
    ) where
        S: Debug + Default + Clone + Sync + Send,
        A: Debug + Default + Clone + Sync + Send,
    {
        if let InputEvent::Resize(new_size) = input_event {
            global_data.set_size(new_size);
            global_data.maybe_saved_offscreen_buffer = None;
            let _ = AppManager::render_app(
                app,
                global_data,
                component_registry_map,
                has_focus,
            );
        }
    }
}

fn handle_result_generated_by_app_after_handling_action_or_input_event<S, A>(
    result: CommonResult<EventPropagation>,
    maybe_input_event: Option<InputEvent>,
    exit_keys: &[InputEvent],
    app: &mut BoxedSafeApp<S, A>,
    global_data: &mut GlobalData<S, A>,
    component_registry_map: &mut ComponentRegistryMap<S, A>,
    has_focus: &mut HasFocus,
) where
    S: Debug + Default + Clone + Sync + Send,
    A: Debug + Default + Clone + Sync + Send + 'static,
{
    let main_thread_channel_sender = global_data.main_thread_channel_sender.clone();

    if let Ok(event_propagation) = result {
        match event_propagation {
            EventPropagation::Propagate => {
                if let Some(input_event) = maybe_input_event {
                    let check_if_exit_keys_pressed = DefaultInputEventHandler::no_consume(
                        input_event.clone(),
                        &exit_keys,
                    );
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
                );
            }

            EventPropagation::Consumed => {}

            EventPropagation::ExitMainEventLoop => {
                request_exit_by_sending_signal(main_thread_channel_sender);
            }
        }
    }
}

fn request_exit_by_sending_signal<A: 'static>(
    channel_sender: mpsc::Sender<TerminalWindowMainThreadSignal<A>>,
) where
    A: Debug + Default + Clone + Sync + Send,
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

struct AppManager<S, A>
where
    S: Debug + Default + Clone + Sync + Send,
    A: Debug + Default + Clone + Sync + Send,
{
    _phantom: PhantomData<(S, A)>,
}

impl<S, A> AppManager<S, A>
where
    S: Debug + Default + Clone + Sync + Send,
    A: Debug + Default + Clone + Sync + Send,
{
    pub fn render_app(
        app: &mut BoxedSafeApp<S, A>,
        global_data: &mut GlobalData<S, A>,
        component_registry_map: &mut ComponentRegistryMap<S, A>,
        has_focus: &mut HasFocus,
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
                    RenderOp::default().flush();

                    telemetry_global_static::set_end_ts();

                    call_if_true!(DEBUG_TUI_MOD, {
                        let msg = format!("MySubscriber::render() error ❌: {error}");
                        log_error(msg);
                    });
                }
                Ok(render_pipeline) => {
                    render_pipeline.paint(FlushKind::ClearBeforeFlush, global_data);

                    telemetry_global_static::set_end_ts();

                    // Print debug message w/ memory utilization, etc.
                    call_if_true!(DEBUG_TUI_MOD, {
                        {
                            let state = &global_data.state;
                            let msg_1 = format!("🎨 MySubscriber::paint() ok ✅: \n window_size: {window_size:?}\n state: {state:?}");
                            let msg_2 = {
                                format!(
                                    "🌍⏳ SPEED: {:?}",
                                    telemetry_global_static::get_avg_response_time_micros(
                                    ),
                                )
                            };

                            if let Some(ref offscreen_buffer) =
                                global_data.maybe_saved_offscreen_buffer
                            {
                                let msg_3 = format!(
                                    "offscreen_buffer: {0:.2}kb",
                                    offscreen_buffer.get_size() as f32 / 1000_f32
                                );
                                log_info(format!("{msg_1}\n{msg_2}, {msg_3}"));
                            } else {
                                log_info(format!("{msg_1}\n{msg_2}"));
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

    let style_bold = style!(attrib: [bold]);

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
