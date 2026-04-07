// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Dynamic display management for the [`PTY`] multiplexer. See [`OutputRenderer`] for
//! details.
//!
//! This module handles rendering output from the active process using [`OffscreenBuffer`]
//! as a compositor to eliminate visual artifacts. It maintains a dynamic status bar
//! showing process information and keyboard shortcuts.
//!
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal

use super::ProcessManager;
use crate::{FlushKind, GCStringOwned, IndexOps, OffscreenBuffer, OutputDevice, PixelChar, Pos, RenderOpsLocalData, SPACE_CHAR, Size, TuiStyle, col, core::coordinates::{idx, len}, lock_output_device_as_mut, ok, print_text_with_attributes, row, tui::terminal_lib_backends::{OffscreenBufferPaint, OffscreenBufferPaintImpl}, tui_color, tui_style_attrib::Bold, tui_style_attribs, width};

/// [`RowHeight`] reserved for the status bar at the bottom of the terminal.
///
/// [`RowHeight`]: crate::RowHeight
pub const STATUS_BAR_HEIGHT: u16 = 1;

/// Maximum number of processes supported (F1-F9).
pub const MAX_PROCESSES: usize = 9;

/// Manages display rendering and status bar for the multiplexer.
///
/// Gets the active process's buffer from [`ProcessManager`] and composites the status bar
/// into it for final rendering.
pub struct OutputRenderer {
    terminal_size: Size,
}

impl std::fmt::Debug for OutputRenderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OutputRenderer")
            .field("terminal_size", &self.terminal_size)
            .finish()
    }
}

impl OutputRenderer {
    /// Creates a new output renderer with the given terminal size.
    #[must_use]
    pub fn new(terminal_size: Size) -> Self { Self { terminal_size } }

    /// Renders the active process's buffer with the status bar composited on top.
    ///
    /// **Buffer compositing**:
    /// This method demonstrates how the virtual terminal architecture works:
    /// 1. Get the active process's complete virtual terminal ([`OffscreenBuffer`])
    /// 2. Clone it for compositing (preserves the original state)
    /// 3. Composite the status bar into the last row
    /// 4. Paint the entire composite to the real terminal all at once
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
        paint_buffer(&composite_buffer, output_device);

        ok!()
    }

    /// Composite status bar into the last row of the given [`OffscreenBuffer`].
    ///
    /// This modifies the provided buffer by writing the status bar to its last row.
    fn composite_status_bar_into_buffer(
        &mut self,
        ofs_buf: &mut OffscreenBuffer,
        process_manager: &ProcessManager,
    ) {
        // The buffer is shorter than the terminal by STATUS_BAR_HEIGHT rows, so use
        // buf_size (not self.terminal_size) when indexing into it.
        let buf_size = ofs_buf.window_size;
        let last_row_idx = buf_size.row_height.as_usize().saturating_sub(1);

        let status_style = TuiStyle {
            attribs: tui_style_attribs(Bold),
            color_fg: Some(tui_color!(lizard_green)),
            color_bg: Some(tui_color!(night_blue)),
            ..Default::default()
        };

        // Fill entire status bar row with styled spaces (background color spans full
        // width).
        for col_idx in 0..buf_size.col_width.as_usize() {
            ofs_buf[last_row_idx][col_idx] = PixelChar::PlainText {
                display_char: SPACE_CHAR,
                style: status_style,
            };
        }

        // Use print_text_with_attributes() to write styled text into the buffer.
        // This correctly handles Unicode display widths, grapheme clusters, and
        // clipping — the same code path used by the full rendering pipeline.
        let status_text = self.generate_status_text(process_manager);
        let render_local_data = RenderOpsLocalData {
            fg_color: status_style.color_fg,
            bg_color: status_style.color_bg,
            ..Default::default()
        };

        // Position cursor at the start of the status bar row.
        ofs_buf.cursor_pos = Pos::new((row(last_row_idx), col(0)));

        match print_text_with_attributes(
            &status_text,
            Some(&status_style),
            ofs_buf,
            None,
            &render_local_data,
        ) {
            Ok(new_pos) => {
                tracing::debug!(
                    "Status bar rendered OK: text_len={}, new_pos={:?}",
                    status_text.len(),
                    new_pos
                );
            }
            Err(e) => {
                tracing::error!(
                    "Status bar render FAILED: {:?}, row={}, buf_rows={}, text='{}'",
                    e,
                    last_row_idx,
                    buf_size.row_height.as_usize(),
                    status_text
                );
            }
        }
    }

    /// Generate the complete status bar text with process tabs and shortcuts.
    fn generate_status_text(&self, process_manager: &ProcessManager) -> String {
        let mut status_parts = Vec::new();

        // Show process tabs with live status indicators: 1:[🟢claude] 2:[🔴btop] etc.
        let mut current_width = width(0);

        for (i, process) in process_manager.processes().iter().enumerate() {
            let is_active = i == process_manager.active_index();
            let status_indicator = if process.is_running() { "🟢" } else { "🔴" };

            let tab_text = if is_active {
                format!(" [{}:{}{}] ", i + 1, status_indicator, process.name)
            } else {
                format!(" {}:{}{} ", i + 1, status_indicator, process.name)
            };

            // Use display width (not char count) to account for wide chars like emoji.
            let tab_width = GCStringOwned::from(tab_text.as_str())
                .display_width()
                .as_usize();
            let new_width = current_width + tab_width;
            if new_width > self.terminal_size.col_width {
                break;
            }

            status_parts.push(tab_text);
            current_width += tab_width;
        }

        // Show dynamic keyboard shortcuts based on process count.
        let process_count = process_manager.processes().len();
        let shortcuts = Self::generate_shortcuts_text(process_count);

        let shortcuts_width = GCStringOwned::from(shortcuts.as_str()).display_width();
        let total_width = current_width + shortcuts_width;
        if total_width > self.terminal_size.col_width {
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

    /// Renders initial status bar on startup using [`OffscreenBuffer`] composition.
    ///
    /// # Errors
    ///
    /// Returns an error if terminal output operations fail.
    pub fn render_initial_status_bar(
        &mut self,
        output_device: &OutputDevice,
        process_manager: &ProcessManager,
    ) -> miette::Result<()> {
        self.render_from_active_buffer(output_device, process_manager)
    }

    /// Updates the terminal size used for status bar compositing.
    pub fn update_terminal_size(&mut self, new_size: Size) {
        self.terminal_size = new_size;
    }
}

/// Paint the given [`OffscreenBuffer`] to terminal using existing paint
/// infrastructure.
fn paint_buffer(ofs_buf: &OffscreenBuffer, output_device: &OutputDevice) {
    let mut crossterm_impl = OffscreenBufferPaintImpl {};
    let render_ops = crossterm_impl.render(ofs_buf);
    crossterm_impl.paint(
        render_ops,
        FlushKind::JustFlush,
        ofs_buf.window_size, // Don't use self.terminal_size (may be different).
        lock_output_device_as_mut!(output_device),
        false, // is_mock = false
    );
}
