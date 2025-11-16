// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Main PTY multiplexer orchestrator.
//!
//! This module provides the main `PTYMux` struct that coordinates all components
//! and manages the event loop for the terminal multiplexer.

use super::{InputRouter, OutputRenderer, Process, ProcessManager, output_renderer};
use crate::{InputEvent, RawMode, Size, clear_screen_and_home_cursor,
            core::{get_size,
                   osc::OscController,
                   pty::pty_core::pty_sessions::show_notification,
                   terminal_io::{InputDevice, OutputDevice}},
            lock_output_device_as_mut};

/// Main PTY multiplexer that orchestrates all components.
pub struct PTYMux {
    process_manager: ProcessManager,
    input_router: InputRouter,
    output_renderer: OutputRenderer,
    terminal_size: Size,
    output_device: OutputDevice,
    input_device: InputDevice,
}

impl std::fmt::Debug for PTYMux {
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

/// Builder for configuring and creating a `PTYMux` instance.
#[derive(Default, Debug)]
pub struct PTYMuxBuilder {
    processes: Vec<Process>,
}

impl PTYMuxBuilder {
    /// Set the processes to be managed by the multiplexer.
    #[must_use]
    pub fn processes(mut self, processes: Vec<Process>) -> Self {
        self.processes = processes;
        self
    }

    /// Add a single process to the multiplexer.
    #[must_use]
    pub fn add_process(mut self, process: Process) -> Self {
        self.processes.push(process);
        self
    }

    /// Build the `PTYMux` instance.
    ///
    /// # Errors
    ///
    /// Returns an error if no processes are configured or terminal setup fails.
    pub fn build(self) -> miette::Result<PTYMux> {
        if self.processes.is_empty() {
            miette::bail!("At least one process must be configured");
        }

        if self.processes.len() > output_renderer::MAX_PROCESSES {
            miette::bail!(
                "Maximum of {} processes allowed",
                output_renderer::MAX_PROCESSES
            );
        }

        let terminal_size = get_size()?;
        let output_device = OutputDevice::new_stdout();
        let input_device = InputDevice::default();

        Ok(PTYMux {
            process_manager: ProcessManager::new(self.processes, terminal_size),
            input_router: InputRouter::new(),
            output_renderer: OutputRenderer::new(terminal_size),
            terminal_size,
            output_device,
            input_device,
        })
    }
}

impl PTYMux {
    /// Create a new builder for configuring a `PTYMux` instance.
    #[must_use]
    pub fn builder() -> PTYMuxBuilder { PTYMuxBuilder::default() }

    /// Run the multiplexer event loop.
    ///
    /// This is the main entry point that:
    /// 1. Starts raw mode
    /// 2. Sets initial terminal title
    /// 3. Runs the main event loop
    /// 4. Cleans up on exit
    ///
    /// # Errors
    ///
    /// Returns an error if terminal setup, process management, or event handling fails.
    pub async fn run(mut self) -> miette::Result<()> {
        // Start raw mode using existing RawMode.
        RawMode::start(
            self.terminal_size,
            lock_output_device_as_mut!(&self.output_device),
            false,
        );
        tracing::debug!("Raw mode started");

        // Set initial terminal title using OSC controller.
        {
            let mut osc = OscController::new(&self.output_device);
            osc.set_title_and_tab("PTYMux Example - Starting")?;
        }

        // Start all processes at startup.
        tracing::debug!("Starting all processes");
        self.process_manager.start_all_processes()?;

        // Clear screen before showing first process.
        clear_screen_and_home_cursor(&self.output_device);

        // Trigger initial process switch to show first process.
        self.process_manager.switch_to(0);

        // Render initial status bar.
        self.output_renderer
            .render_initial_status_bar(&self.output_device, &self.process_manager)?;

        // Main event loop
        tracing::debug!("Starting main event loop");
        let result = self.run_event_loop().await;
        tracing::debug!("Main event loop exited with result: {:?}", result);

        // Always cleanup regardless of error.
        self.cleanup_terminal();

        result
    }

