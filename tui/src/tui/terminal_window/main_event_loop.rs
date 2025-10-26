// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.
use super::{BoxedSafeApp, Continuation, DefaultInputEventHandler, EventPropagation,
            MainEventLoopFuture};
use crate::{Ansi256GradientIndex, ColorWheel, ColorWheelConfig, ColorWheelSpeed,
            CommonResult, ComponentRegistryMap, DEBUG_TUI_MOD, DISPLAY_LOG_TELEMETRY,
            DefaultSize, DefaultTiming, Flush, FlushKind, GCStringOwned, GetMemSize,
            GlobalData, GradientGenerationPolicy, HasFocus, InputDevice, InputDeviceExt,
            InputEvent, LockedOutputDevice, MinSize, OffscreenBufferPool, OutputDevice,
            RawMode, RenderOpCommon, RenderOpIR, RenderPipeline, Size, SufficientSize,
            TelemetryAtomHint, TerminalWindowMainThreadSignal, TextColorizationPolicy,
            ZOrder, ch, col, glyphs, height, inline_string, lock_output_device_as_mut,
            new_style, ok, render_pipeline, row,
            telemetry::{Telemetry, telemetry_default_constants},
            telemetry_record, width};
use smallvec::smallvec;
use std::{fmt::{Debug, Display},
          marker::PhantomData};
use tokio::sync::mpsc;

// XMARK: Box::pin a future that is larger than 16KB.

/// Main event loop implementation that handles terminal UI events and app state
/// management.
///
/// This is the internal API, and not the public API
/// [`super::TerminalWindow::main_event_loop()`]. This separation exists to allow for
/// testing using dependency injection.
///
/// This function takes pre-initialized components (terminal size, input/output devices)
/// and runs the actual async event loop. It handles all input events, dispatches them
/// to the [`crate::App`] for processing, renders the app after each event, and manages
/// all signals sent from the app to the main event loop.
///
/// # Arguments
///
/// * `app` - The [`BoxedSafeApp`] instance that will handle input events and signals.
/// * `exit_keys` - A slice of [`InputEvent`]s that will trigger application exit.
/// * `state` - The initial application state.
/// * `initial_size` - The initial terminal size.
/// * `input_device` - The [`InputDevice`] for reading input events.
/// * `output_device` - The [`OutputDevice`] for writing output.
///
/// # Returns
///
/// Returns a [`MainEventLoopFuture`] that resolves to a [`CommonResult`] containing:
/// * `global_data` - The final [`GlobalData`] state after the event loop exits.
/// * `event_stream` - The [`InputDevice`] used for input events.
/// * `stdout` - The [`OutputDevice`] used for output.
///
/// # Errors
///
/// Returns [`miette::Error`] if there are errors during:
/// * Event loop initialization (setting up raw mode, app initialization).
/// * Event loop execution (input processing, rendering, signal handling).
/// * Terminal cleanup and restoration.
///
/// # Why return a boxed pinned future?
///
/// This function returns a [`Box::pin`]ned future (> 16KB clippy threshold) for safer
/// memory management and better performance characteristics.
///
/// ## Performance Benefits
///
/// * **Without [`Box::pin`]**: The entire > 16KB future gets copied every time it moves
///   between stack frames (function calls, async state transitions, select! operations).
/// * **With [`Box::pin`]**: Only an 8-byte pointer moves, while the actual future data
///   stays fixed on the heap, avoiding expensive > 16KB memory copies.
/// * Reduces stack pressure and improves CPU cache locality.
///
/// ## Safety Benefits
///
/// * This function may be called when the stack already has many frames from the main
///   application logic. Pinning this future to the heap avoids potential stack overflow
///   issues when the stack is deep.
/// * Provides defensive programming "better safe than sorry" approach for stack depth
///   management.
///
/// ## Usage Context
///
/// The returned boxed pinned future from this function is typically used in contexts
/// where:
/// - Single use: The future is created, awaited once, and then dropped - no loops or
///   repeated moves.
/// - Not stored in a struct: The future isn't being stored in a data structure that would
///   require [`std::pin::Pin`].
/// - Direct await: It's immediately awaited, not passed around or stored.
pub fn main_event_loop_impl<'a, S, AS>(
    app: BoxedSafeApp<S, AS>,
    exit_keys: &'a [InputEvent],
    state: S,
    initial_size: Size,
    input_device: InputDevice,
    output_device: OutputDevice,
) -> MainEventLoopFuture<'a, S, AS>
where
    S: Display + Debug + Default + Clone + Sync + Send + 'a,
    AS: Debug + Default + Clone + Sync + Send + 'static,
{
    Box::pin(async move {
        let mut app = app;
        let event_loop_state = EventLoopState::initialize(
            state,
            initial_size,
            output_device.clone(),
            &mut app,
        )?;

        run_main_event_loop(
            event_loop_state,
            app,
            exit_keys,
            input_device,
            output_device,
        )
        .await
    })
}

