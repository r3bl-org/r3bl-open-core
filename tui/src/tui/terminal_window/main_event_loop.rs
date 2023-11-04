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

use std::{fmt::Debug, sync::Arc};

use async_trait::async_trait;
use get_size::GetSize;
use r3bl_redux::*;
use r3bl_rs_utils_core::*;
use r3bl_rs_utils_macro::*;
use tokio::sync::{mpsc, RwLock};

use crate::*;

pub struct TerminalWindow;

#[derive(Debug)]
pub enum ParallelExecutionPolicy {
    /// Run all tasks in parallel.
    Parallel,
    /// Run all tasks in sequence.
    Serial,
}

/// This value should be set to [ParallelExecutionPolicy::Serial] unless you have a very good reason
/// otherwise.
///
/// # Issues w/ parallel processing of input events
/// The event loop runs as a task in a tokio executor (runtime). It processes input events
/// asynchronously. So you can easily spawn parallel tasks to process each input event as it comes
/// in. This will result in a state consistency issue which you can easily observe this if you set
/// this value to [ParallelExecutionPolicy::Parallel] and introduce a delay in any event handling of
/// a [Component]. In parallel, each task gets a copy of the [r3bl_redux::Store] and it gets a copy
/// of the state and mutates it. If 2 tasks are running in parallel and they both mutate the same
/// starting state, then this will result in an undefined state. Eg: you press the right arrow twice
/// in an editor component (which has a huge delay). Then it will look like only one right arrow
/// action was dispatched.
///
/// # Deep dive into how HID / pointing devices work in Linux
/// <https://monroeclinton.com/pointing-devices-in-linux/>
pub const PARALLEL_EXECUTION_POLICY: ParallelExecutionPolicy =
    ParallelExecutionPolicy::Serial;

impl TerminalWindow {
    /// The where clause needs to match up w/ the trait bounds for [Store].
    ///
    /// ```ignore
    /// where
    /// S: Debug + Default + Clone + PartialEq + Sync + Send,
    /// A: Debug + Default + Clone + Sync + Send,
    /// ```
    pub async fn main_event_loop<S, A>(
        shared_app: SharedApp<S, A>,
        store: Store<S, A>,
        exit_keys: Vec<InputEvent>,
    ) -> CommonResult<()>
    where
        S: Debug + Default + Clone + PartialEq + Sync + Send + 'static,
        A: Debug + Default + Clone + Sync + Send + 'static,
    {
        // Initialize the terminal window data struct.
        let _global_data = GlobalData::try_to_create_instance()?;
        let shared_global_data: SharedGlobalData = Arc::new(RwLock::new(_global_data));

        // Start raw mode.
        RawMode::start(&shared_global_data).await;

        // Move the store into an Arc & RwLock.
        let shared_store: SharedStore<S, A> = Arc::new(RwLock::new(store));

        // Create a subscriber (AppManager) & attach it to the store.
        let _subscriber =
            AppManager::new_box(&shared_app, &shared_store, &shared_global_data);
        shared_store.write().await.add_subscriber(_subscriber).await;

        // Create a new event stream (async).
        let mut async_event_stream = AsyncEventStream::default();

        // Perform first render.
        AppManager::render_app(&shared_store, &shared_app, &shared_global_data, None)
            .await?;

        shared_global_data
            .read()
            .await
            .dump_to_log("main_event_loop -> Startup 🚀");

        // mpsc channel to send exit signal to main loop.
        let (exit_channel_sender, mut exit_channel_receiver) = mpsc::channel::<bool>(1);

        // Main event loop.
        loop {
            tokio::select! {
                // Handle exit channel.
                result = exit_channel_receiver.recv() => {
                    if result.is_some() {
                        RawMode::end(&shared_global_data).await;
                        break;
                    }
                }

                // Handle input event.
                maybe_input_event = async_event_stream.try_to_get_input_event() => {
                    if let Some(input_event) = maybe_input_event {
                        telemetry_global_static::set_start_ts();

                        call_if_true!(DEBUG_TUI_MOD, {
                            let msg = format!("main_event_loop -> Tick: ⏰ {input_event}");
                            log_info(msg);
                        });

                        Self::handle_resize_event(&input_event, &shared_global_data, &shared_store, &shared_app).await;

                        match PARALLEL_EXECUTION_POLICY {
                            ParallelExecutionPolicy::Parallel => {
                                let shared_global_data = shared_global_data.clone();
                                let shared_store = shared_store.clone();
                                let shared_app = shared_app.clone();
                                let input_event = input_event.clone();
                                let exit_keys = exit_keys.clone();
                                let exit_channel_sender = exit_channel_sender.clone();
                                tokio::spawn(async move {
                                    // This function does not block the caller, since it
                                    // [spawns](tokio::spawn) a task in parallel to handle the
                                    // `input_event` by passing it to the app.
                                    Self::actually_process_input_event(
                                        shared_global_data,
                                        shared_store,
                                        shared_app,
                                        input_event,
                                        exit_keys,
                                        exit_channel_sender,
                                    ).await;
                                });
                            }
                            ParallelExecutionPolicy::Serial => {
                                Self::actually_process_input_event(
                                    shared_global_data.clone(),
                                    shared_store.clone(),
                                    shared_app.clone(),
                                    input_event.clone(),
                                    exit_keys.clone(),
                                    exit_channel_sender.clone(),
                                ).await;
                            }
                        }
                    }
                }
            }
        } // End loop.

        Ok(())
    }

