// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{Ansi256GradientIndex, BoxedSafeApp, ColorWheel, ColorWheelConfig,
            ColorWheelSpeed, CommonResult, ComponentRegistryMap, Continuation,
            DEBUG_TUI_MOD, DISPLAY_LOG_TELEMETRY, DefaultInputEventHandler, DefaultSize,
            DefaultTiming, EventPropagation, FlushKind, GCStringOwned, GetMemSize,
            GlobalData, GradientGenerationPolicy, HasFocus, InputDevice, InputEvent,
            LockedOutputDevice, MainEventLoopFuture, MinSize, OffscreenBufferPool,
            OutputDevice, RawMode, RenderOpCommon, RenderOpFlush, RenderOpIR,
            RenderPipeline, Size, SufficientSize, TelemetryAtomHint,
            TerminalWindowMainThreadSignal, TextColorizationPolicy, ZOrder, ch, col,
            emit_stderr_redirection_disclaimer, glyphs, height, inline_string,
            lock_output_device_as_mut, new_style, ok, render_pipeline, row,
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
/// This is the internal API, and not the public API [`main_event_loop()`]. This
/// separation exists to allow for testing using dependency injection.
///
/// This function takes pre-initialized components (terminal size, input/output devices)
/// and runs the actual async event loop. It handles all input events, dispatches them to
/// the [`crate::App`] for processing, renders the app after each event, and manages all
/// signals sent from the app to the main event loop.
///
/// # Arguments
///
/// * `app` - The [`BoxedSafeApp`] instance that will handle input events and signals.
/// * `exit_keys` - An owned [`Vec`] of [`InputEvent`]s that will trigger application
///   exit.
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
/// # Why return a pinned boxed future?
///
/// This function returns a pinned boxed future ([`Pin<Box<T>>`]; > 16KB clippy threshold)
/// for safer memory management and better performance characteristics.
///
/// ## Performance Benefits
///
/// * **Without [`Box::pin`]**: The entire > 16KB future gets copied every time it moves
///   between stack frames (function calls, async state transitions, [`select!`]
///   operations).
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
/// # Core Async Concepts: [`Pin`] and [`Unpin`]
///
/// Understanding the relationship between the [`Pin`] struct and the [`Unpin`] trait is
/// critical for building efficient async systems.
///
/// ## The Household Objects Metaphor
///
/// Think of items in a house.
///
/// 1. **Portable furniture** (the [`Unpin`] auto-trait): Most types (like a `struct` or
///    `enum`) that don't care where they are placed implement this trait bound
///    automatically. You can move them to a new room (memory address) and they function
///    exactly the same.
///
/// 2. **Fixed fixtures** (the absence of `!Unpin` in the trait bound): Some types (like
///    `async` blocks) do not implement the [`Unpin`] trait bound because they are like
///    **built-in sinks**. They have **internal plumbing** (self-references) that is
///    calibrated to their exact position in the room. If you move a fixed fixture to a
///    new room, the internal plumbing still "remembers" the coordinates of the old room.
///    The connections break because the pipes are now pointing at empty space where the
///    fixture *used* to be.
///
/// The [`Pin`] **struct wrapper** is the **bolt** that fixes the fixture to the floor. It
/// ensures that once those internal connections are established, the fixture stays put so
/// the plumbing always points to the right place.
///
/// ## When is Pinning Necessary?
///
/// 1. **Self-referential types**: Futures created by `async` blocks often contain
///    pointers to their own internal state. These must be pinned before they are polled.
/// 2. **Trait Objects**: When using `Box<dyn Future>` or `Box<dyn Stream>`, the compiler
///    cannot prove if the underlying concrete type is [`Unpin`], so you must wrap it in a
///    [`Pin`] struct.
///
/// ## When is Pinning a "No-op"?
///
/// If a type already implements the [`Unpin`] trait (like [`i32`], [`String`], or
/// [`tokio::task::JoinHandle`]), wrapping it in a [`Pin`] struct adds a heap allocation
/// and indirection without any safety benefit. These types are **portable furniture** —
/// they never store their own coordinates, so they can always be safely moved without
/// breaking anything, making the "bolt" redundant.
///
/// ## How it's Enforced (The "Magic")
///
/// The Rust compiler and standard library work together to enforce pinning:
///
/// 1. **Auto-Traits**: [`Unpin`] is an **auto-trait** (like [`Send`] or [`Sync`]). The
///    compiler automatically implements it for almost every type you write. Only
///    compiler-generated futures (from `async` blocks) are automatically `!Unpin`.
/// 2. **API Restriction**: The [`Pin`] struct does not use compiler magic to "watch"
///    memory. Instead, it **hides** the `&mut T` for `!Unpin` types. Since you cannot get
///    a mutable reference, you cannot call functions like [`std::mem::swap`] or
///    [`std::mem::replace`], making it impossible to move the data safely.
///
/// ## Why `Pin<Box<T>>`?
///
/// You will often see the combination [`Pin`]`<`[`Box`]`<T>>`. This serves two distinct
/// purposes:
///
/// - **[`Box`]** is used because the size of `T` (often an anonymous future or trait
///   object) is not known at compile time. It puts the furniture in a "shipping crate" on
///   the heap.
/// - **[`Pin`]** is used because `T` is `!Unpin`. It bolts that crate to the floor of the
///   heap so it can never be moved to a different heap address.
///
/// [`main_event_loop()`]: crate::TerminalWindow::main_event_loop()
/// [`Pin<Box<T>>`]: std::boxed::Box::pin
/// [`Pin`]: std::pin::Pin
/// [`select!`]: tokio::select
/// [`Unpin`]: std::marker::Unpin
pub fn main_event_loop_impl<S, AS>(
    app: BoxedSafeApp<S, AS>,
    exit_keys: Vec<InputEvent>,
    state: S,
    initial_size: Size,
    input_device: InputDevice,
    output_device: OutputDevice,
) -> MainEventLoopFuture<S, AS>
where
    S: Display + Debug + Default + Clone + Sync + Send + 'static,
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
            &exit_keys,
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
    /// Initializes the event loop state with all required components.
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

        emit_stderr_redirection_disclaimer();

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

    /// Initializes the app and performs the first render.
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
        ok!()
    }

    /// Logs startup information if debugging is enabled.
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

    /// Logs shutdown information if debugging is enabled.
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

    /// Logs telemetry information after each event loop iteration. This function must
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

/// Runs the main event loop with proper separation of concerns.
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
            maybe_input_event = input_device.next() => {
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

/// Handles signals received from the main thread channel.
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

/// Handles render signal from the main thread.
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
    ok!()
}

/// Handles app signal from the main thread.
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

/// Handles input events from the terminal.
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

/// Logs input event if debugging is enabled.
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

/// Handles terminal resize events.
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

/// Processes input events and delegates to the app.
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
                    if let Continuation::Stop = check_if_exit_keys_pressed {
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

/// Requests exit from the main event loop, as exit keys were pressed. Note: make sure to
/// wrap the call to [`send()`] in a [`tokio::spawn()`] so that it doesn't block the
/// calling thread. See [channels] for more details.
///
/// [`send()`]: mpsc::Sender::send
/// [channels]: https://tokio.rs/tokio/tutorial/channels
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
