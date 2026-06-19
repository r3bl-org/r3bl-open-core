// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words cooldown

//! Main [`PTY`] multiplexer orchestrator.
//!
//! This module provides the main [`PTYMux`] struct that coordinates all components and
//! manages the event loop for the terminal multiplexer.
//!
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal

use super::{InputRouter, OutputRenderer, Process, ProcessManager, output_renderer,
            show_notification_non_blocking};
use crate::{AnsiSequenceGenerator, Continuation, DEBUG_TUI_PTY_MUX, InputEvent,
            Size, TerminalInteractiveStatus, TuiAvailability, col,
            core::{check_is_terminal_interactive, emit_stderr_redirection_disclaimer,
                   get_size,
                   osc::OscController,
                   terminal_io::{InputDevice, OutputDevice, TerminalModeController}},
            ok, row};
use std::{fmt::Debug,
          time::{Duration, Instant}};
use tokio::time::interval;

/// Builder for configuring and creating a [`PTYMux`] instance.
#[derive(Default, Debug)]
pub struct PTYMuxBuilder {
    process_configs: Vec<(String, String, Vec<String>)>,
    terminal_size: Option<Size>,
}

impl PTYMuxBuilder {
    /// Sets the processes to be managed by the multiplexer.
    #[must_use]
    pub fn processes(
        mut self,
        processes: Vec<(impl Into<String>, impl Into<String>, Vec<String>)>,
    ) -> Self {
        self.process_configs = processes
            .into_iter()
            .map(|(n, c, a)| (n.into(), c.into(), a))
            .collect();
        self
    }

    /// Adds a single process to the multiplexer.
    #[must_use]
    pub fn add_process(
        mut self,
        name: impl Into<String>,
        command: impl Into<String>,
        args: Vec<String>,
    ) -> Self {
        self.process_configs
            .push((name.into(), command.into(), args));
        self
    }

    /// Sets the terminal size for the multiplexer.
    #[must_use]
    pub fn terminal_size(mut self, size: Size) -> Self {
        self.terminal_size = Some(size);
        self
    }

    /// Builds the [`PTYMux`] instance.
    ///
    /// # Returns
    ///
    /// Returns a [`TuiAvailability`] containing the [`PTYMux`] instance if the
    /// terminal is interactive.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No processes are configured
    /// - More than [`MAX_PROCESSES`] processes are configured
    ///
    /// [`check_is_terminal_interactive()`]: crate::check_is_terminal_interactive
    /// [`MAX_PROCESSES`]: output_renderer::MAX_PROCESSES
    #[must_use]
    pub fn build(self) -> TuiAvailability<PTYMux> {
        if self.process_configs.is_empty() {
            return TuiAvailability::Broken(miette::miette!(
                "At least one process must be configured"
            ));
        }

        if self.process_configs.len() > output_renderer::MAX_PROCESSES {
            return TuiAvailability::Broken(miette::miette!(
                "Maximum of {} processes allowed",
                output_renderer::MAX_PROCESSES
            ));
        }

        match check_is_terminal_interactive() {
            TerminalInteractiveStatus::NotAvailable(reason) => {
                TuiAvailability::NotAvailable(reason)
            }

            TerminalInteractiveStatus::Available => {
                let init = || -> miette::Result<PTYMux> {
                    let terminal_size = match self.terminal_size {
                        Some(size) => size,
                        None => get_size()?,
                    };

                    let processes = self
                        .process_configs
                        .into_iter()
                        .map(|(name, command, args)| {
                            Process::new(name, command, args, terminal_size)
                        })
                        .collect();

                    emit_stderr_redirection_disclaimer();

                    let output_device = OutputDevice::new_stdout();
                    let input_device = InputDevice::default();

                    Ok(PTYMux {
                        process_manager: ProcessManager::new(processes, terminal_size),
                        input_router: InputRouter::new(),
                        output_renderer: OutputRenderer::new(terminal_size),
                        terminal_size,
                        output_device,
                        input_device,
                    })
                };
                match init() {
                    Ok(mux) => TuiAvailability::Available(mux),
                    Err(e) => TuiAvailability::Broken(e),
                }
            }
        }
    }
}