    async fn actually_process_input_event<S, A>(
        shared_global_data: SharedGlobalData,
        shared_store: SharedStore<S, A>,
        shared_app: SharedApp<S, A>,
        input_event: InputEvent,
        exit_keys: Vec<InputEvent>,
        exit_channel_sender: mpsc::Sender<bool>,
    ) where
        S: Debug + Default + Clone + PartialEq + Sync + Send + 'static,
        A: Debug + Default + Clone + Sync + Send + 'static,
    {
        let result = AppManager::route_input_to_app(
            shared_global_data.clone(),
            shared_store.clone(),
            shared_app.clone(),
            input_event.clone(),
        )
        .await;

        call_if_true!(DEBUG_TUI_MOD, {
            let msg = format!(
                "main_event_loop -> 🚥 SPAWN propagation_result_from_app: {result:?}"
            );
            log_info(msg);
        });

        if let Ok(event_propagation) = result {
            match event_propagation {
                EventPropagation::Propagate => {
                    let check_if_exit_keys_pressed = DefaultInputEventHandler::no_consume(
                        input_event.clone(),
                        &exit_keys,
                    );
                    if let Continuation::Exit = check_if_exit_keys_pressed.await {
                        // Exit keys were pressed.
                        let _ = exit_channel_sender.send(true).await;
                    };
                }
                EventPropagation::ConsumedRender => {
                    let _ = AppManager::render_app(
                        &shared_store,
                        &shared_app,
                        &shared_global_data,
                        None,
                    )
                    .await;
                }
                EventPropagation::Consumed => {}
                EventPropagation::ExitMainEventLoop => {
                    // Exit the main event loop.
                    let _ = exit_channel_sender.send(true).await;
                }
            }
        }
    }

    /// Before any app gets to process the `input_event`, perform special handling in case it is a
    /// resize event.
    pub async fn handle_resize_event<S, A>(
        input_event: &InputEvent,
        shared_global_data: &SharedGlobalData,
        shared_store: &SharedStore<S, A>,
        shared_app: &SharedApp<S, A>,
    ) where
        S: Debug + Default + Clone + PartialEq + Sync + Send + 'static,
        A: Debug + Default + Clone + Sync + Send + 'static,
    {
        if let InputEvent::Resize(new_size) = input_event {
            shared_global_data.write().await.set_size(*new_size);
            shared_global_data
                .write()
                .await
                .maybe_saved_offscreen_buffer = None;
            let _ = AppManager::render_app(
                shared_store,
                shared_app,
                shared_global_data,
                None,
            )
            .await;
        }
    }
}

