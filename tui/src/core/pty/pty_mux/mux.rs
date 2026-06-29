// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Main [`PTY`] multiplexer orchestrator.
//!
//! This module provides the main [`PTYMux`] struct that coordinates all components and
//! manages the event loop for the terminal multiplexer.
//!
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal

use super::{InputRouter, OutputRenderer, Process, ProcessManager};
use crate::{AnsiSequenceGenerator, Continuation, DEBUG_TUI_PTY_MUX, EventPropagation,
            InputEvent, Size, TerminalInteractiveStatus, TuiAvailability, col,
            core::{check_is_terminal_interactive, emit_stderr_redirection_disclaimer,
                   get_size,
                   osc::OscController,
                   pty::pty_mux::{AdaptiveRenderResult::{Render, Skip},
                                  Budget},
                   terminal_io::{InputDevice, OutputDevice, TerminalModeController}},
            format_no_alloc, ok,
            pty_mux::output_renderer::MAX_PROCESSES,
            row};
use std::{fmt::Debug,
          thread::sleep,
          time::{Duration, Instant}};
use tokio::time::interval;

pub const STATUS_BAR_UPDATE_INTERVAL_MS: u64 = 500;
pub const OUTPUT_POLL_INTERVAL_MS: u64 = 10;

/// Builder for configuring and creating a [`PTYMux`] instance.
///
/// This struct follows the builder pattern to configure the initial state of the terminal
/// multiplexer before it begins running. You can use it to set up the underlying
/// processes to spawn, define the virtual terminal size, and attach an optional
/// [`InputInterceptorFn`] to handle global hotkeys.
///
/// Once configuration is complete, call [`build()`] to construct the actual [`PTYMux`]
/// engine.
///
/// # Examples
///
/// ```rust
/// use r3bl_tui::pty_mux::PTYMux;
///
/// let builder = PTYMux::builder()
///     .add_process("htop", "htop", vec![])
///     .add_process("bash", "bash", vec![]);
/// ```
///
/// [`build()`]: PTYMuxBuilder::build
#[derive(Default)]
pub struct PTYMuxBuilder {
    process_configs: Vec<(String, String, Vec<String>)>,
    terminal_size: Option<Size>,
    maybe_input_interceptor_fn: Option<Box<InputInterceptorFn>>,
}

/// Type alias for the input interceptor closure.
///
/// This closure is executed on every [`InputEvent`] before it is routed to the currently
/// active process. It is primarily used to define global hotkeys (e.g., for switching
/// panes or exiting the multiplexer).
///
/// Because it implements [`FnMut`], it can capture and mutate variables from its
/// surrounding environment. It also receives a mutable reference to the
/// [`ProcessManager`], which allows it to change the active process index or inspect
/// running processes.
///
/// The closure must return an [`EventPropagation`] to indicate whether the event should
/// continue to the active process.
///
/// [`EventPropagation`]: crate::EventPropagation
/// [`InputEvent`]: crate::InputEvent
/// [`ProcessManager`]: super::ProcessManager
pub type InputInterceptorFn =
    dyn FnMut(&InputEvent, &mut ProcessManager) -> EventPropagation;