/// Main [`PTY`] multiplexer that orchestrates all components.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub struct PTYMux {
    process_manager: ProcessManager,
    input_router: InputRouter,
    output_renderer: OutputRenderer,
    terminal_size: Size,
    output_device: OutputDevice,
    input_device: InputDevice,
}

impl Debug for PTYMux {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PTYMux")
            .field("process_manager", &self.process_manager)
            .field("input_router", &self.input_router)
            .field("output_renderer", &self.output_renderer)
            .field("terminal_size", &self.terminal_size)
            .field("output_device", &"<OutputDevice>")
            .field("input_device", &"<InputDevice>")
            .finish()
    }
}

impl PTYMux {
    /// Creates a new builder for configuring a [`PTYMux`] instance.
    #[must_use]
    pub fn builder() -> PTYMuxBuilder { PTYMuxBuilder::default() }

    /// Runs the multiplexer event loop.
    ///
    /// This is the main entry point that:
    /// 1. Starts raw mode
    /// 2. Sets initial terminal title
    /// 3. Runs the main event loop
    /// 4. Cleans up on exit
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Terminal title ([`OSC`]) setup fails
    /// - Process spawning fails
    /// - Status bar rendering fails
    /// - Input or output event handling fails during the main loop
    ///
    /// # Note on [`stderr`] redirection
    ///
    /// This function calls [`emit_stderr_redirection_disclaimer()`] to ensure that if
    /// [`stderr`] is redirected, the user is notified that application logs are handled
    /// internally.
    ///
    /// # Other entry points for interactive terminal apps
    ///
    /// See [interactive terminal application entry points].
    ///
    /// [`emit_stderr_redirection_disclaimer()`]: crate::emit_stderr_redirection_disclaimer
    /// [`OSC`]: crate::OscEvent
    /// [`stderr`]: std::io::stderr
    /// [interactive terminal application entry points]: crate#interactive-terminal-application-entry-points
    pub async fn run(mut self) -> miette::Result<()> {
        // Start raw mode.
        let _raw_mode_guard = self.output_device.enter_raw_mode()?;
        let _fullscreen_tui_mode_guard = self.output_device.setup_full_screen_tui()?;

        DEBUG_TUI_PTY_MUX.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "PTYMux::run",
                info = %crate::inline_string!("Raw mode started")
            };
        });

        // Set initial terminal title using OSC controller.
        {
            let mut osc = OscController::new(&self.output_device);
            osc.set_title_and_tab("PTYMux Example - Starting")?;
        }

        // Start all processes at startup.
        DEBUG_TUI_PTY_MUX.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "PTYMux::run",
                info = %crate::inline_string!("Starting all processes")
            };
        });
        self.process_manager.start_all_processes()?;

        // Clear screen before showing first process.
        self.output_device.write(|out| {
            let _unused = out.write_all(AnsiSequenceGenerator::clear_screen().as_bytes());
            let _unused = out.write_all(
                AnsiSequenceGenerator::cursor_position(row(0), col(0)).as_bytes(),
            );
            let _unused = out.flush();
        });

        // Trigger initial process switch to show first process.
        self.process_manager.switch_to(0);

        // Render initial status bar.
        self.output_renderer
            .render_initial_status_bar(&self.output_device, &self.process_manager)?;

        // Main event loop
        DEBUG_TUI_PTY_MUX.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "PTYMux::run",
                info = %crate::inline_string!("Starting main event loop")
            };
        });
        let result = self.run_event_loop().await;
        DEBUG_TUI_PTY_MUX.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "PTYMux::run",
                info = %crate::inline_string!("Main event loop exited with result: {:?}", result)
            };
        });

        // Always cleanup regardless of error.
        self.cleanup_terminal();

        // `_fullscreen_tui_mode_guard` and `_raw_mode_guard` are dropped here,
        // which securely restores the terminal to its original state.
        result
    }

    /// Main event loop that handles input and output events.
    async fn run_event_loop(&mut self) -> miette::Result<()> {
        use adaptive_render_budget::{AdaptiveRenderResult::{Render, Skip},
                                     Budget};

        // Create a periodic timer for status bar updates.
        let mut status_bar_interval = interval(Duration::from_millis(500));

        // Create a fast timer for polling PTY output.
        let mut output_poll_interval = interval(Duration::from_millis(10));

        let mut render_budget = Budget::default();

        loop {
            tokio::select! {
                // Poll ALL processes and update their virtual terminal buffers.
                // https://developerlife.com/2024/07/10/rust-async-cancellation-safety-tokio/#example-1-right-and-wrong-way-to-sleep-and-interval
                _ = output_poll_interval.tick() => {
                    match render_budget.should_render(&mut self.process_manager) {
                        Render => {
                            render_budget.mark_start();
                            // Get the active process's virtual terminal and render it.
                            self.output_renderer.render_from_active_buffer(
                                &self.output_device,
                                &self.process_manager
                            )?;
                            // Clear the "needs rendering" flag for the active process.
                            self.process_manager.mark_active_as_rendered();
                            render_budget.mark_end();
                        },
                        Skip => {
                            DEBUG_TUI_PTY_MUX.then(|| {
                                // % is Display, ? is Debug.
                                tracing::info! {
                                    message = "Skipping render, backpressure detected in stdout",
                                    info = %crate::inline_string!("Current frame delay: {:?}", render_budget.render_cooldown_delay)
                                };
                            });
                        },

                    }
                }

                // Handle user input using existing InputDevice.
                Some(input_event) = self.input_device.next() => {
                    crate::DEBUG_TUI_PTY_MUX.then(|| {
                        // % is Display, ? is Debug.
                        tracing::debug! {
                            message = "PTYMux::run_event_loop",
                            info = %crate::inline_string!("Received input event: {:?}", input_event)
                        };
                    });

                    // Show desktop notification for input event (filter out mouse events)
                    if !matches!(input_event, InputEvent::Mouse(_)) {
                        show_notification_non_blocking("PTY Mux - Input Event", &format!("Input event received: {input_event:?}"));
                    }

                    // Create OSC controller for this input handling.
                    let mut osc = OscController::new(&self.output_device);

                    // Handle input events using the input router.
                    crate::DEBUG_TUI_PTY_MUX.then(|| {
                        // % is Display, ? is Debug.
                        tracing::debug! {
                            message = "PTYMux::run_event_loop",
                            info = %crate::inline_string!("Handling input event: {:?}", input_event)
                        };
                    });
                    let continuation = self.input_router.handle_input(
                        input_event,
                        &mut self.process_manager,
                        &mut osc,
                        &self.output_device
                    )?;

                    if continuation == Continuation::Stop {
                        crate::DEBUG_TUI_PTY_MUX.then(|| {
                            // % is Display, ? is Debug.
                            tracing::debug! {
                                message = "PTYMux::run_event_loop",
                                info = %crate::inline_string!("Exit requested by input router - breaking main event loop")
                            };
                        });
                        break;
                    }
                }

                // Periodic status bar updates - ensures status bar is visible even when idle.
                _ = status_bar_interval.tick() => {
                    self.output_renderer.render_initial_status_bar(&self.output_device, &self.process_manager)?;
                }
            }
        }

        DEBUG_TUI_PTY_MUX.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "PTYMux::run_event_loop",
                info = %crate::inline_string!("Event loop completed - returning Ok(())")
            };
        });

        ok!()
    }

    /// Updates terminal size for all components.
    ///
    /// This method is called when the terminal is resized and ensures all components are
    /// aware of the new size.
    pub fn update_terminal_size(&mut self, new_size: Size) {
        self.terminal_size = new_size;
        self.process_manager.handle_terminal_resize(new_size);
        self.output_renderer.update_terminal_size(new_size);
    }

    /// Gets the current terminal size.
    #[must_use]
    pub fn terminal_size(&self) -> Size { self.terminal_size }

    /// Gets a reference to the process manager.
    #[must_use]
    pub fn process_manager(&self) -> &ProcessManager { &self.process_manager }

    /// Gets a mutable reference to the process manager.
    pub fn process_manager_mut(&mut self) -> &mut ProcessManager {
        &mut self.process_manager
    }

    /// Cleanup terminal state - always called on exit.
    #[allow(clippy::too_many_lines)]
    fn cleanup_terminal(&mut self) {
        let start_time = std::time::Instant::now();
        DEBUG_TUI_PTY_MUX.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "PTYMux::cleanup_terminal",
                info = %crate::inline_string!("Starting cleanup - terminal size: {:?}", self.terminal_size)
            };
        });

        // First, kill all running processes.
        DEBUG_TUI_PTY_MUX.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "PTYMux::cleanup_terminal",
                info = %crate::inline_string!("Step 1: Shutting down process manager")
            };
        });
        self.process_manager.shutdown_all_processes();
        DEBUG_TUI_PTY_MUX.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "PTYMux::cleanup_terminal",
                info = %crate::inline_string!("Step 1 completed in {:?}", start_time.elapsed())
            };
        });

        // Give processes a short time to terminate gracefully, then force exit.
        DEBUG_TUI_PTY_MUX.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "PTYMux::cleanup_terminal",
                info = %crate::inline_string!("Step 2: Waiting 100ms for processes to terminate gracefully")
            };
        });
        std::thread::sleep(std::time::Duration::from_millis(100));
        DEBUG_TUI_PTY_MUX.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "PTYMux::cleanup_terminal",
                info = %crate::inline_string!("Step 2 completed in {:?}", start_time.elapsed())
            };
        });

        // Force flush any pending output.
        DEBUG_TUI_PTY_MUX.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "PTYMux::cleanup_terminal",
                info = %crate::inline_string!("Step 3: Flushing pending output")
            };
        });
        self.output_device.write(|out| match out.flush() {
            Ok(()) => {
                crate::DEBUG_TUI_PTY_MUX.then(|| {
                    // % is Display, ? is Debug.
                    tracing::debug! {
                        message = "PTYMux::cleanup_terminal",
                        info = "Step 3: Output flush successful"
                    };
                });
            }
            Err(e) => {
                crate::DEBUG_TUI_PTY_MUX.then(|| {
                    // % is Display, ? is Debug.
                    tracing::warn! {
                        message = "PTYMux::cleanup_terminal",
                        info = %crate::inline_string!("Step 3: Output flush failed: {:?}", e)
                    };
                });
            }
        });
        DEBUG_TUI_PTY_MUX.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "PTYMux::cleanup_terminal",
                info = %crate::inline_string!("Step 3 completed in {:?}", start_time.elapsed())
            };
        });

        // Clear screen
        DEBUG_TUI_PTY_MUX.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "PTYMux::cleanup_terminal",
                info = %crate::inline_string!("Step 4: Clearing screen and homing cursor")
            };
        });
        self.output_device.write(|out| {
            let _unused = out.write_all(AnsiSequenceGenerator::clear_screen().as_bytes());
            let _unused = out.write_all(
                AnsiSequenceGenerator::cursor_position(row(0), col(0)).as_bytes(),
            );
            let _unused = out.flush();
        });
        DEBUG_TUI_PTY_MUX.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "PTYMux::cleanup_terminal",
                info = %crate::inline_string!("Step 4 completed in {:?}", start_time.elapsed())
            };
        });

        // Force flush after escape sequences.
        DEBUG_TUI_PTY_MUX.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "PTYMux::cleanup_terminal",
                info = %crate::inline_string!("Step 5: Final output flush after escape sequences")
            };
        });
        self.output_device.write(|out| match out.flush() {
            Ok(()) => {
                crate::DEBUG_TUI_PTY_MUX.then(|| {
                    // % is Display, ? is Debug.
                    tracing::debug! {
                        message = "PTYMux::cleanup_terminal",
                        info = "Step 5: Final flush successful"
                    };
                });
            }
            Err(e) => {
                crate::DEBUG_TUI_PTY_MUX.then(|| {
                    // % is Display, ? is Debug.
                    tracing::warn! {
                        message = "PTYMux::cleanup_terminal",
                        info = %crate::inline_string!("Step 5: Final flush failed: {:?}", e)
                    };
                });
            }
        });
        DEBUG_TUI_PTY_MUX.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "PTYMux::cleanup_terminal",
                info = %crate::inline_string!("Step 5 completed in {:?}", start_time.elapsed())
            };
        });

        // End raw mode
        DEBUG_TUI_PTY_MUX.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "PTYMux::cleanup_terminal",
                info = %crate::inline_string!("Step 6: Ending raw mode")
            };
        });
        self.output_device.teardown_full_screen_tui().ok();
        DEBUG_TUI_PTY_MUX.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "PTYMux::cleanup_terminal",
                info = %crate::inline_string!("Step 6: Raw mode ended successfully")
            };
        });

        let total_time = start_time.elapsed();
        DEBUG_TUI_PTY_MUX.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "PTYMux::cleanup_terminal",
                info = %crate::inline_string!("Cleanup completed successfully in {:?}", total_time)
            };
        });

        if total_time > std::time::Duration::from_millis(500) {
            DEBUG_TUI_PTY_MUX.then(|| {
                // % is Display, ? is Debug.
                tracing::warn! {
                    message = "PTYMux::cleanup_terminal",
                    info = %crate::inline_string!("Cleanup took longer than expected: {:?}", total_time)
                };
            });
        }

        // If cleanup took too long, there might be zombie processes.
        if total_time > std::time::Duration::from_secs(1) {
            DEBUG_TUI_PTY_MUX.then(|| {
                // % is Display, ? is Debug.
                tracing::error! {
                    message = "PTYMux::cleanup_terminal",
                    info = %crate::inline_string!("Cleanup took over 1 second; check for potential zombie processes")
                };
            });
        }
    }
}