struct AppManager<S, A>
where
    S: Debug + Default + Clone + PartialEq + Sync + Send + 'static,
    A: Debug + Default + Clone + Sync + Send + 'static,
{
    shared_app: SharedApp<S, A>,
    shared_store: SharedStore<S, A>,
    shared_global_data: SharedGlobalData,
}

#[async_trait]
impl<S, A> AsyncSubscriber<S> for AppManager<S, A>
where
    S: Debug + Default + Clone + PartialEq + Sync + Send + 'static,
    A: Debug + Default + Clone + Sync + Send,
{
    async fn run(&self, my_state: S) {
        let result = AppManager::render_app(
            &self.shared_store,
            &self.shared_app,
            &self.shared_global_data,
            my_state.into(),
        )
        .await;
        if let Err(e) = result {
            call_if_true!(DEBUG_TUI_MOD, {
                let msg = format!("MySubscriber::run -> Error: {e}");
                log_error(msg);
            })
        }
    }
}

impl<S, A> AppManager<S, A>
where
    S: Debug + Default + Clone + PartialEq + Sync + Send + 'static,
    A: Debug + Default + Clone + Sync + Send,
{
    fn new_box(
        shared_app: &SharedApp<S, A>,
        shared_store: &SharedStore<S, A>,
        shared_global_data: &SharedGlobalData,
    ) -> Box<Self> {
        Box::new(AppManager {
            shared_app: shared_app.clone(),
            shared_store: shared_store.clone(),
            shared_global_data: shared_global_data.clone(),
        })
    }

    /// Pass the event to the `shared_app` for further processing.
    pub async fn route_input_to_app(
        shared_global_data: SharedGlobalData,
        shared_store: SharedStore<S, A>,
        shared_app: SharedApp<S, A>,
        input_event: InputEvent,
    ) -> CommonResult<EventPropagation> {
        throws_with_return!({
            // Create global scope args.
            let state = shared_store.read().await.get_state();
            let window_size = shared_global_data.read().await.get_size();
            let global_scope_args = GlobalScopeArgs {
                shared_global_data: &shared_global_data,
                shared_store: &shared_store,
                state: &state,
                window_size: &window_size,
            };

            // Call app_handle_event.
            shared_app
                .write()
                .await
                .app_handle_event(global_scope_args, &input_event)
                .await?
        });
    }

    pub async fn render_app(
        shared_store: &SharedStore<S, A>,
        shared_app: &SharedApp<S, A>,
        shared_global_data: &SharedGlobalData,
        maybe_state: Option<S>,
    ) -> CommonResult<()> {
        throws!({
            // Create global scope args.
            let window_size = shared_global_data.read().await.get_size();
            let state: S = if let Some(state) = maybe_state {
                state
            } else {
                shared_store.read().await.get_state()
            };
            let global_scope_args = GlobalScopeArgs {
                state: &state,
                shared_store,
                shared_global_data,
                window_size: &window_size,
            };

            // Check to see if the window_size is large enough to render.
            let render_result: CommonResult<RenderPipeline> = if window_size
                .is_too_small_to_display(MinSize::Col as u8, MinSize::Row as u8)
            {
                shared_global_data
                    .write()
                    .await
                    .maybe_saved_offscreen_buffer = None;
                Ok(render_window_size_too_small(window_size))
            } else {
                // Call app_render.
                shared_app.write().await.app_render(global_scope_args).await
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
                    render_pipeline
                        .paint(FlushKind::ClearBeforeFlush, shared_global_data)
                        .await;

                    telemetry_global_static::set_end_ts();

                    // Print debug message w/ memory utilization, etc.
                    call_if_true!(DEBUG_TUI_MOD, {
                        {
                            let msg_1 = format!("🎨 MySubscriber::paint() ok ✅: \n window_size: {window_size:?}\n state: {state:?}");
                            let msg_2 = {
                                format!(
                                    "🌍⏳ SPEED: {:?}",
                                    telemetry_global_static::get_avg_response_time_micros(
                                    ),
                                )
                            };

                            if let Some(ref offscreen_buffer) = &shared_global_data
                                .read()
                                .await
                                .maybe_saved_offscreen_buffer
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

fn render_window_size_too_small(window_size: Size) -> RenderPipeline {
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