/// Holds all the state required for the main event loop.
#[allow(missing_debug_implementations)]
pub struct EventLoopState<S, AS>
where
    S: Display + Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send + 'static,
{
    global_data: GlobalData<S, AS>,
    main_thread_channel_receiver: mpsc::Receiver<TerminalWindowMainThreadSignal<AS>>,
    component_registry_map: ComponentRegistryMap<S, AS>,
    has_focus: HasFocus,
    telemetry: Telemetry<{ telemetry_default_constants::RING_BUFFER_SIZE }>,
}

impl<S, AS> EventLoopState<S, AS>
where
    S: Display + Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send + 'static,
{
    /// Initialize the event loop state with all required components.
    #[allow(clippy::needless_pass_by_value)]
    fn initialize(
        state: S,
        initial_size: Size,
        output_device: OutputDevice,
        app: &mut BoxedSafeApp<S, AS>,
    ) -> CommonResult<Self> {
        // Create communication channel.
        let (main_thread_channel_sender, main_thread_channel_receiver) =
            mpsc::channel::<TerminalWindowMainThreadSignal<AS>>(
                DefaultSize::MainThreadSignalChannelBufferSize.into(),
            );

        // Initialize global data.
        let global_data = GlobalData::try_to_create_instance(
            main_thread_channel_sender,
            state,
            initial_size,
            output_device.clone(),
            OffscreenBufferPool::new(initial_size),
        )?;

        // Initialize other components.
        let component_registry_map = ComponentRegistryMap::default();
        let has_focus = HasFocus::default();
        let telemetry = Telemetry::new((
            DefaultTiming::TelemetryRateLimitTimeThresholdMicros.into(),
            DefaultTiming::TelemetryFilterLowestResponseTimeMinMicros.into(),
        ));

        let mut event_loop_state = Self {
            global_data,
            main_thread_channel_receiver,
            component_registry_map,
            has_focus,
            telemetry,
        };

        // Initialize the app and perform first render.
        event_loop_state.initialize_app_and_render(app, &output_device)?;

        Ok(event_loop_state)
    }

    /// Initialize the app and perform the first render.
    fn initialize_app_and_render(
        &mut self,
        app: &mut BoxedSafeApp<S, AS>,
        output_device: &OutputDevice,
    ) -> CommonResult<()> {
        // Start raw mode
        RawMode::start(
            self.global_data.window_size,
            lock_output_device_as_mut!(output_device),
            output_device.is_mock,
        );

        // Initialize app and render.
        let telemetry = &mut self.telemetry;
        telemetry_record!(
            @telemetry: telemetry,
            @hint: TelemetryAtomHint::Render,
            @block: {
                app.app_init(&mut self.component_registry_map, &mut self.has_focus);
                AppManager::render_app(
                    app,
                    &mut self.global_data,
                    &mut self.component_registry_map,
                    &mut self.has_focus,
                    lock_output_device_as_mut!(output_device),
                    output_device.is_mock,
                )?;
            },
            @after_block: {
                self.global_data.set_hud_report(telemetry.report());
            }
        );

        self.log_startup_info();
        Ok(())
    }

    /// Log startup information if debugging is enabled.
    fn log_startup_info(&self) {
        (DISPLAY_LOG_TELEMETRY || DEBUG_TUI_MOD).then(|| {
            tracing::info!(
                message = %inline_string!(
                    "main_event_loop {sp} Startup {ch}",
                    sp = glyphs::RIGHT_ARROW_GLYPH,
                    ch = glyphs::CELEBRATE_GLYPH
                ),
                global_data_mut_ref = ?self.global_data,
            );
        });
    }

    /// Log shutdown information if debugging is enabled.
    fn log_shutdown_info(&self) {
        (DISPLAY_LOG_TELEMETRY || DEBUG_TUI_MOD).then(|| {
            tracing::info!(
                message = %inline_string!(
                    "main_event_loop {sp} Shutdown {ch}",
                    ch = glyphs::BYE_GLYPH,
                    sp = glyphs::RIGHT_ARROW_GLYPH,
                ),
                session_duration = %self.telemetry.session_duration()
            );
        });
    }

    /// Log telemetry information after each event loop iteration. This function must
    /// execute quickly, so it avoids deep traversal of the editor buffer and dialog
    /// buffers. This is called in a hot loop, on every render, so it must be quick!
    pub fn log_telemetry_info(&mut self) {
        (DISPLAY_LOG_TELEMETRY || DEBUG_TUI_MOD).then(|| {
            // % is Display, ? is Debug.
            tracing::info!(
                message = %inline_string!(
                    "AppManager::render_app() ok {ch}",
                    ch = glyphs::PAINT_GLYPH
                ),
                window_size = ?self.global_data.window_size,
                state = %self.global_data.state,
                report = %self.global_data.get_hud_report_no_spinner(),
            );

            if let Some(ref mut offscreen_buffer) = self.global_data.maybe_saved_ofs_buf {
                let mem_used = inline_string!(
                    "mem used: {size}",
                    size = offscreen_buffer.get_mem_size()
                );
                // % is Display, ? is Debug.
                tracing::info!(
                    message = %inline_string!(
                        "AppManager::render_app() offscreen_buffer {mem_used} {ch}",
                        mem_used = mem_used,
                        ch = glyphs::SCREEN_BUFFER_GLYPH
                    ),
                );
            }
        });
    }
}