pub mod adaptive_render_budget {
    #[allow(clippy::wildcard_imports)]
    use super::*;
    use crate::DEBUG_TUI_PTY_MUX;

    /// Default render speed is ~60 FPS.
    pub const DEFAULT_FRAME_DELAY_MS: Duration = Duration::from_millis(16);

    /// Slowest render speed is ~10 FPS.
    pub const MAX_FRAME_DELAY_MS: Duration = Duration::from_millis(100);

    /// Min FPS is uncapped. If the terminal is fast enough, there is no throttling and it
    /// will render as fast as possible.
    pub const MIN_FRAME_DELAY_MS: Duration = Duration::from_millis(0);

    /// Flush taking >5ms indicates pressure.
    pub const RENDER_TIME_BACKPRESSURE_THRESHOLD_MS: Duration =
        Duration::from_millis(RENDER_TIME_BASE * 5);

    // Asymmetric backoff - penalty is 5 x higher than reward. If we detect backpressure,
    // we backoff and hit the brakes hard.
    pub const THROTTLE_PENALTY_MS: Duration =
        Duration::from_millis(RENDER_TIME_BASE * 10);

    // Asymmetric backoff - reward is 5 x lower than reward. If we detect flushing is
    // getting faster, we are easy into the recovery, by getting back on the gas
    // gradually.
    pub const RECOVERY_REWARD_MS: Duration = Duration::from_millis(RENDER_TIME_BASE);

