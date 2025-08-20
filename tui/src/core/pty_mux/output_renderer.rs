// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Dynamic display management for the PTY multiplexer.
//!
//! This module handles rendering output from the active process using OffscreenBuffer
//! as a compositor to eliminate visual artifacts. It maintains a dynamic status bar
//! showing process information and keyboard shortcuts.


use miette::IntoDiagnostic;
use vte::Parser;

use super::{ProcessManager, ProcessOutput, ansi_parser::AnsiToBufferProcessor};
use crate::{idx, lock_output_device_as_mut, tui_style_attrib, ANSIBasicColor, FlushKind, OffscreenBuffer, PixelChar, RingBuffer, RingBufferStack, Size, TuiColor, TuiStyle, OutputDevice,
            tui::terminal_lib_backends::{OffscreenBufferPaint, OffscreenBufferPaintImplCrossterm}};

/// Height reserved for the status bar at the bottom of the terminal.
pub const STATUS_BAR_HEIGHT: u16 = 1;

/// Maximum number of processes supported (F1-F9).
pub const MAX_PROCESSES: usize = 9;


/// Manages display rendering and status bar for the multiplexer using OffscreenBuffer composition.
pub struct OutputRenderer {
    terminal_size: Size,
    offscreen_buffer: OffscreenBuffer,
    vte_parser: Parser,
    first_output_seen: RingBufferStack<(), MAX_PROCESSES>, // None=false, Some(())=true
}

impl std::fmt::Debug for OutputRenderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OutputRenderer")
            .field("terminal_size", &self.terminal_size)
            .field("offscreen_buffer", &self.offscreen_buffer)
            .field("vte_parser", &"<Parser>")
            .field("first_output_seen", &self.first_output_seen)
            .finish()
    }
}

impl OutputRenderer {
    /// Create a new output renderer with the given terminal size.
    #[must_use]
    pub fn new(terminal_size: Size) -> Self {
        Self {
            terminal_size,
            offscreen_buffer: OffscreenBuffer::new_with_capacity_initialized(terminal_size),
            vte_parser: Parser::new(),
            // RingBufferStack initializes with all None by default (all false)
            first_output_seen: RingBufferStack::new(),
        }
    }

    /// Render output from the process manager using OffscreenBuffer composition.
    ///
    /// This processes PTY output through ANSI parsing into an OffscreenBuffer,
    /// composites the status bar, and then paints the entire buffer atomically
    /// to eliminate visual artifacts.
    ///
    /// # Errors
    ///
    /// Returns an error if terminal output operations fail.
    pub fn render(
        &mut self,
        output: ProcessOutput,
        output_device: &OutputDevice,
        process_manager: &ProcessManager,
    ) -> miette::Result<()>
    {
        match output {
            ProcessOutput::Active(data) => {
                let active_index = process_manager.active_index();

                // Clear buffer on first output from this process
                if self.first_output_seen.get(idx(active_index)).is_none() {
                    self.offscreen_buffer.clear();
                    // Mark as seen by setting to Some(())
                    self.first_output_seen.set(idx(active_index), ());
                }

                // Process PTY output through ANSI parser into OffscreenBuffer
                self.process_pty_output(&data)?;

                // Composite status bar into buffer (last row)
                self.composite_status_bar(process_manager);

                // Paint buffer to terminal using existing paint infrastructure
                self.paint_buffer(output_device)?;
            }
            ProcessOutput::ProcessSwitch {
                from: _from,
                to: to_index,
            } => {
                // Clear buffer for new process
                self.offscreen_buffer.clear();

                // Mark as first output for new process
                self.first_output_seen.set(idx(to_index), ()); // Reset to "not seen" - will be None until set

                // Clear terminal screen for process switch
                {
                    let locked_output_device = lock_output_device_as_mut!(output_device);
                    write!(locked_output_device, "{}{}", 
                           crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
                           crossterm::cursor::MoveTo(0, 0)).into_diagnostic()?;
                }

                // Render initial status bar
                self.composite_status_bar(process_manager);
                self.paint_buffer(output_device)?;
            }
        }
        Ok(())
    }

    /// Process PTY output through ANSI parser and update OffscreenBuffer.
    fn process_pty_output(&mut self, data: &[u8]) -> miette::Result<()> {
        let mut processor = AnsiToBufferProcessor::new(&mut self.offscreen_buffer);

        for &byte in data {
            self.vte_parser.advance(&mut processor, byte);
        }

        // Update buffer cursor position from processor
        self.offscreen_buffer.my_pos = processor.cursor_pos();
        Ok(())
    }