/// Run the main event loop with proper separation of concerns.
async fn run_main_event_loop<S, AS>(
    mut event_loop_state: EventLoopState<S, AS>,
    mut app: BoxedSafeApp<S, AS>,
    exit_keys: &[InputEvent],
    mut input_device: InputDevice,
    output_device: OutputDevice,
) -> CommonResult<(GlobalData<S, AS>, InputDevice, OutputDevice)>
where
    S: Display + Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send + 'static,
{
    // Main event loop
    loop {
        tokio::select! {
            // Handle signals from the app.
            maybe_signal = event_loop_state.main_thread_channel_receiver.recv() => {
                if let Some(signal) = maybe_signal
                    && handle_main_thread_signal(
                        signal,
                        &mut event_loop_state,
                        &mut app,
                        exit_keys,
                        &output_device,
                    )? {
                        break; // Exit requested
                    }
            }

            // Handle input events.
            maybe_input_event = input_device.next_input_event() => {
                if let Some(input_event) = maybe_input_event {
                    handle_input_event(
                        input_event,
                        &mut event_loop_state,
                        &mut app,
                        exit_keys,
                        &output_device,
                    );
                } else {
                    // No more events, exit loop.
                    break;
                }
            }
        }

        event_loop_state.log_telemetry_info();
    }

    // Cleanup and return results.
    event_loop_state.log_shutdown_info();
    Ok((event_loop_state.global_data, input_device, output_device))
}

/// Handle signals received from the main thread channel.
/// Returns true if exit was requested.
fn handle_main_thread_signal<S, AS>(
    signal: TerminalWindowMainThreadSignal<AS>,
    event_loop_state: &mut EventLoopState<S, AS>,
    app: &mut BoxedSafeApp<S, AS>,
    exit_keys: &[InputEvent],
    output_device: &OutputDevice,
) -> CommonResult<bool>
where
    S: Display + Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send + 'static,
{
    match signal {
        TerminalWindowMainThreadSignal::Exit => {
            RawMode::end(
                event_loop_state.global_data.window_size,
                lock_output_device_as_mut!(output_device),
                output_device.is_mock,
            );
            Ok(true) // Request exit
        }
        TerminalWindowMainThreadSignal::Render(_) => {
            handle_render_signal(event_loop_state, app, output_device)?;
            Ok(false)
        }
        TerminalWindowMainThreadSignal::ApplyAppSignal(action) => {
            handle_app_signal(&action, event_loop_state, app, exit_keys, output_device);
            Ok(false)
        }
    }
}

