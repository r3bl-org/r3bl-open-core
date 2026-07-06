// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{ProcessManager, ViewportRowMapping};
use crate::{ArrayBoundsCheck, ArrayOverflowResult, CursorVisibilityMode, FlushKind,
            GCStringOwned, IndexOps, OfsBuf, OutputDevice, PixelChar,
            ProcessStatus, RangeExt, RenderOpsLocalData, SPACE_CHAR, Size, TuiStyle,
            col,
            core::coordinates::{idx, len},
            ok, print_text_with_attributes, row,
            tui::{DEBUG_TUI_PTY_MUX,
                  terminal_lib_backends::{OfsBufPaint,
                                          OfsBufPaintImpl}},
            tui_color,
            tui_style_attrib::{self, Bold},
            tui_style_attribs, width};
use std::fmt::Debug;

/// Dynamic display management for the [`PTY`] multiplexer.
///
/// - Manages rendering output from the active process's buffer from [`ProcessManager`] by
///   using [`OfsBuf`] as a compositor.
/// - Maintains a dynamic status bar showing process information and keyboard shortcuts.
/// - Handles scrollback buffer, see [`render_from_active_buffer()`] for details.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`render_from_active_buffer()`]: Self::render_from_active_buffer()
#[derive(Debug)]
pub struct OutputRenderer {
    terminal_size: Size,
}

impl OutputRenderer {
    /// Creates a new output renderer with the given terminal size.
    #[must_use]
    pub fn new(terminal_size: Size) -> Self { Self { terminal_size } }

    /// Renders the active process's terminal state, handles its scrollback history, and
    /// composites the status bar.
    ///
    /// This method safely overlays the multiplexer's chrome / UI (like the status bar)
    /// onto the underlying process without modifying the process's actual terminal state.
    /// It uses a double-buffering approach to eliminate visual artifacts:
    ///
    /// 1. Get the active process's current scrollback state and terminal size.
    /// 2. Create a new, blank composite buffer ([`OfsBuf`]).
    /// 3. Fill the composite buffer's rows from the process's history and active buffers.
    /// 4. Composite the virtual cursor (if currently visible).
    /// 5. Composite the status bar onto the last row.
    /// 6. Paint the entire composite buffer to the real terminal all at once.
    ///
    /// # Mental Model for Scrolling
    ///
    /// The `scrollback_amt` represents how many lines into the **past** the viewport has
    /// been shifted.
    ///
    /// - **The Present (Live Boundary)**: When `scrollback_amt = 0`, you are locked to
    ///   the absolute bottom of the terminal where new text is actively printed. This is
    ///   the experience without scrolling back or forwards.
    /// - **The Past (History)**: When scrolling back, `scrollback_amt` grows, meaning you
    ///   are looking further back into the history buffer.
    /// - **The Future (Does not exist!)**: `scrollback_amt` can never be negative. You
    ///   can scroll back (if there is history). But you can't scroll forwards past the
    ///   live boundary.
    ///
    /// For a visual diagram of how the viewport is split into history and live zones
    /// on the physical screen during scrollback, see [`ViewportRowMapping::calculate`].
    ///
    /// # Errors
    ///
    /// Returns an error if terminal output operations fail.
    ///
    /// [`ScrollbackAmount`]: super::ScrollbackAmount
    /// [`ViewportRowMapping::calculate`]: super::ViewportRowMapping::calculate
    pub fn render_from_active_buffer(
        &mut self,
        output_device: &OutputDevice,
        process_manager: &ProcessManager,
    ) -> miette::Result<()> {
        // Get the active process's buffer.
        let active_buffer = process_manager.active_buffer();

        // Create a new composite buffer sized for the full terminal height.
        let mut new_ofs_buf = OfsBuf::new_empty(self.terminal_size);

        // Dimensions.
        let (pty_max_rows, pty_max_cols) = (
            active_buffer.ofs_buf.get_window_size().row_height,
            active_buffer.ofs_buf.get_window_size().col_width,
        );

        // Scroll state.
        let scrollback_amt = match process_manager.active_process().maybe_scroll_offset {
            Some(it) => it,
            None => 0.into(),
        };

        // Render the PTY output (either from history or the active buffer) into the
        // composite buffer.
        for row_idx in 0..pty_max_rows.as_usize() {
            let mapped_viewport_idx = ViewportRowMapping::calculate(
                scrollback_amt,
                &active_buffer.scrollback_buffer,
                row_idx,
            );

            let maybe_pixel_char_line = match mapped_viewport_idx {
                ViewportRowMapping::History(history_row_idx) => {
                    active_buffer.scrollback_buffer.lines.get(history_row_idx).map(|l| l.pixel_chars.as_slice())
                }
                ViewportRowMapping::Live(active_buffer_row_idx) => {
                    active_buffer.ofs_buf.get_row(active_buffer_row_idx)
                }
            };

            let Some(line) = maybe_pixel_char_line else {
                // This is mathematically guaranteed to be Some(...) under normal
                // operation. However, during a terminal resize event, the window size may
                // update before the underlying buffers are physically reallocated. If we
                // hit this mid-resize race condition, simply skip drawing the
                // out-of-bounds row for this frame.
                continue;
            };

            // Copy the line of pixel chars into the offscreen buffer.
            for col_idx in 0..pty_max_cols.as_usize() {
                new_ofs_buf[row_idx][col_idx] =
                    line.get(col_idx).copied().unwrap_or_default();
            }
        }

        // Calculate the shifted row index.
        let adj_cursor_row_idx = active_buffer.get_cursor_pos().row_index + *scrollback_amt;

        // 1. Composite PTY virtual cursor if it's visible.
        // Only render the cursor if it hasn't scrolled off the bottom of the screen.
        let is_cursor_visible = adj_cursor_row_idx.as_usize() < pty_max_rows.as_usize();
        if is_cursor_visible {
            // Inherit the original cursor properties (column, shape, etc.) but with the
            // shifted row.
            let mut cursor_pos = active_buffer.get_cursor_pos();
            cursor_pos.row_index = adj_cursor_row_idx;
            new_ofs_buf.set_cursor_pos(cursor_pos);

            // Composite the cursor into the buffer.
            Self::composite_virtual_cursor_into_buffer(
                &mut new_ofs_buf,
                active_buffer.parser_global_state.cursor_visibility,
            );
        }

        // 2. Composite status bar into the last row.
        self.composite_status_bar_into_buffer(&mut new_ofs_buf, process_manager);

        // Paint the composite buffer to terminal.
        paint_buffer(&new_ofs_buf, output_device);

        ok!()
    }