    /// Main event loop that handles input and output events.
    async fn run_event_loop(&mut self) -> miette::Result<()> {
        // Create a periodic timer for status bar updates.
        let mut status_bar_interval =
            tokio::time::interval(tokio::time::Duration::from_millis(500));

        // Create a fast timer for polling PTY output.
        let mut output_poll_interval =
            tokio::time::interval(tokio::time::Duration::from_millis(10));

        'main_loop: loop {
            tokio::select! {
                // Poll ALL processes and update their virtual terminal buffers.
                // https://developerlife.com/2024/07/10/rust-async-cancellation-safety-tokio/#example-1-right-and-wrong-way-to-sleep-and-interval
                _ = output_poll_interval.tick() => {
                    // **Core of per-process virtual terminal architecture**:
                    // Poll ALL processes continuously (every 10ms), not just the active one.
                    // Each process updates its own OffscreenBuffer independently.
                    // This is what enables instant switching with full state preservation.
                    let active_had_output = self.process_manager.poll_all_processes();

                    // **Selective rendering optimization**:
                    // Only render when the currently visible process has new output.
                    // All other processes continue updating their virtual terminals.
                    // in the background, ready for instant switching.
                    if active_had_output {
                        // Get the active process's virtual terminal and render it.
                        self.output_renderer.render_from_active_buffer(
                            &self.output_device,
                            &self.process_manager
                        )?;

                        // Clear the "needs rendering" flag for the active process.
                        self.process_manager.mark_active_as_rendered();
                    }
                }

                // Handle user input using existing InputDevice.
                Some(input_event) = self.input_device.next() => {
                    tracing::debug!("Received input event: {:?}", input_event);

                    // Show desktop notification for input event (filter out mouse events)
                    if !matches!(input_event, InputEvent::Mouse(_)) {
                        show_notification("PTY Mux - Input Event", &format!("Input event received: {input_event:?}"));
                    }

                    // Create OSC controller for this input handling.
                    let mut osc = OscController::new(&self.output_device);

                    // Handle input events using the input router.
                    tracing::debug!("Handling input event: {:?}", input_event);
                    let should_exit = self.input_router.handle_input(
                        input_event,
                        &mut self.process_manager,
                        &mut osc,
                        &self.output_device
                    )?;

                    if should_exit {
                        tracing::debug!("Exit requested by input router - breaking main event loop");
                        break 'main_loop; // Exit requested - break out of the loop
                    }
                }

                // Periodic status bar updates - ensures status bar is visible even when idle.
                _ = status_bar_interval.tick() => {
                    self.output_renderer.render_initial_status_bar(&self.output_device, &self.process_manager)?;
                }
            }
        }
        tracing::debug!("Event loop completed - returning Ok(())");
        Ok(())
    }

    /// Update terminal size for all components.
    ///
    /// This method is called when the terminal is resized and ensures
    /// all components are aware of the new size.
    pub fn update_terminal_size(&mut self, new_size: Size) {
        self.terminal_size = new_size;
        self.process_manager.handle_terminal_resize(new_size);
        self.output_renderer.update_terminal_size(new_size);
    }

    /// Get the current terminal size.
    #[must_use]
    pub fn terminal_size(&self) -> Size { self.terminal_size }

    /// Get a reference to the process manager.
    #[must_use]
    pub fn process_manager(&self) -> &ProcessManager { &self.process_manager }

    /// Get a mutable reference to the process manager.
    pub fn process_manager_mut(&mut self) -> &mut ProcessManager {
        &mut self.process_manager
    }

    /// Cleanup terminal state - always called on exit.
    fn cleanup_terminal(&mut self) {
        let start_time = std::time::Instant::now();
        tracing::debug!("Starting cleanup - terminal size: {:?}", self.terminal_size);

        // First, kill all running processes.
        tracing::debug!("Step 1: Shutting down process manager");
        self.process_manager.shutdown_all_processes();
        tracing::debug!("Step 1 completed in {:?}", start_time.elapsed());

        // Give processes a short time to terminate gracefully, then force exit.
        tracing::debug!("Step 2: Waiting 100ms for processes to terminate gracefully");
        std::thread::sleep(std::time::Duration::from_millis(100));
        tracing::debug!("Step 2 completed in {:?}", start_time.elapsed());

        // Force flush any pending output.
        tracing::debug!("Step 3: Flushing pending output");
        match lock_output_device_as_mut!(&self.output_device).flush() {
            Ok(()) => tracing::debug!("Step 3: Output flush successful"),
            Err(e) => tracing::warn!("Step 3: Output flush failed: {:?}", e),
        }
        tracing::debug!("Step 3 completed in {:?}", start_time.elapsed());

        // Clear screen
        tracing::debug!("Step 4: Clearing screen and homing cursor");
        clear_screen_and_home_cursor(&self.output_device);
        tracing::debug!("Step 4 completed in {:?}", start_time.elapsed());

        // Force flush after escape sequences.
        tracing::debug!("Step 5: Final output flush after escape sequences");
        match lock_output_device_as_mut!(&self.output_device).flush() {
            Ok(()) => tracing::debug!("Step 5: Final flush successful"),
            Err(e) => tracing::warn!("Step 5: Final flush failed: {:?}", e),
        }
        tracing::debug!("Step 5 completed in {:?}", start_time.elapsed());

        // End raw mode
        tracing::debug!("Step 6: Ending raw mode");
        RawMode::end(
            self.terminal_size,
            lock_output_device_as_mut!(&self.output_device),
            false,
        );
        tracing::debug!("Step 6: Raw mode ended successfully");

        let total_time = start_time.elapsed();
        tracing::debug!("Cleanup completed successfully in {:?}", total_time);

        if total_time > std::time::Duration::from_millis(500) {
            tracing::warn!("Cleanup took longer than expected: {:?}", total_time);
        }

        // If cleanup took too long, there might be zombie processes.
        // Force exit to prevent hanging.
        if total_time > std::time::Duration::from_millis(1000) {
            tracing::error!("Cleanup took over 1 second, forcing exit to prevent hang");
            std::process::exit(0);
        }
    }
}