/// Handle render signal from the main thread.
fn handle_render_signal<S, AS>(
    event_loop_state: &mut EventLoopState<S, AS>,
    app: &mut BoxedSafeApp<S, AS>,
    output_device: &OutputDevice,
) -> CommonResult<()>
where
    S: Display + Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send + 'static,
{
    let telemetry = &mut event_loop_state.telemetry;
    telemetry_record!(
        @telemetry: telemetry,
        @hint: TelemetryAtomHint::Render,
        @block: {
            AppManager::render_app(
                app,
                &mut event_loop_state.global_data,
                &mut event_loop_state.component_registry_map,
                &mut event_loop_state.has_focus,
                lock_output_device_as_mut!(output_device),
                output_device.is_mock,
            )?;
        },
        @after_block: {
            event_loop_state.global_data.set_hud_report(telemetry.report());
        }
    );
    Ok(())
}

/// Handle app signal from the main thread.
fn handle_app_signal<S, AS>(
    action: &AS,
    event_loop_state: &mut EventLoopState<S, AS>,
    app: &mut BoxedSafeApp<S, AS>,
    exit_keys: &[InputEvent],
    output_device: &OutputDevice,
) where
    S: Display + Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send + 'static,
{
    let telemetry = &mut event_loop_state.telemetry;
    telemetry_record!(
        @telemetry: telemetry,
        @hint: TelemetryAtomHint::Signal,
        @block: {
            let result = app.app_handle_signal(
                action,
                &mut event_loop_state.global_data,
                &mut event_loop_state.component_registry_map,
                &mut event_loop_state.has_focus,
            );
            handle_result_generated_by_app_after_handling_action_or_input_event(
                result,
                None,
                exit_keys,
                app,
                &mut event_loop_state.global_data,
                &mut event_loop_state.component_registry_map,
                &mut event_loop_state.has_focus,
                lock_output_device_as_mut!(output_device),
                output_device.is_mock,
            );
        },
        @after_block: {
            event_loop_state.global_data.set_hud_report(telemetry.report());
        }
    );
}

/// Handle input events from the terminal.
fn handle_input_event<S, AS>(
    input_event: InputEvent,
    event_loop_state: &mut EventLoopState<S, AS>,
    app: &mut BoxedSafeApp<S, AS>,
    exit_keys: &[InputEvent],
    output_device: &OutputDevice,
) where
    S: Display + Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send + 'static,
{
    log_input_event_if_enabled(&input_event);

    // Handle resize events specially.
    if let InputEvent::Resize(new_size) = input_event {
        handle_resize_event(new_size, event_loop_state, app, output_device);
    }

    // Process all input events (including resize)
    process_input_event(input_event, event_loop_state, app, exit_keys, output_device);
}

/// Log input event if debugging is enabled.
fn log_input_event_if_enabled(input_event: &InputEvent) {
    (DISPLAY_LOG_TELEMETRY || DEBUG_TUI_MOD).then(|| {
        if let InputEvent::Keyboard(_) = input_event {
            tracing::info!(
                message = %inline_string!(
                    "main_event_loop {sp} Tick {ch}",
                    sp = glyphs::RIGHT_ARROW_GLYPH,
                    ch = glyphs::CLOCK_TICK_GLYPH
                ),
                input_event = ?input_event
            );
        }
    });
}

