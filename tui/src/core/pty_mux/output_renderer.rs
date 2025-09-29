// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Dynamic display management for the PTY multiplexer.
//!
//! This module handles rendering output from the active process using `OffscreenBuffer`
//! as a compositor to eliminate visual artifacts. It maintains a dynamic status bar
//! showing process information and keyboard shortcuts.

use super::ProcessManager;
use crate::{ANSIBasicColor, ArrayOverflowResult, FlushKind, IndexOps, LengthOps,
            OffscreenBuffer, OutputDevice, PixelChar, Size, TuiColor, TuiStyle, col,
            core::units::{idx, len},
            lock_output_device_as_mut,
            tui::terminal_lib_backends::{OffscreenBufferPaint,
                                         OffscreenBufferPaintImplCrossterm},
            tui_style_attrib::Bold,
            tui_style_attribs};

/// Height reserved for the status bar at the bottom of the terminal.
pub const STATUS_BAR_HEIGHT: u16 = 1;

/// Maximum number of processes supported (F1-F9).
pub const MAX_PROCESSES: usize = 9;

/// Manages display rendering and status bar for the multiplexer with per-process buffers.
///
/// This renderer gets the active process's buffer from `ProcessManager` and composites
/// the status bar into it for final rendering. No longer maintains its own single buffer.
pub struct OutputRenderer {
    terminal_size: Size,
    // Removed: single offscreen_buffer - now uses per-process buffers
    // Removed: vte_parser - now handled per-process
    // Removed: first_output_seen - now handled per-process
}

impl std::fmt::Debug for OutputRenderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OutputRenderer")
            .field("terminal_size", &self.terminal_size)
            .finish()
    }
}

impl OutputRenderer {
    /// Create a new output renderer with the given terminal size.
    ///
    /// With per-process buffers, the renderer no longer maintains its own buffer
    /// or parser - it gets buffers from the `ProcessManager` when needed.
    #[must_use]
    pub fn new(terminal_size: Size) -> Self { Self { terminal_size } }

    /// Render the active process's buffer with status bar using per-process buffers.
    ///
    /// **Per-process buffer compositing**:
    /// This method demonstrates how the virtual terminal architecture works:
    /// 1. Get the active process's complete virtual terminal (`OffscreenBuffer`)
    /// 2. Clone it for compositing (preserves the original state)
    /// 3. Composite the status bar into the last row
    /// 4. Paint the entire composite to the real terminal atomically
    ///
    /// **Key benefits**:
    /// - The original process buffer is never modified (preserves state)
    /// - Status bar is overlaid without affecting the process's virtual terminal
    /// - Atomic painting eliminates visual artifacts
    /// - Works universally with all program types
    ///
    /// # Errors
    ///
    /// Returns an error if terminal output operations fail.
    pub fn render_from_active_buffer(
        &mut self,
        output_device: &OutputDevice,
        process_manager: &ProcessManager,
    ) -> miette::Result<()> {
        // Get the active process's buffer.
        let active_buffer = process_manager.get_active_buffer();

        // Clone the buffer for compositing (we don't modify the original)
        let mut composite_buffer = active_buffer.clone();

        // Composite status bar into the last row.
        self.composite_status_bar_into_buffer(&mut composite_buffer, process_manager);

        // Paint the composite buffer to terminal.
        self.paint_buffer(&composite_buffer, output_device);

        Ok(())
    }

