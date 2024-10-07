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
                position,
                throws,
                Ansi256GradientIndex,
                ColorWheel,
                ColorWheelConfig,
                ColorWheelSpeed,
                CommonResult,
                GradientGenerationPolicy,
                InputDevice,
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
    RawMode::start(global_data_ref.window_size);

    let app = &mut app;

    // This map is used to cache [Component]s that have been created and are meant to be reused between
    // multiple renders.
    // 1. It is entirely up to the [App] on how this [ComponentRegistryMap] is used.
    // 2. The methods provided allow components to be added to the map.
    let component_registry_map = &mut ComponentRegistryMap::default();
    let has_focus = &mut HasFocus::default();

    // Init the app, and perform first render.
    app.app_init(component_registry_map, has_focus);
    AppManager::render_app(app, global_data_ref, component_registry_map, has_focus)?;

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
                            RawMode::end(global_data_ref.window_size);
                            break;
                        },
                        TerminalWindowMainThreadSignal::Render(_) => {
                            AppManager::render_app(
                                app,
                                global_data_ref,
                                component_registry_map,
                                has_focus,
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
                            tracing::info!("main_event_loop -> Tick: ⏰ {input_event}");
                        }
                    });

                    handle_resize_if_applicable(input_event,
                        global_data_ref, app,
                        component_registry_map,
                        has_focus);

                    actually_process_input_event(
                        global_data_ref,
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
        tracing::info!("main_event_loop -> Shutdown 🛑");
    });

    ok!((global_data, input_device, output_device))
}

fn actually_process_input_event<S, AS>(
    global_data: &mut GlobalData<S, AS>,
    app: &mut BoxedSafeApp<S, AS>,
    input_event: InputEvent,
    exit_keys: &[InputEvent],
    component_registry_map: &mut ComponentRegistryMap<S, AS>,
    has_focus: &mut HasFocus,
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
) where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
{
    if let InputEvent::Resize(new_size) = input_event {
        global_data.set_size(new_size);
        global_data.maybe_saved_offscreen_buffer = None;
        let _ =
            AppManager::render_app(app, global_data, component_registry_map, has_focus);
    }
}

fn handle_result_generated_by_app_after_handling_action_or_input_event<S, AS>(
    result: CommonResult<EventPropagation>,
    maybe_input_event: Option<InputEvent>,
    exit_keys: &[InputEvent],
    app: &mut BoxedSafeApp<S, AS>,
    global_data: &mut GlobalData<S, AS>,
    component_registry_map: &mut ComponentRegistryMap<S, AS>,
    has_focus: &mut HasFocus,
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
                        tracing::error!("MySubscriber::render() error ❌: {error}");
                    });
                }
                Ok(render_pipeline) => {
                    render_pipeline.paint(FlushKind::ClearBeforeFlush, global_data);

                    telemetry_global_static::set_end_ts();

                    // Print debug message w/ memory utilization, etc.
                    call_if_true!(DEBUG_TUI_MOD, {
                        {
                            let state = &global_data.state;
                            tracing::info!("🎨 MySubscriber::paint() ok ✅: \n window_size: {window_size:?}\n state: {state:?}");
                            tracing::info!(
                                "🌍⏳ SPEED: {:?}",
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

// 00: add a test for main_event_loop_impl & return GlobalState