/// Handle terminal resize events.
fn handle_resize_event<S, AS>(
    new_size: Size,
    event_loop_state: &mut EventLoopState<S, AS>,
    app: &mut BoxedSafeApp<S, AS>,
    output_device: &OutputDevice,
) where
    S: Display + Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send + 'static,
{
    let telemetry = &mut event_loop_state.telemetry;
    telemetry_record!(
        @telemetry: telemetry,
        @hint: TelemetryAtomHint::Resize,
        @block: {
            handle_resize(
                new_size,
                &mut event_loop_state.global_data,
                app,
                &mut event_loop_state.component_registry_map,
                &mut event_loop_state.has_focus,
                lock_output_device_as_mut!(output_device),
                output_device.is_mock,
            );
        },
        @after_block: {
            event_loop_state.global_data.set_hud_report(telemetry.report());
        }
    );
}

/// Process input events and delegate to the app.
fn process_input_event<S, AS>(
    input_event: InputEvent,
    event_loop_state: &mut EventLoopState<S, AS>,
    app: &mut BoxedSafeApp<S, AS>,
    exit_keys: &[InputEvent],
    output_device: &OutputDevice,
) where
    S: Display + Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send + 'static,
{
    let telemetry = &mut event_loop_state.telemetry;
    telemetry_record!(
        @telemetry: telemetry,
        @hint: TelemetryAtomHint::Input,
        @block: {
            actually_process_input_event(
                &mut event_loop_state.global_data,
                app,
                input_event,
                exit_keys,
                &mut event_loop_state.component_registry_map,
                &mut event_loop_state.has_focus,
                lock_output_device_as_mut!(output_device),
                output_device.is_mock,
            );
        },
        @after_block: {
            event_loop_state.global_data.set_hud_report(telemetry.report());
        }
    );
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
    S: Display + Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send + 'static,
{
    let result = app.app_handle_input_event(
        input_event.clone(),
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
#[allow(clippy::implicit_hasher)]
pub fn handle_resize<S, AS>(
    new_size: Size,
    global_data_mut_ref: &mut GlobalData<S, AS>,
    app: &mut BoxedSafeApp<S, AS>,
    component_registry_map: &mut ComponentRegistryMap<S, AS>,
    has_focus: &mut HasFocus,
    locked_output_device: LockedOutputDevice<'_>,
    is_mock: bool,
) where
    S: Display + Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
{
    global_data_mut_ref.set_size(new_size);
    global_data_mut_ref.maybe_saved_ofs_buf = None;
    global_data_mut_ref.offscreen_buffer_pool.resize(new_size);

    // We don't care about the result of this operation.
    AppManager::render_app(
        app,
        global_data_mut_ref,
        component_registry_map,
        has_focus,
        locked_output_device,
        is_mock,
    )
    .ok();
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
    S: Display + Debug + Default + Clone + Sync + Send,
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
                    }
                }
            }

            EventPropagation::ConsumedRender => {
                AppManager::render_app(
                    app,
                    global_data_mut_ref,
                    component_registry_map,
                    has_focus,
                    locked_output_device,
                    is_mock,
                )
                .ok();
            }

            EventPropagation::Consumed => {}

            EventPropagation::ExitMainEventLoop => {
                request_exit_by_sending_signal(main_thread_channel_sender);
            }
        },
        Err(error) => {
            tracing::error!(
                message = %inline_string!(
                    "main_event_loop {sp} handle_result_generated_by_app_after_handling_action_or_input_event {ch}",
                    ch = glyphs::SUSPICIOUS_GLYPH,
                    sp = glyphs::RIGHT_ARROW_GLYPH,
                ),
                error = ?error
            );
        }
    }
}

/// Request exit from the main event loop, as exit keys were pressed.
/// Note: make sure to wrap the call to `send()` in a [`tokio::spawn()`] so that it
/// doesn't block the calling thread.
///
/// More info: <https://tokio.rs/tokio/tutorial/channels>.
fn request_exit_by_sending_signal<AS>(
    channel_sender: mpsc::Sender<TerminalWindowMainThreadSignal<AS>>,
) where
    AS: Debug + Default + Clone + Sync + Send + 'static,
{
    tokio::spawn(async move {
        // We don't care about the result of this operation.
        channel_sender
            .send(TerminalWindowMainThreadSignal::Exit)
            .await
            .ok();
    });
}

struct AppManager<S, AS>
where
    S: Display + Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
{
    _phantom: PhantomData<(S, AS)>,
}