    /// Composite status bar into the last row of the given `OffscreenBuffer`.
    ///
    /// This modifies the provided buffer by writing the status bar to its last row.
    fn composite_status_bar_into_buffer(
        &mut self,
        ofs_buf: &mut OffscreenBuffer,
        process_manager: &ProcessManager,
    ) {
        let status_text = self.generate_status_text(process_manager);
        let last_row_idx = self.terminal_size.row_height.as_usize().saturating_sub(1);

        // Clear status bar row.
        for col_idx in 0..self.terminal_size.col_width.as_usize() {
            ofs_buf[last_row_idx][col_idx] = PixelChar::Spacer;
        }

        // Write status text with appropriate style.
        let status_style = TuiStyle {
            attribs: tui_style_attribs(Bold),
            color_fg: Some(TuiColor::Basic(ANSIBasicColor::White)),
            color_bg: Some(TuiColor::Basic(ANSIBasicColor::Blue)),
            ..Default::default()
        };

        for (col_idx, ch) in status_text.chars().enumerate() {
            if self.terminal_size.col_width.is_overflowed_by(col(col_idx))
                == ArrayOverflowResult::Overflowed
            {
                break;
            }
            ofs_buf[last_row_idx][col_idx] = PixelChar::PlainText {
                display_char: ch,
                style: status_style,
            };
        }
    }

    /// Paint the given `OffscreenBuffer` to terminal using existing paint infrastructure.
    fn paint_buffer(&mut self, ofs_buf: &OffscreenBuffer, output_device: &OutputDevice) {
        let mut crossterm_impl = OffscreenBufferPaintImplCrossterm {};
        let render_ops = crossterm_impl.render(ofs_buf);

        crossterm_impl.paint(
            render_ops,
            FlushKind::JustFlush,
            self.terminal_size,
            lock_output_device_as_mut!(output_device),
            false, // is_mock = false
        );
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

            // Check if we have space for this tab.
            let tab_width = tab_text.chars().count();
            let new_width = current_width + tab_width;
            if self
                .terminal_size
                .col_width
                .is_overflowed_by(col(new_width))
                == ArrayOverflowResult::Overflowed
            {
                break;
            }

            status_parts.push(tab_text);
            current_width += tab_width;
        }

        // Show dynamic keyboard shortcuts based on process count.
        let process_count = process_manager.processes().len();
        let shortcuts = Self::generate_shortcuts_text(process_count);

        // Check if we have space for shortcuts.
        let shortcuts_width = shortcuts.chars().count();
        let total_width = current_width + shortcuts_width;
        if self
            .terminal_size
            .col_width
            .is_overflowed_by(col(total_width))
            == ArrayOverflowResult::Overflowed
        {
            return status_parts.join("");
        }
        status_parts.push(shortcuts);

        status_parts.join("")
    }

    /// Generate keyboard shortcuts text based on the number of processes.
    fn generate_shortcuts_text(process_count: usize) -> String {
        if process_count <= 4 {
            // For 1-4 processes, show explicit function keys.
            match process_count {
                1 => "  F1: Switch | Ctrl+Q: Quit".to_string(),
                2 => "  F1/F2: Switch | Ctrl+Q: Quit".to_string(),
                3 => "  F1/F2/F3: Switch | Ctrl+Q: Quit".to_string(),
                4 => "  F1/F2/F3/F4: Switch | Ctrl+Q: Quit".to_string(),
                _ => "  Ctrl+Q: Quit".to_string(),
            }
        } else {
            // For 5+ processes, show range notation.
            format!("  F1-F{}: Switch | Ctrl+Q: Quit", {
                let process_idx = idx(process_count);
                let max_display = len(9);
                process_idx.clamp_to_max_length(max_display).as_usize()
            })
        }
    }

    /// Render initial status bar on startup using `OffscreenBuffer` composition.
    ///
    /// # Errors
    ///
    /// Returns an error if terminal output operations fail.
    pub fn render_initial_status_bar(
        &mut self,
        output_device: &OutputDevice,
        process_manager: &ProcessManager,
    ) -> miette::Result<()> {
        // With per-process buffers, just render from the active buffer.
        self.render_from_active_buffer(output_device, process_manager)
    }

    /// Update the terminal size for the renderer.
    ///
    /// With per-process buffers, the renderer doesn't maintain its own buffer,
    /// so this just updates the terminal size for status bar compositing.
    pub fn update_terminal_size(&mut self, new_size: Size) {
        self.terminal_size = new_size;
    }
}