    /// Composites a virtual block cursor into the buffer.
    ///
    /// This framework handles [display widths] and [segmentation] prior to populating the
    /// [`OfsBuf`], allowing us to flip the [`Reverse`] attribute on the existing
    /// [`PixelChar`]. This inverts the colors without corrupting wide characters or
    /// disrupting alignment.
    ///
    /// [`PixelChar`]: crate::PixelChar
    /// [`Reverse`]: crate::tui_style_attrib::Reverse
    /// [display widths]: unicode-width
    /// [segmentation]: crate::graphemes
    pub fn composite_virtual_cursor_into_buffer(
        ofs_buf: &mut OfsBuf,
        cursor_visibility: CursorVisibilityMode,
    ) {
        // Only do something if the child process requested a visible cursor.
        if cursor_visibility == CursorVisibilityMode::Hidden {
            return;
        }

        // Locate the requested cursor position in the offscreen buffer.
        let row_idx = ofs_buf.get_cursor_pos().row_index;
        let col_idx = ofs_buf.get_cursor_pos().col_index;

        // Bounds check.
        let buf_size = ofs_buf.get_window_size();
        if row_idx.overflows(buf_size.row_height) == ArrayOverflowResult::Overflowed
            || col_idx.overflows(buf_size.col_width) == ArrayOverflowResult::Overflowed
        {
            return;
        }

        // Grab the pixel char at that position.
        let row_usize = row_idx.as_usize();
        let mut col_usize = col_idx.as_usize();
        let original_col = col_usize;

        // If the cursor lands on a Void, it's inside the trailing columns of a wide
        // grapheme cluster (like a jumbo emoji). We scan backwards to find the origin
        // character and invert that instead, highlighting the entire wide cluster.
        while let PixelChar::Void = ofs_buf[row_usize][col_usize] {
            if col_usize == 0 {
                break;
            }
            col_usize -= 1;
        }

        // Generate a structured trace log if the cursor was snapped backwards
        if original_col != col_usize {
            DEBUG_TUI_PTY_MUX.then(|| {
                // % is Display, ? is Debug.
                tracing::info! {
                    message = "OutputRenderer::composite_virtual_cursor_into_buffer",
                    status = "Cursor landed on Void, snapped back to grapheme origin",
                    original_col = ?original_col,
                    snapped_col = ?col_usize,
                };
            });
        }

        let mut pixel_char = ofs_buf[row_usize][col_usize];

        match &mut pixel_char {
            PixelChar::PlainText { style, .. } => {
                style.attribs.reverse = Some(tui_style_attrib::Reverse);
            }
            PixelChar::Spacer => {
                let mut style = TuiStyle::default();
                style.attribs.reverse = Some(crate::tui_style_attrib::Reverse);
                pixel_char = PixelChar::PlainText {
                    display_char: SPACE_CHAR,
                    style,
                };
            }
            PixelChar::Void => {
                // Fallback: If we hit a malformed buffer (e.g. Void at column 0), do
                // nothing.
                DEBUG_TUI_PTY_MUX.then(|| {
                    // % is Display, ? is Debug.
                    tracing::info! {
                        message = "OutputRenderer::composite_virtual_cursor_into_buffer",
                        status = "Cursor landed on malformed Void (column 0), ignoring",
                        row = ?row_usize,
                        col = ?col_usize,
                    };
                });
            }
        }

        ofs_buf[row_usize][col_usize] = pixel_char;
    }