    const RENDER_TIME_BASE: u64 = 1;

    #[derive(Debug)]
    pub enum AdaptiveRenderResult {
        Skip,
        Render,
    }

    #[derive(Debug)]
    pub struct Budget {
        pub last_render_time: Instant,
        pub render_cooldown_delay: Duration,
        pub maybe_render_start: Option<Instant>,
    }

    impl Default for Budget {
        fn default() -> Self {
            Self {
                last_render_time: Instant::now(),
                render_cooldown_delay: DEFAULT_FRAME_DELAY_MS,
                maybe_render_start: None,
            }
        }
    }

    impl Budget {
        /// Decides if we should render this frame based on output and budget.
        pub fn should_render(
            &self,
            process_manager: &mut ProcessManager,
        ) -> AdaptiveRenderResult {
            let process_had_output = process_manager.poll_all_processes();
            if !process_had_output {
                return AdaptiveRenderResult::Skip;
            }
            let time_since_last_render = self.last_render_time.elapsed();
            if time_since_last_render >= self.render_cooldown_delay {
                AdaptiveRenderResult::Render
            } else {
                AdaptiveRenderResult::Skip
            }
        }

        /// Marks the start of a rendering pass. This timestamp is used to measure how
        /// long the rendering operation takes, which informs the adaptive budget
        /// calculation.
        ///
        /// # Panics
        ///
        /// Panics if called twice without an intervening [`mark_end()`] call, enforcing
        /// the strict [`mark_start()`] -> render -> [`mark_end()`] state machine.
        ///
        /// [`mark_end()`]: Self::mark_end
        /// [`mark_start()`]: Self::mark_start
        pub fn mark_start(&mut self) {
            assert!(
                self.maybe_render_start.is_none(),
                "Can't call mark_start() more than once"
            );
            self.maybe_render_start = Some(Instant::now());
        }