mod impl_pty_mux_builder {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Debug for PTYMuxBuilder {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("PTYMuxBuilder")
                .field("process_configs", &self.process_configs)
                .field("terminal_size", &self.terminal_size)
                .field(
                    "maybe_input_interceptor_fn",
                    &if self.maybe_input_interceptor_fn.is_some() {
                        "Some(...)"
                    } else {
                        "None"
                    },
                )
                .finish()
        }
    }

    impl PTYMuxBuilder {
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

        /// Sets the input interceptor for handling custom global shortcuts.
        #[must_use]
        pub fn input_interceptor_fn(
            mut self,
            interceptor: Box<InputInterceptorFn>,
        ) -> Self {
            self.maybe_input_interceptor_fn = Some(interceptor);
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
        /// [`MAX_PROCESSES`]: super::MAX_PROCESSES
        #[must_use]
        pub fn build(self) -> TuiAvailability<PTYMux> {
            if self.process_configs.is_empty() {
                return TuiAvailability::Broken(miette::miette!(
                    "At least one process must be configured"
                ));
            }

            if self.process_configs.len() > MAX_PROCESSES {
                return TuiAvailability::Broken(miette::miette!(
                    "Maximum of {} processes allowed",
                    MAX_PROCESSES
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
                            process_manager: ProcessManager::new(
                                processes,
                                terminal_size,
                            ),
                            input_router: InputRouter::new(),
                            output_renderer: OutputRenderer::new(terminal_size),
                            terminal_size,
                            output_device,
                            input_device,
                            maybe_input_interceptor_fn: self.maybe_input_interceptor_fn,
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
    maybe_input_interceptor_fn: Option<Box<InputInterceptorFn>>,
}

mod impl_pty_mux {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Debug for PTYMux {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("PTYMux")
                .field("process_manager", &self.process_manager)
                .field("input_router", &self.input_router)
                .field("output_renderer", &self.output_renderer)
                .field("terminal_size", &self.terminal_size)
                .field("output_device", &"<OutputDevice>")
                .field("input_device", &"<InputDevice>")
                .field(
                    "input_interceptor",
                    &if self.maybe_input_interceptor_fn.is_some() {
                        "Some(...)"
                    } else {
                        "None"
                    },
                )
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
        /// [`stderr`] is redirected, the user is notified that application logs are
        /// handled internally.
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
            let _fullscreen_tui_mode_guard =
                self.output_device.setup_full_screen_tui()?;

            DEBUG_TUI_PTY_MUX.then(|| {
                // % is Display, ? is Debug.
                tracing::debug! {
                    message = "PTYMux::run",
                    info = "Raw mode started"
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
                    info = "Starting all processes"
                };
            });
            self.process_manager.start_all_processes()?;

            // Clear screen before showing first process.
            self.output_device.write(|out| {
                let _unused =
                    out.write_all(AnsiSequenceGenerator::clear_screen().as_bytes());
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
                    info = "Starting main event loop"
                };
            });
            let result = self.run_event_loop().await;
            DEBUG_TUI_PTY_MUX.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "PTYMux::run",
                info = %format!("Main event loop exited with result: {result:?}")
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
            // Create a periodic timer for status bar updates.
            let mut status_bar_interval =
                interval(Duration::from_millis(STATUS_BAR_UPDATE_INTERVAL_MS));

            // Create a fast timer for polling PTY output.
            let mut output_poll_interval =
                interval(Duration::from_millis(OUTPUT_POLL_INTERVAL_MS));

            let mut render_budget = Budget::default();

            // Reusable String to avoid allocating a new one for the title on every tick.
            let mut title_buffer =
                String::with_capacity(self.terminal_size.col_width.as_usize());

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
                                        info = %format!(
                                            "Current frame delay: {:?}",
                                            render_budget.render_cooldown_delay
                                        )
                                    };
                                });
                            },

                        }
                    }

                    // Handle user input using existing InputDevice.
                    Some(input_event) = self.input_device.next() => {
                        DEBUG_TUI_PTY_MUX.then(|| {
                            // % is Display, ? is Debug.
                            tracing::debug! {
                                message = "PTYMux::run_event_loop",
                                info = %format!(
                                    "Received input event: {:?}",
                                    input_event
                                )
                            };
                        });

                        if let Some(interceptor) = &mut self.maybe_input_interceptor_fn {
                            match interceptor(&input_event, &mut self.process_manager) {
                                EventPropagation::Propagate => {}
                                EventPropagation::Consumed | EventPropagation::ConsumedRender => continue,
                                EventPropagation::ExitMainEventLoop => break,
                            }
                        }

                        // Create OSC controller for this input handling.

                        // Handle input events using the input router.
                        DEBUG_TUI_PTY_MUX.then(|| {
                            // % is Display, ? is Debug.
                            tracing::debug! {
                                message = "PTYMux::run_event_loop",
                                info = %format!(
                                    "Handling input event: {:?}",
                                    input_event
                                )
                            };
                        });
                        let continuation = self.input_router.handle_input(
                            input_event,
                            &mut self.process_manager,
                        )?;

                        if continuation == Continuation::Stop {
                            DEBUG_TUI_PTY_MUX.then(|| {
                                // % is Display, ? is Debug.
                                tracing::debug! {
                                    message = "PTYMux::run_event_loop",
                                    info = "Exit requested by input router - breaking main event loop"
                                };
                            });
                            break;
                        }
                    }

                    // Periodic status bar updates - ensures status bar is visible even when idle.
                    _ = status_bar_interval.tick() => {
                        self.output_renderer.render_initial_status_bar(&self.output_device, &self.process_manager)?;
                        let mut osc = OscController::new(&self.output_device);
                        self.update_terminal_title(&mut osc, &mut title_buffer)?;
                    }
                }
            }

            DEBUG_TUI_PTY_MUX.then(|| {
                // % is Display, ? is Debug.
                tracing::debug! {
                    message = "PTYMux::run_event_loop",
                    info = "Event loop completed - returning Ok(())"
                };
            });

            ok!()
        }

        /// Updates the terminal title based on the currently active process.
        fn update_terminal_title(
            &self,
            osc: &mut OscController<'_>,
            title_buffer: &mut String,
        ) -> miette::Result<()> {
            // Check if the active process has set a custom terminal title.
            if let Some(custom_title) = self.process_manager.active_terminal_title() {
                // Use the process's custom title.
                format_no_alloc!(
                    title_buffer,
                    "PTYMux - {} - {}",
                    self.process_manager.active_name(),
                    custom_title
                );
            } else {
                // Use default title with just process name.
                format_no_alloc!(
                    title_buffer,
                    "PTYMux - {}",
                    self.process_manager.active_name()
                );
            }

            osc.set_title_and_tab(title_buffer)?;
            ok!()
        }

        /// Updates terminal size for all components.
        ///
        /// This method is called when the terminal is resized and ensures all components
        /// are aware of the new size.
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
            let start_time = Instant::now();
            DEBUG_TUI_PTY_MUX.then(|| {
                // % is Display, ? is Debug.
                tracing::debug! {
                    message = "PTYMux::cleanup_terminal",
                    info = %format!(
                        "Starting cleanup - terminal size: {:?}",
                        self.terminal_size)
                };
            });

            // First, kill all running processes.
            DEBUG_TUI_PTY_MUX.then(|| {
                // % is Display, ? is Debug.
                tracing::debug! {
                    message = "PTYMux::cleanup_terminal",
                    info = "Step 1: Shutting down process manager"
                };
            });
            self.process_manager.shutdown_all_processes();
            DEBUG_TUI_PTY_MUX.then(|| {
                // % is Display, ? is Debug.
                tracing::debug! {
                    message = "PTYMux::cleanup_terminal",
                    info = %format!("Step 1 completed in {:?}", start_time.elapsed())
                };
            });

            // Give processes a short time to terminate gracefully, then force exit.
            DEBUG_TUI_PTY_MUX.then(|| {
                // % is Display, ? is Debug.
                tracing::debug! {
                    message = "PTYMux::cleanup_terminal",
                    info = "Step 2: Waiting 100ms for processes to terminate gracefully"
                };
            });
            sleep(Duration::from_millis(100));
            DEBUG_TUI_PTY_MUX.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "PTYMux::cleanup_terminal",
                info = %format!("Step 2 completed in {:?}", start_time.elapsed())
            };
        });

            // Force flush any pending output.
            DEBUG_TUI_PTY_MUX.then(|| {
                // % is Display, ? is Debug.
                tracing::debug! {
                    message = "PTYMux::cleanup_terminal",
                    info = "Step 3: Flushing pending output"
                };
            });
            self.output_device.write(|out| match out.flush() {
                Ok(()) => {
                    DEBUG_TUI_PTY_MUX.then(|| {
                        // % is Display, ? is Debug.
                        tracing::debug! {
                            message = "PTYMux::cleanup_terminal",
                            info = "Step 3: Output flush successful"
                        };
                    });
                }
                Err(e) => {
                    DEBUG_TUI_PTY_MUX.then(|| {
                        // % is Display, ? is Debug.
                        tracing::warn! {
                            message = "PTYMux::cleanup_terminal",
                            info = %format!("Step 3: Output flush failed: {:?}", e)
                        };
                    });
                }
            });
            DEBUG_TUI_PTY_MUX.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "PTYMux::cleanup_terminal",
                info = %format!("Step 3 completed in {:?}", start_time.elapsed())
            };
        });

            // Clear screen
            DEBUG_TUI_PTY_MUX.then(|| {
                // % is Display, ? is Debug.
                tracing::debug! {
                    message = "PTYMux::cleanup_terminal",
                    info = "Step 4: Clearing screen and homing cursor"
                };
            });
            self.output_device.write(|out| {
                let _unused =
                    out.write_all(AnsiSequenceGenerator::clear_screen().as_bytes());
                let _unused = out.write_all(
                    AnsiSequenceGenerator::cursor_position(row(0), col(0)).as_bytes(),
                );
                let _unused = out.flush();
            });
            DEBUG_TUI_PTY_MUX.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "PTYMux::cleanup_terminal",
                info = %format!("Step 4 completed in {:?}", start_time.elapsed())
            };
        });

            // Force flush after escape sequences.
            DEBUG_TUI_PTY_MUX.then(|| {
                // % is Display, ? is Debug.
                tracing::debug! {
                    message = "PTYMux::cleanup_terminal",
                    info = "Step 5: Final output flush after escape sequences"
                };
            });
            self.output_device.write(|out| match out.flush() {
                Ok(()) => {
                    DEBUG_TUI_PTY_MUX.then(|| {
                        // % is Display, ? is Debug.
                        tracing::debug! {
                            message = "PTYMux::cleanup_terminal",
                            info = "Step 5: Final flush successful"
                        };
                    });
                }
                Err(e) => {
                    DEBUG_TUI_PTY_MUX.then(|| {
                        // % is Display, ? is Debug.
                        tracing::warn! {
                            message = "PTYMux::cleanup_terminal",
                            info = %format!("Step 5: Final flush failed: {:?}", e)
                        };
                    });
                }
            });
            DEBUG_TUI_PTY_MUX.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "PTYMux::cleanup_terminal",
                info = %format!("Step 5 completed in {:?}", start_time.elapsed())
            };
        });

            // End raw mode
            DEBUG_TUI_PTY_MUX.then(|| {
                // % is Display, ? is Debug.
                tracing::debug! {
                    message = "PTYMux::cleanup_terminal",
                    info = "Step 6: Ending raw mode"
                };
            });
            self.output_device.teardown_full_screen_tui().ok();
            DEBUG_TUI_PTY_MUX.then(|| {
                // % is Display, ? is Debug.
                tracing::debug! {
                    message = "PTYMux::cleanup_terminal",
                    info = "Step 6: Raw mode ended successfully"
                };
            });

            let total_time = start_time.elapsed();
            DEBUG_TUI_PTY_MUX.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "PTYMux::cleanup_terminal",
                info = %format!("Cleanup completed successfully in {:?}", total_time)
            };
        });

            if total_time > Duration::from_millis(500) {
                DEBUG_TUI_PTY_MUX.then(|| {
                // % is Display, ? is Debug.
                tracing::warn! {
                    message = "PTYMux::cleanup_terminal",
                    info = %format!("Cleanup took longer than expected: {:?}", total_time)
                };
            });
            }

            // If cleanup took too long, there might be zombie processes.
            if total_time > Duration::from_secs(1) {
                DEBUG_TUI_PTY_MUX.then(|| {
                // % is Display, ? is Debug.
                tracing::error! {
                    message = "PTYMux::cleanup_terminal",
                    info = "Cleanup took over 1 second; check for potential zombie processes"
                };
            });
            }
        }
    }
}