impl<S, AS> AppManager<S, AS>
where
    S: Display + Debug + Default + Clone + Sync + Send,
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
                global_data_mut_ref.maybe_saved_ofs_buf = None;
                Ok(render_window_too_small_error(window_size))
            }
        };

        match render_result {
            Err(error) => {
                let mut painter = crate::PaintRenderOpImplCrossterm {};
                painter.flush(locked_output_device);

                // Print debug message w/ error.
                DEBUG_TUI_MOD.then(|| {
                    tracing::error!(
                        message = %inline_string!(
                            "AppManager::render_app() error {ch}",
                            ch = glyphs::SUSPICIOUS_GLYPH
                        ),
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

fn render_window_too_small_error(window_size: Size) -> RenderPipeline {
    // Show warning message that window_size is too small.
    let msg = inline_string!(
        "Window size is too small. Minimum size is {} cols x {} rows",
        MinSize::Col as u8,
        MinSize::Row as u8
    );
    let msg_gcs = GCStringOwned::from(msg);
    let trunc_msg = msg_gcs.trunc_end_to_fit(window_size);

    let trunc_msg_gcs = GCStringOwned::from(trunc_msg);
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

    let style_bold = new_style!(bold);

    render_pipeline! {
        @push_into pipeline
        at ZOrder::Normal
        =>
            RenderOpIR::Common(RenderOpCommon::ResetColor),
            RenderOpIR::Common(RenderOpCommon::MoveCursorPositionAbs(col_pos + row_pos))
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
    use crate::{assert_eq2, ch, col, defaults::get_default_gradient_stops, height, inline_string, is_fully_uninteractive_terminal, is_partially_uninteractive_terminal, key_press, main_event_loop_impl, new_style, ok, render_pipeline, render_tui_styled_texts_into, send_signal, tui_color, tui_style_attrib, tui_styled_text, width, App, ColorWheel, ColorWheelConfig, ColorWheelSpeed, CommonResult, ComponentRegistryMap, CrosstermEventResult, EventPropagation, GlobalData, GradientGenerationPolicy, GradientLengthKind, HasFocus, InlineVec, InputDevice, InputDeviceExtMock, InputEvent, Key, KeyPress, OutputDevice, OutputDeviceExt, PixelChar, RenderOpCommon, RenderOpIRVec, RenderPipeline, SpecialKey, TTYResult, TerminalWindowMainThreadSignal, TextColorizationPolicy, TuiStyle, TuiStyleAttribs, ZOrder};
    use smallvec::smallvec;
    use std::{fmt::{Debug, Display, Formatter},
              time::Duration};
    use test_fixture_app::AppMainTest;
    use test_fixture_state::{AppSignal, State};

    #[tokio::test]
    #[allow(clippy::needless_return)]
    #[allow(clippy::too_many_lines)]
    async fn test_main_event_loop_impl() -> CommonResult<()> {
        // Skip this test if not running in an interactive terminal (e.g., when output is redirected).
        if let TTYResult::IsNotInteractive = is_partially_uninteractive_terminal() {
            return ok!();
        }

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
        let exit_keys: InlineVec<InputEvent> =
            smallvec![InputEvent::Keyboard(key_press! { @char 'x' })];

        // Simulated key inputs.
        let generator_vec: InlineVec<CrosstermEventResult> = smallvec![
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
        assert!(
            stdout_mock
                .get_copy_of_buffer_as_string_strip_ansi()
                .contains("State{counter:2}")
        );

        // println!(
        //     "global_data.offscreen_buffer: {:?}",
        //     global_data.maybe_saved_ofs_buf.
        // );

        let ofs_buf = global_data.maybe_saved_ofs_buf.unwrap();

        // This is for CI/CD environment. It does not support truecolor, and degrades to
        // ANSI 256 colors
        if let TTYResult::IsNotInteractive = is_fully_uninteractive_terminal() {
            // Check pixel char at 4 x 7.
            {
                let PixelChar::PlainText {
                    display_char,
                    style: _,
                } = ofs_buf.buffer[4][7]
                else {
                    panic!(
                        "Expected PixelChar::PlainText, got: {:?}",
                        ofs_buf.buffer[4][7]
                    );
                };
                assert_eq2!(display_char.to_string(), "S");
            }

            // Check pixel char at 10 x 7.
            {
                let PixelChar::PlainText {
                    display_char,
                    style: _,
                } = ofs_buf.buffer[10][7]
                else {
                    panic!(
                        "Expected PixelChar::PlainText, got: {:?}",
                        ofs_buf.buffer[10][7]
                    );
                };
                assert_eq2!(display_char.to_string(), "H");
            }
        }
        // This is for local development environment. It supports truecolor.
        else {
            // Check pixel char at 4 x 7.
            {
                assert_eq2!(
                    PixelChar::PlainText {
                        display_char: 'S',
                        style: TuiStyle {
                            color_fg: Some(tui_color!(102, 0, 255)),
                            ..Default::default()
                        },
                    },
                    ofs_buf.buffer[4][7].clone()
                );
            }

            // Check pixel char at 10 x 7.
            {
                assert_eq2!(
                    PixelChar::PlainText {
                        display_char: 'H',
                        style: TuiStyle {
                            id: None,
                            attribs: TuiStyleAttribs {
                                dim: Some(tui_style_attrib::Dim),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                    },
                    ofs_buf.buffer[10][7].clone()
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

        impl Display for State {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "State{{counter:{}}}", self.counter)
            }
        }

        impl Debug for State {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "State {{ counter: {:?} }}", self.counter)
            }
        }
    }

    mod test_fixture_app {
        use crate::ColorWheel;

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
        use super::*;
        use crate::{Pos, row, throws_with_return};

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
                        inline_string!("{a:?}", a = global_data_mut_ref.state);
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
                        let mut acc_render_op = RenderOpIRVec::new();
                        acc_render_op.push(RenderOpCommon::ResetColor);

                        // Render using color_wheel_rgb.
                        acc_render_op.push(RenderOpCommon::MoveCursorPositionAbs(Pos {
                            col_index: col_idx,
                            row_index: row_idx,
                        }));

                        let index = data.color_wheel_rgb.get_index();
                        let len = match data.color_wheel_rgb.get_gradient_len() {
                            GradientLengthKind::ColorWheel(len) => len,
                            _ => 0,
                        };

                        let string = inline_string!(
                            "{state_string}, gradient: [index: {a:?}, len: {b}]",
                            a = index,
                            b = len
                        );

                        let string_gcs = string.into();

                        render_tui_styled_texts_into(
                            &data.color_wheel_rgb.colorize_into_styled_texts(
                                &string_gcs,
                                GradientGenerationPolicy::ReuseExistingGradientAndIndex,
                                TextColorizationPolicy::ColorEachWord(None),
                            ),
                            &mut acc_render_op,
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
        use crate::{LengthOps, Size, tui_styled_texts};

        /// Shows helpful messages at the bottom row of the screen.
        pub fn create_status_bar_message(pipeline: &mut RenderPipeline, size: Size) {
            let styled_texts = tui_styled_texts! {
                tui_styled_text!{ @style: new_style!(dim)       , @text: "Hints:"},
                tui_styled_text!{ @style: new_style!(bold)      , @text: " x : Exit 🖖 "},
                tui_styled_text!{ @style: new_style!(dim)       , @text: " … "},
                tui_styled_text!{ @style: new_style!(underline) , @text: " ↑ / + : inc "},
                tui_styled_text!{ @style: new_style!(dim)       , @text: " … "},
                tui_styled_text!{ @style: new_style!(underline) , @text: " ↓ / - : dec "},
            };

            let display_width = styled_texts.display_width();
            let col_center = *(size.col_width - display_width) / ch(2);
            let row_bottom = size.row_height.convert_to_index();
            let center = col(col_center) + row_bottom;

            let mut render_ops = RenderOpIRVec::new();
            render_ops += RenderOpCommon::MoveCursorPositionAbs(center);
            render_tui_styled_texts_into(&styled_texts, &mut render_ops);
            pipeline.push(ZOrder::Normal, render_ops);
        }
    }
}