        /// Updates the budget based on how long the render actually took.
        ///
        /// # Panics
        ///
        /// Panics if called without a preceding [`mark_start()`] call, enforcing the
        /// strict [`mark_start()`] -> render -> [`mark_end()`] state machine.
        ///
        /// [`mark_end()`]: Self::mark_end
        /// [`mark_start()`]: Self::mark_start
        pub fn mark_end(&mut self) {
            // Mark the end of the render pass. This is how long a render pass took.
            let render_duration = self
                .maybe_render_start
                .take()
                .expect("Can't call mark_end() without calling mark_start() first")
                .elapsed();

            // Mark the current time as the last render time. This will be used in the
            // should_render() method.
            self.last_render_time = Instant::now();

            // Adjust budget dynamically based on detected back pressure. We are
            // implementing asymmetric backoff.
            let backpressure_detected =
                render_duration > RENDER_TIME_BACKPRESSURE_THRESHOLD_MS;
            if backpressure_detected {
                // Penalize render budget for backpressure.
                self.render_cooldown_delay = self
                    .render_cooldown_delay
                    .saturating_add(THROTTLE_PENALTY_MS)
                    .min(MAX_FRAME_DELAY_MS);

                DEBUG_TUI_PTY_MUX.then(|| {
                    // % is Display, ? is Debug.
                    tracing::info! {
                        message = "Budget::mark_end",
                        info = %crate::inline_string!(
                            "Render took {:?} (> {:?}). Throttling frame delay to {:?}",
                            render_duration,
                            RENDER_TIME_BACKPRESSURE_THRESHOLD_MS,
                            self.render_cooldown_delay
                        )
                    };
                });
            } else {
                // Used for logging.
                let old_delay = self.render_cooldown_delay;

                // Reward render budget for smooth rendering.
                self.render_cooldown_delay = self
                    .render_cooldown_delay
                    .saturating_sub(RECOVERY_REWARD_MS)
                    .max(MIN_FRAME_DELAY_MS);

                // Only log recovery if the delay actually changed to avoid spamming
                // the logs
                if old_delay != self.render_cooldown_delay {
                    DEBUG_TUI_PTY_MUX.then(|| {
                        // % is Display, ? is Debug.
                        tracing::info! {
                            message = "Budget::mark_end",
                            info = %crate::inline_string!(
                                "Render took {:?}. Recovering frame delay to {:?}",
                                render_duration,
                                self.render_cooldown_delay
                            )
                        };
                    });
                }
            }
        }
    }
}