    /// Composite status bar into the last row of the given [`OfsBuf`].
    ///
    /// This modifies the provided buffer by writing the status bar to its last row.
    ///
    /// The `ofs_buf` parameter is expected to be the full terminal height, so we draw the
    /// status bar on the very last row, without clobbering any pre-existing content from
    /// other processes.
    fn composite_status_bar_into_buffer(
        &mut self,
        ofs_buf: &mut OfsBuf,
        process_manager: &ProcessManager,
    ) {
        let buf_size = ofs_buf.get_window_size();
        let last_row_idx = buf_size.row_height.as_usize().saturating_sub(1);

        let status_style = TuiStyle {
            attribs: tui_style_attribs(Bold),
            color_fg: Some(tui_color!(lizard_green)),
            color_bg: Some(tui_color!(night_blue)),
            ..Default::default()
        };

        // Fill entire status bar row with styled spaces (background color spans full
        // width).
        let col_range = (..buf_size.col_width).as_usize_range();
        ofs_buf[last_row_idx][col_range].fill(PixelChar::PlainText {
            display_char: SPACE_CHAR,
            style: status_style,
        });

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
        ofs_buf.set_cursor_pos(row(last_row_idx) + col(0));

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

        // Show process tabs with live status indicators: 1:[🟢hx] 2:[🔴btop] etc.
        let mut current_width = width(0);

        for (i, process) in process_manager.processes().iter().enumerate() {
            let is_active = i == process_manager.active_index();
            let status_indicator = if process.status() == ProcessStatus::Running {
                "🟢"
            } else {
                "🔴"
            };

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

    /// Renders initial status bar on startup using [`OfsBuf`] composition.
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

/// Paint the given [`OfsBuf`] to terminal using existing paint infrastructure.
///
/// # Note on Side Effects
///
/// We explicitly push [`hide_cursor`] here instead of passing the parsed
/// visibility state. This permanently suppresses the terminal emulator cursor when the
/// multiplexer is active, preventing flickering and cursor parking issues.
///
/// There is no danger of this messing up the chrome UI since it doesn't natively require
/// a terminal emulator cursor. If interactive regions (like a find feature) are added to
/// the chrome in the future, they will be handled by compositing another virtual caret.
///
/// [`hide_cursor`]: crate::TerminalModeController::hide_cursor
fn paint_buffer(ofs_buf: &OfsBuf, output_device: &OutputDevice) {
    let mut ofs_buf_paint_impl = OfsBufPaintImpl {};
    let render_ops = ofs_buf_paint_impl.render(ofs_buf);
    output_device.write(|out| {
        ofs_buf_paint_impl.paint(
            render_ops,
            FlushKind::JustFlush,
            ofs_buf.get_window_size(),
            out,
        );
    });
}