    /// Composite status bar into the last row of OffscreenBuffer.
    fn composite_status_bar(&mut self, process_manager: &ProcessManager) {
        let status_text = self.generate_status_text(process_manager);
        let last_row_idx = self.terminal_size.row_height.as_usize().saturating_sub(1);

        // Clear status bar row
        for col_idx in 0..self.terminal_size.col_width.as_usize() {
            self.offscreen_buffer.buffer[last_row_idx][col_idx] = PixelChar::Spacer;
        }

        // Write status text with appropriate style
        let status_style = Some(TuiStyle {
            id: None,
            bold: Some(tui_style_attrib::Bold),
            italic: None,
            dim: None,
            underline: None,
            reverse: None,
            hidden: None,
            strikethrough: None,
            computed: None,
            color_fg: Some(TuiColor::Basic(ANSIBasicColor::White)),
            color_bg: Some(TuiColor::Basic(ANSIBasicColor::Blue)),
            padding: None,
            lolcat: None,
        });

        for (col_idx, ch) in status_text.chars().enumerate() {
            if col_idx >= self.terminal_size.col_width.as_usize() {
                break;
            }
            self.offscreen_buffer.buffer[last_row_idx][col_idx] = PixelChar::PlainText {
                display_char: ch,
                maybe_style: status_style,
            };
        }
    }

    /// Paint OffscreenBuffer to terminal using existing paint infrastructure.
    fn paint_buffer(&mut self, output_device: &OutputDevice) -> miette::Result<()>
    {
        let mut crossterm_impl = OffscreenBufferPaintImplCrossterm {};
        let render_ops = crossterm_impl.render(&self.offscreen_buffer);

        crossterm_impl.paint(
            render_ops,
            FlushKind::JustFlush,
            self.terminal_size,
            lock_output_device_as_mut!(output_device),
            false, // is_mock = false
        );

        Ok(())
    }

    /// Generate the complete status bar text with process tabs and shortcuts.
    fn generate_status_text(&self, process_manager: &ProcessManager) -> String {
        let mut status_parts = Vec::new();

        // Show process tabs with live status indicators: 1:[🟢claude] 2:[🔴btop] etc.
        let mut current_width = 0usize;
        for (i, process) in process_manager.processes().iter().enumerate() {
            let is_active = i == process_manager.active_index();
            let status_indicator = if process.is_running() { "🟢" } else { "🔴" };

            let tab_text = if is_active {
                format!(" [{}:{}{}] ", i + 1, status_indicator, process.name)
            } else {
                format!(" {}:{}{} ", i + 1, status_indicator, process.name)
            };

            // Check if we have space for this tab
            let tab_width = tab_text.chars().count();
            if current_width + tab_width > self.terminal_size.col_width.as_usize() {
                break;
            }

            status_parts.push(tab_text);
            current_width += tab_width;
        }

        // Show dynamic keyboard shortcuts based on process count
        let process_count = process_manager.processes().len();
        let shortcuts = Self::generate_shortcuts_text(process_count);

        // Check if we have space for shortcuts
        let available_width = self.terminal_size.col_width.as_usize().saturating_sub(current_width);
        if shortcuts.chars().count() <= available_width {
            status_parts.push(shortcuts);
        }

        status_parts.join("")
    }

    /// Generate keyboard shortcuts text based on the number of processes.
    fn generate_shortcuts_text(process_count: usize) -> String {
        if process_count <= 4 {
            // For 1-4 processes, show explicit function keys
            match process_count {
                1 => "  F1: Switch | Ctrl+Q: Quit".to_string(),
                2 => "  F1/F2: Switch | Ctrl+Q: Quit".to_string(),
                3 => "  F1/F2/F3: Switch | Ctrl+Q: Quit".to_string(),
                4 => "  F1/F2/F3/F4: Switch | Ctrl+Q: Quit".to_string(),
                _ => "  Ctrl+Q: Quit".to_string(),
            }
        } else {
            // For 5+ processes, show range notation
            format!(
                "  F1-F{}: Switch | Ctrl+Q: Quit",
                std::cmp::min(process_count, 9)
            )
        }
    }

    /// Render initial status bar on startup using OffscreenBuffer composition.
    ///
    /// # Errors
    ///
    /// Returns an error if terminal output operations fail.
    pub fn render_initial_status_bar(
        &mut self,
        output_device: &OutputDevice,
        process_manager: &ProcessManager,
    ) -> miette::Result<()>
    {
        // Clear buffer first
        self.offscreen_buffer.clear();

        // Clear terminal screen
        {
            let locked_output_device = lock_output_device_as_mut!(output_device);
            write!(locked_output_device, "{}{}", 
                   crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
                   crossterm::cursor::MoveTo(0, 0)).into_diagnostic()?;
        }

        // Render status bar into buffer and paint
        self.composite_status_bar(process_manager);
        self.paint_buffer(output_device)?;
        Ok(())
    }

    /// Update the terminal size for the renderer and recreate OffscreenBuffer.
    pub fn update_terminal_size(&mut self, new_size: Size) {
        self.terminal_size = new_size;
        self.offscreen_buffer = OffscreenBuffer::new_with_capacity_initialized(new_size);
    }

}
