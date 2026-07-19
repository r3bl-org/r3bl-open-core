// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words unshifted

use super::ProcessManager;
use crate::{ArrayBoundsCheck, ArrayOverflowResult, CursorVisibilityMode, Flat2DArray,
            FlushKind, GCStringOwned, LengthOps, NarrowingCastToU16, OfsBuf,
            OfsBufVT100, OutputDevice, PixelChar, ProcessStatus, RangeExt,
            RenderOpsLocalData, SPACE_CHAR, ScrollbackAmount, TuiStyle, VPCol, VPPos,
            VPRow, VPSize, Viewport, ViewportToCanvasExt, ok,
            print_text_with_attributes,
            tui::{DEBUG_TUI_PTY_MUX,
                  terminal_lib_backends::{paint_ofs_buf, render_ofs_buf}},
            tui_color,
            tui_style_attrib::{self, Bold},
            tui_style_attribs, vp_col, vp_pos, vp_row, vp_width};
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
    terminal_size: VPSize,
}

impl OutputRenderer {
    /// Creates a new output renderer with the given terminal size.
    #[must_use]
    pub fn new(terminal_size: VPSize) -> Self { Self { terminal_size } }

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
    /// # Errors
    ///
    /// Returns an error if terminal output operations fail.
    ///
    /// [`ScrollbackAmount`]: super::ScrollbackAmount
    pub fn render_from_active_buffer(
        &mut self,
        output_device: &OutputDevice,
        process_manager: &ProcessManager,
    ) -> miette::Result<()> {
        // Get the focused process's buffer.
        let focused_process_buffer = process_manager.focused_process_buffer();

        // Create a new composite buffer sized for the full terminal height.
        let mut new_ofs_buf = OfsBuf::new(Flat2DArray::new_empty(
            self.terminal_size,
            PixelChar::Spacer,
        ));

        // Scroll state: defaults to 0 (live bottom view).
        let scrollback_amt = process_manager
            .focused_process()
            .maybe_scroll_offset
            .unwrap_or_default();

        // Composite PTY output (either from history or the active buffer).
        render_from_active_buffer_helpers::composite_pty_output(
            &mut new_ofs_buf,
            focused_process_buffer,
            scrollback_amt,
        );

        // Composite PTY virtual cursor if it's visible.
        render_from_active_buffer_helpers::composite_virtual_cursor(
            &mut new_ofs_buf,
            focused_process_buffer,
            scrollback_amt,
        );

        // Composite status bar into the last row.
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
    /// [`PixelChar`]: crate::tui::PixelChar
    /// [`Reverse`]: crate::tui_style_attrib::Reverse
    /// [display widths]: unicode-width
    /// [segmentation]: crate::graphemes
    pub fn composite_virtual_cursor_into_buffer(
        ofs_buf: &mut OfsBuf,
        cursor_visibility: CursorVisibilityMode,
    ) {
        render_from_active_buffer_helpers::composite_virtual_cursor_into_buffer(
            ofs_buf,
            cursor_visibility,
        );
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
        let last_row = buf_size.row_height.convert_to_index();

        let status_style = TuiStyle {
            attribs: tui_style_attribs(Bold),
            color_fg: Some(tui_color!(lizard_green)),
            color_bg: Some(tui_color!(night_blue)),
            ..Default::default()
        };

        // Fill entire status bar row with styled spaces (background color spans full
        // width).
        let col_range = ..buf_size.col_width;
        let col_range = col_range.as_usize_range();
        let spacer = PixelChar::PlainText {
            display_char: SPACE_CHAR,
            style: status_style,
        };
        let status_row_slice = &mut ofs_buf[last_row.as_usize()][col_range];
        status_row_slice.fill(spacer);

        // Use print_text_with_attributes() to write styled text into the buffer. This
        // correctly handles Unicode display widths, grapheme clusters, and clipping. The
        // same code path used by the full rendering pipeline.
        let status_text = self.generate_status_text(process_manager);
        let render_local_data = RenderOpsLocalData {
            fg_color: status_style.color_fg,
            bg_color: status_style.color_bg,
            ..Default::default()
        };

        // Position cursor at the start of the status bar row.
        ofs_buf.set_cursor_pos(vp_pos(vp_col(0), last_row));

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
                    last_row.as_usize(),
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
        let mut current_width = vp_width(0);

        for (i, process) in process_manager.processes().iter().enumerate() {
            let is_focused = i == process_manager.focused_index();
            let status_indicator = if process.status() == ProcessStatus::Running {
                "🟢"
            } else {
                "🔴"
            };

            let tab_text = if is_focused {
                format!(" [{}:{}{}] ", i + 1, status_indicator, process.name)
            } else {
                format!(" {}:{}{} ", i + 1, status_indicator, process.name)
            };

            // Use display width (not char count) to account for wide chars like emoji.
            let tab_width = GCStringOwned::from(tab_text.as_str()).display_width();
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
            // For 5+ processes, show range notation (up to F9).
            format!("  F1-F{}: Switch | Ctrl+Q: Quit", process_count.min(9))
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
    pub fn update_terminal_size(&mut self, new_size: VPSize) {
        self.terminal_size = new_size;
    }
}

/// Paint the given [`OfsBuf`] to terminal using existing paint infrastructure.
///
/// # Note on Side Effects
///
/// We explicitly push [`hide_cursor`] here instead of passing the parsed visibility
/// state. This permanently suppresses the terminal emulator cursor when the multiplexer
/// is active, preventing flickering and cursor parking issues.
///
/// There is no danger of this messing up the chrome UI since it doesn't natively require
/// a terminal emulator cursor. If interactive regions (like a find feature) are added to
/// the chrome in the future, they will be handled by compositing another virtual caret.
///
/// [`hide_cursor`]: TerminalModeController::hide_cursor
fn paint_buffer(ofs_buf: &OfsBuf, output_device: &OutputDevice) {
    let render_ops = render_ofs_buf(ofs_buf);
    output_device.write(|out| {
        paint_ofs_buf(
            render_ops,
            FlushKind::JustFlush,
            ofs_buf.get_window_size(),
            out,
        );
    });
}

pub mod render_from_active_buffer_helpers {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Copies rows from the focused process's active screen buffer (or scrollback
    /// history) into the composite [`OfsBuf`], translating Viewport columns into Canvas
    /// columns.
    pub fn composite_pty_output(
        ofs_buf: &mut OfsBuf,
        focused_process_buffer: &OfsBufVT100,
        scrollback_amt: ScrollbackAmount,
    ) {
        let active_screen_buffer = focused_process_buffer.get_active_screen_buffer();
        let active_buffer_vp = active_screen_buffer.get_viewport();
        let physical_size = ofs_buf.get_window_size();

        for vp_row in (..physical_size.row_height).as_index_iter() {
            let maybe_pixel_char_line =
                active_screen_buffer.get_row_with_scrollback(vp_row, scrollback_amt);

            let Some(line) = maybe_pixel_char_line else {
                // This is mathematically guaranteed to be Some(...) under normal
                // operation. However, during a terminal resize event, the window size may
                // update before the underlying buffers are physically reallocated. If we
                // hit this mid-resize race condition, simply skip drawing the
                // out-of-bounds row for this frame.
                continue;
            };

            let dest_row = &mut ofs_buf[vp_row.as_usize()];

            // Copy the line of pixel chars into the destination row by translating
            // Viewport col to Canvas col.
            for vp_col in (..physical_size.col_width).as_index_iter() {
                let c_col = active_buffer_vp.to_canvas(vp_col);
                let pixel_char = line.get(c_col.as_usize()).copied().unwrap_or_default();
                dest_row[vp_col.as_usize()] = pixel_char;
            }
        }
    }

    /// Composites the [`PTY`] virtual cursor into the destination [`OfsBuf`] if visible
    /// and within screen bounds.
    ///
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    pub fn composite_virtual_cursor(
        ofs_buf: &mut OfsBuf,
        focused_process_buffer: &OfsBufVT100,
        scrollback_amt: ScrollbackAmount,
    ) {
        let active_screen_buffer = focused_process_buffer.get_active_screen_buffer();
        let virtual_terminal_vp_cursor_pos = active_screen_buffer.get_cursor_vp_pos();

        // Construct the physical screen viewport looking into the virtual canvas.
        //
        // We combine:
        // 1. The virtual terminal active screen buffer's horizontal pan offset
        //    (origin_pos, e.g. col: 500).
        // 2. The physical screen dimensions (ofs_buf window size, e.g. 80x24).
        //
        // Using ofs_buf's viewport alone lacks the pan offset (always 0,0), while using
        // the virtual terminal's viewport alone lacks physical boundary clipping (width
        // is 1000).
        let multiplexer_viewport = Viewport::new(
            active_screen_buffer.get_viewport().get_origin_pos(),
            ofs_buf.get_window_size(),
        );

        if let Some(physical_screen_vp_cursor_pos) =
            cursor_projection_helpers::calculate_screen_cursor_pos(
                virtual_terminal_vp_cursor_pos,
                multiplexer_viewport,
                scrollback_amt,
            )
        {
            ofs_buf.set_cursor_vp_pos(physical_screen_vp_cursor_pos);

            composite_virtual_cursor_into_buffer(
                ofs_buf,
                focused_process_buffer
                    .get_parser_global_state()
                    .cursor_visibility,
            );
        }
    }

    /// Composites a virtual block cursor into the buffer.
    ///
    /// This framework handles [display widths] and [segmentation] prior to populating the
    /// [`OfsBuf`], allowing us to flip the [`Reverse`] attribute on the existing
    /// [`PixelChar`]. This inverts the colors without corrupting wide characters or
    /// disrupting alignment.
    ///
    /// [`PixelChar`]: crate::tui::PixelChar
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
        let cursor_pos: VPPos = ofs_buf.get_cursor_pos();
        let row_idx = cursor_pos.row_index;
        let col_idx = cursor_pos.col_index;

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

        // Generate a structured trace log if the cursor was snapped backwards.
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
                style.attribs.reverse = Some(tui_style_attrib::Reverse);
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
}

pub mod cursor_projection_helpers {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Translates the child process's `virtual_terminal_vp_cursor_pos` into the
    /// `physical_screen_vp_pos` visible on the terminal display, accounting for both
    /// multiplexer horizontal panning and vertical scrollback.
    ///
    /// Returns `Some(`[`VPPos`]`)` in **Physical Screen Viewport Coordinates** if visible
    /// on screen, or `None` if panned or scrolled outside the visible viewport window.
    ///
    /// ```text
    ///   (a) Virtual Terminal Frame (Child Process)       (b) Physical Screen Frame (User Display)
    ///  ┌──────────────────────────────────────────┐     ┌────────────────────────────────────┐
    ///  │ (0,0)                                    │     │ (0,0)                              │
    ///  │   virtual_terminal_vp_cursor_pos         │ ──► │   physical_screen_vp_cursor_pos    │
    ///  │   (col: 15, row: 2)                      │     │   (col: 5, row: 7)                 │
    ///  └──────────────────────────────────────────┘     └────────────────────────────────────┘
    ///   origin_col = 10 (panned right by 10)             scrollback_amt = 5 (scrolled up by 5)
    ///   col = 15 - 10 = 5                                row = 2 + 5 = 7
    /// ```
    #[must_use]
    pub fn calculate_screen_cursor_pos(
        virtual_terminal_vp_cursor_pos: VPPos,
        multiplexer_viewport: Viewport,
        scrollback_amt: ScrollbackAmount,
    ) -> Option<VPPos> {
        let visible_col = calculate_panned_cursor_col(
            virtual_terminal_vp_cursor_pos.col_index,
            multiplexer_viewport,
        )?;
        let visible_row = calculate_scrolled_cursor_row(
            virtual_terminal_vp_cursor_pos.row_index,
            multiplexer_viewport,
            scrollback_amt,
        )?;
        Some(vp_pos(visible_col, visible_row))
    }

    /// Horizontal Panning Projection:
    ///
    /// Translates `virtual_terminal_vp_col` (column index in the child process's
    /// virtual terminal) into `physical_screen_vp_col` (column index on the physical
    /// terminal screen) by subtracting the multiplexer's horizontal pan offset
    /// (`origin_col`).
    ///
    /// ```text
    ///   Virtual Terminal Columns (Child Process Buffer):
    ///  ┌──────────┬───────────────────────────────┬───────────────────────────┐
    ///  │ 0 ... 9  │ 10 ... 15 ... 89              │ 90 ...                    │
    ///  └──────────┴───────────────────────────────┴───────────────────────────┘
    ///   (Off-Left) ▲               ▲               ▲ (Off-Right)
    ///              │               │               │
    ///              origin_col = 10 │ cursor = 15   origin_col + width = 90
    ///                              ▼
    ///                     Physical Screen Col = 15 - 10 = 5
    ///              ┌───────────────────────────────┐
    ///              │ 0 ... 5 ... 79                │ Viewport Width = 80
    ///              └───────────────────────────────┘
    /// ```
    ///
    /// Returns `Some(`[`VPCol`]`)` if within horizontal screen bounds `[0, width)`, or
    /// `None` if panned off-screen left or right.
    #[must_use]
    pub fn calculate_panned_cursor_col(
        virtual_terminal_vp_col: VPCol,
        multiplexer_viewport: Viewport,
    ) -> Option<VPCol> {
        let origin_col = multiplexer_viewport.get_origin_pos().col_index.as_usize();
        let physical_col_usize =
            virtual_terminal_vp_col.as_usize().checked_sub(origin_col)?;
        let physical_screen_vp_col = vp_col(physical_col_usize);

        if physical_screen_vp_col.overflows(multiplexer_viewport.get_width())
            == ArrayOverflowResult::Within
        {
            Some(physical_screen_vp_col)
        } else {
            None
        }
    }

    /// Vertical Scrollback Projection:
    ///
    /// Translates `virtual_terminal_vp_row` (row index in the child process's live screen
    /// buffer) into `physical_screen_vp_row` (row index on the physical terminal screen)
    /// by adding the scrollback amount (since scrolling up into history pushes the live
    /// buffer downward on the display).
    ///
    /// ```text
    ///   Physical Screen Display (User Viewport Window):
    ///   ┌─────────────────────────────────────────┐
    ///  0│ History Line 0                          │ ▲
    ///  1│ History Line 1                          │ │ scrollback_amt = 5
    ///  2│ History Line 2                          │ │ (live buffer pushed down by 5 rows)
    ///  3│ History Line 3                          │ │
    ///  4│ History Line 4                          │ ▼
    ///   ├─────────────────────────────────────────┤
    ///  5│ Live Buffer Row 0                       │
    ///  6│ Live Buffer Row 1                       │
    ///  7│ Live Buffer Row 2 ◄── Cursor (row 7)    │ Physical Screen Row = 2 + 5 = 7
    ///   └─────────────────────────────────────────┘
    /// ```
    ///
    /// Returns `Some(`[`VPRow`]`)` if within vertical screen bounds `[0, height)`, or
    /// `None` if scrolled off-screen bottom into the future.
    #[must_use]
    pub fn calculate_scrolled_cursor_row(
        virtual_terminal_vp_row: VPRow,
        multiplexer_viewport: Viewport,
        scrollback_amt: ScrollbackAmount,
    ) -> Option<VPRow> {
        let physical_screen_vp_row = vp_row(
            virtual_terminal_vp_row
                .as_u16()
                .saturating_add(scrollback_amt.as_u16_narrowing()),
        );

        if physical_screen_vp_row.overflows(multiplexer_viewport.get_height())
            == ArrayOverflowResult::Within
        {
            Some(physical_screen_vp_row)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Process, vp_height, vp_row, vp_size};

    #[test]
    fn test_composite_virtual_cursor_ascii_plain_text() {
        let size = vp_size(vp_width(80), vp_height(25));
        let mut ofs_buf = OfsBuf::new(Flat2DArray::new_empty(size, PixelChar::Spacer));

        // Write "Hello" on row 0.
        let text = "Hello";
        for (i, ch) in text.chars().enumerate() {
            ofs_buf[0][i] = PixelChar::PlainText {
                display_char: ch,
                style: TuiStyle::default(),
            };
        }

        // Set cursor to (1, 0) (over 'e').
        ofs_buf.set_cursor_vp_pos(vp_pos(vp_col(1), vp_row(0)));

        // Composite visible cursor.
        OutputRenderer::composite_virtual_cursor_into_buffer(
            &mut ofs_buf,
            CursorVisibilityMode::Visible,
        );

        // Cell (1, 0) should now have the Reverse style attribute.
        match ofs_buf[0][1] {
            PixelChar::PlainText {
                style,
                display_char,
            } => {
                assert_eq!(display_char, 'e');
                assert!(style.attribs.reverse.is_some());
            }
            _ => panic!("Expected PlainText at (1, 0)"),
        }

        // Cell (0, 0) should remain un-reversed.
        match ofs_buf[0][0] {
            PixelChar::PlainText { style, .. } => {
                assert!(style.attribs.reverse.is_none());
            }
            _ => panic!("Expected PlainText at (0, 0)"),
        }
    }

    #[test]
    fn test_composite_virtual_cursor_hidden() {
        let size = vp_size(vp_width(80), vp_height(25));
        let mut ofs_buf = OfsBuf::new(Flat2DArray::new_empty(size, PixelChar::Spacer));

        ofs_buf[0][0] = PixelChar::PlainText {
            display_char: 'A',
            style: TuiStyle::default(),
        };
        ofs_buf.set_cursor_vp_pos(vp_pos(vp_col(0), vp_row(0)));

        OutputRenderer::composite_virtual_cursor_into_buffer(
            &mut ofs_buf,
            CursorVisibilityMode::Hidden,
        );

        match ofs_buf[0][0] {
            PixelChar::PlainText { style, .. } => {
                assert!(style.attribs.reverse.is_none());
            }
            _ => panic!("Expected PlainText at (0, 0)"),
        }
    }

    #[test]
    fn test_composite_virtual_cursor_spacer() {
        let size = vp_size(vp_width(80), vp_height(25));
        let mut ofs_buf = OfsBuf::new(Flat2DArray::new_empty(size, PixelChar::Spacer));

        ofs_buf.set_cursor_vp_pos(vp_pos(vp_col(5), vp_row(5)));

        OutputRenderer::composite_virtual_cursor_into_buffer(
            &mut ofs_buf,
            CursorVisibilityMode::Visible,
        );

        match ofs_buf[5][5] {
            PixelChar::PlainText {
                style,
                display_char,
            } => {
                assert_eq!(display_char, SPACE_CHAR);
                assert!(style.attribs.reverse.is_some());
            }
            _ => panic!("Expected PlainText at (5, 5)"),
        }
    }

    #[test]
    fn test_composite_virtual_cursor_void_snap_back() {
        let size = vp_size(vp_width(80), vp_height(25));
        let mut ofs_buf = OfsBuf::new(Flat2DArray::new_empty(size, PixelChar::Spacer));

        // Place wide character at col 2, and Void at col 3.
        ofs_buf[0][2] = PixelChar::PlainText {
            display_char: '🦀',
            style: TuiStyle::default(),
        };
        ofs_buf[0][3] = PixelChar::Void;

        // Position cursor on the trailing Void column (col 3).
        ofs_buf.set_cursor_vp_pos(vp_pos(vp_col(3), vp_row(0)));

        OutputRenderer::composite_virtual_cursor_into_buffer(
            &mut ofs_buf,
            CursorVisibilityMode::Visible,
        );

        // Col 2 (origin) should have Reverse set.
        match ofs_buf[0][2] {
            PixelChar::PlainText {
                style,
                display_char,
            } => {
                assert_eq!(display_char, '🦀');
                assert!(style.attribs.reverse.is_some());
            }
            _ => panic!("Expected PlainText at (0, 2)"),
        }

        // Col 3 should remain Void.
        assert_eq!(ofs_buf[0][3], PixelChar::Void);
    }

    #[test]
    fn test_composite_virtual_cursor_out_of_bounds() {
        let size = vp_size(vp_width(80), vp_height(25));
        let mut ofs_buf = OfsBuf::new(Flat2DArray::new_empty(size, PixelChar::Spacer));

        // Position cursor outside buffer bounds.
        ofs_buf.set_cursor_vp_pos(vp_pos(vp_col(100), vp_row(100)));

        // Should not panic.
        OutputRenderer::composite_virtual_cursor_into_buffer(
            &mut ofs_buf,
            CursorVisibilityMode::Visible,
        );
    }

    #[test]
    fn test_status_bar_generation_and_composite() {
        let terminal_size = vp_size(vp_width(80), vp_height(25));
        let proc1 = Process::new("bash", "bash", vec![], terminal_size);
        let proc2 = Process::new("htop", "htop", vec![], terminal_size);
        let process_manager = ProcessManager::new(vec![proc1, proc2], terminal_size);

        let mut renderer = OutputRenderer::new(terminal_size);
        let status_text = renderer.generate_status_text(&process_manager);

        // Status text should contain indicators and process names.
        assert!(status_text.contains("1:"));
        assert!(status_text.contains("bash"));
        assert!(status_text.contains("2:"));
        assert!(status_text.contains("htop"));

        let mut ofs_buf =
            OfsBuf::new(Flat2DArray::new_empty(terminal_size, PixelChar::Spacer));
        renderer.composite_status_bar_into_buffer(&mut ofs_buf, &process_manager);

        // Last row (row 24) should have been populated with status text characters.
        let last_row = &ofs_buf[24];
        let has_content = last_row.iter().any(|cell| match cell {
            PixelChar::PlainText { display_char, .. } => *display_char != SPACE_CHAR,
            _ => false,
        });
        assert!(has_content);
    }

    #[test]
    fn test_composite_virtual_cursor_horizontal_pan_translation() {
        let terminal_size = vp_size(vp_width(80), vp_height(25));
        let mut proc = Process::new("bash", "bash", vec![], terminal_size);

        // Position the cursor at canvas column 15, row 2.
        proc.terminal_state
            .get_active_screen_buffer_mut()
            .set_cursor_vp_pos(vp_pos(vp_col(15), vp_row(2)));

        // Pan the viewport horizontally to column 10 (so canvas col 15 -> viewport col
        // 5).
        proc.pan_right_by(vp_width(10));

        let mut ofs_buf =
            OfsBuf::new(Flat2DArray::new_empty(terminal_size, PixelChar::Spacer));

        render_from_active_buffer_helpers::composite_virtual_cursor(
            &mut ofs_buf,
            &proc.terminal_state,
            ScrollbackAmount::default(),
        );

        // Virtual cursor should be composited at viewport col 5 (15 - 10 = 5).
        match ofs_buf[2][5] {
            PixelChar::PlainText { style, .. } => {
                assert!(style.attribs.reverse.is_some());
            }
            _ => panic!("Expected virtual cursor at viewport col 5"),
        }

        // Viewport col 15 should NOT have reverse attribute.
        match ofs_buf[2][15] {
            PixelChar::Spacer => {}
            _ => panic!("Expected un-inverted spacer at viewport col 15"),
        }
    }

    #[test]
    fn test_composite_virtual_cursor_panned_offscreen() {
        let terminal_size = vp_size(vp_width(80), vp_height(25));
        let mut proc = Process::new("bash", "bash", vec![], terminal_size);

        // Position cursor at canvas column 5.
        proc.terminal_state
            .get_active_screen_buffer_mut()
            .set_cursor_vp_pos(vp_pos(vp_col(5), vp_row(2)));

        // Pan viewport right to column 10 (canvas col 5 is now off-screen to the left).
        proc.pan_right_by(vp_width(10));

        let mut ofs_buf =
            OfsBuf::new(Flat2DArray::new_empty(terminal_size, PixelChar::Spacer));

        render_from_active_buffer_helpers::composite_virtual_cursor(
            &mut ofs_buf,
            &proc.terminal_state,
            ScrollbackAmount::default(),
        );

        // No cell should have reverse attribute because cursor is off-screen.
        for cell in &ofs_buf[2] {
            assert!(matches!(cell, PixelChar::Spacer));
        }
    }

    #[test]
    fn test_cursor_projection_helpers_panning_and_scrolling() {
        use crate::{c_pos, scrollback_amount};

        let viewport = Viewport::new(c_pos(10, 0), vp_size(vp_width(80), vp_height(24)));

        // Test calculate_panned_cursor_col:
        // Col 15 in canvas with origin 10 -> col 5
        assert_eq!(
            cursor_projection_helpers::calculate_panned_cursor_col(vp_col(15), viewport),
            Some(vp_col(5))
        );
        // Col 5 in canvas with origin 10 -> None (off-screen left)
        assert_eq!(
            cursor_projection_helpers::calculate_panned_cursor_col(vp_col(5), viewport),
            None
        );
        // Col 90 in canvas with origin 10 -> None (80 >= width 80, off-screen right)
        assert_eq!(
            cursor_projection_helpers::calculate_panned_cursor_col(vp_col(90), viewport),
            None
        );

        // Test calculate_scrolled_cursor_row:
        // Row 2 with scrollback 5 -> row 7
        assert_eq!(
            cursor_projection_helpers::calculate_scrolled_cursor_row(
                vp_row(2),
                viewport,
                scrollback_amount(5)
            ),
            Some(vp_row(7))
        );
        // Row 20 with scrollback 5 -> None (25 >= height 24, off-screen bottom)
        assert_eq!(
            cursor_projection_helpers::calculate_scrolled_cursor_row(
                vp_row(20),
                viewport,
                scrollback_amount(5)
            ),
            None
        );

        // Test calculate_screen_cursor_pos:
        assert_eq!(
            cursor_projection_helpers::calculate_screen_cursor_pos(
                vp_pos(vp_col(15), vp_row(2)),
                viewport,
                scrollback_amount(5)
            ),
            Some(vp_pos(vp_col(5), vp_row(7)))
        );
    }

    #[test]
    fn test_composite_virtual_cursor_combined_panning_and_scrollback() {
        use crate::scrollback_amount;

        let terminal_size = vp_size(vp_width(80), vp_height(25));
        let mut proc = Process::new("bash", "bash", vec![], terminal_size);

        // Position the cursor at canvas column 15, row 2.
        proc.terminal_state
            .get_active_screen_buffer_mut()
            .set_cursor_vp_pos(vp_pos(vp_col(15), vp_row(2)));

        // Pan the viewport horizontally right by 10 columns (origin_col = 10).
        proc.pan_right_by(vp_width(10));

        let mut ofs_buf =
            OfsBuf::new(Flat2DArray::new_empty(terminal_size, PixelChar::Spacer));

        // Composite with scrollback amount of 3.
        render_from_active_buffer_helpers::composite_virtual_cursor(
            &mut ofs_buf,
            &proc.terminal_state,
            scrollback_amount(3),
        );

        // Cursor should appear at viewport col 5 (15 - 10) and row 5 (2 + 3).
        match ofs_buf[5][5] {
            PixelChar::PlainText { style, .. } => {
                assert!(style.attribs.reverse.is_some());
            }
            _ => panic!("Expected virtual cursor at viewport pos (5, 5)"),
        }

        // Original unshifted position (col 15, row 2) should NOT have reverse attribute.
        match ofs_buf[2][15] {
            PixelChar::Spacer => {}
            _ => panic!("Expected un-inverted spacer at (15, 2)"),
        }
    }

    #[test]
    fn test_composite_virtual_cursor_scrolled_offscreen_bottom() {
        use crate::scrollback_amount;

        let terminal_size = vp_size(vp_width(80), vp_height(25));
        let mut proc = Process::new("bash", "bash", vec![], terminal_size);

        // Position cursor at row 22.
        proc.terminal_state
            .get_active_screen_buffer_mut()
            .set_cursor_vp_pos(vp_pos(vp_col(5), vp_row(22)));

        let mut ofs_buf =
            OfsBuf::new(Flat2DArray::new_empty(terminal_size, PixelChar::Spacer));

        // Scrollback of 5 pushes row 22 + 5 = 27 >= 25 (off-screen bottom).
        render_from_active_buffer_helpers::composite_virtual_cursor(
            &mut ofs_buf,
            &proc.terminal_state,
            scrollback_amount(5),
        );

        // No cell in the buffer should have the reverse attribute.
        let height = ofs_buf.get_window_size().row_height.as_usize();
        for r in 0..height {
            for cell in &ofs_buf[r] {
                assert!(matches!(cell, PixelChar::Spacer));
            }
        }
    }

    #[test]
    fn test_composite_virtual_cursor_panned_offscreen_right() {
        let terminal_size = vp_size(vp_width(80), vp_height(25));
        let mut proc = Process::new("bash", "bash", vec![], terminal_size);

        // Position cursor at column 90 (which exceeds width 80 when origin_col = 0).
        proc.terminal_state
            .get_active_screen_buffer_mut()
            .set_cursor_vp_pos(vp_pos(vp_col(90), vp_row(2)));

        let mut ofs_buf =
            OfsBuf::new(Flat2DArray::new_empty(terminal_size, PixelChar::Spacer));

        render_from_active_buffer_helpers::composite_virtual_cursor(
            &mut ofs_buf,
            &proc.terminal_state,
            ScrollbackAmount::default(),
        );

        // No cell should have reverse attribute because cursor is off-screen right.
        for cell in &ofs_buf[2] {
            assert!(matches!(cell, PixelChar::Spacer));
        }
    }

    #[test]
    fn test_composite_pty_output_horizontal_panning() {
        let terminal_size = vp_size(vp_width(80), vp_height(25));
        let mut proc = Process::new("bash", "bash", vec![], terminal_size);

        // Place 'Z' at canvas col 10, row 0 in the process's screen buffer.
        assert!(
            proc.terminal_state
                .get_active_screen_buffer_mut()
                .set_char(
                    vp_pos(vp_col(10), vp_row(0)),
                    PixelChar::PlainText {
                        display_char: 'Z',
                        style: TuiStyle::default(),
                    },
                )
                .is_ok()
        );

        // Pan viewport right by 10 (so canvas col 10 maps to viewport col 0).
        proc.pan_right_by(vp_width(10));

        let mut ofs_buf =
            OfsBuf::new(Flat2DArray::new_empty(terminal_size, PixelChar::Spacer));

        render_from_active_buffer_helpers::composite_pty_output(
            &mut ofs_buf,
            &proc.terminal_state,
            ScrollbackAmount::default(),
        );

        // Viewport col 0 on row 0 should contain 'Z'.
        match ofs_buf[0][0] {
            PixelChar::PlainText { display_char, .. } => {
                assert_eq!(display_char, 'Z');
            }
            _ => panic!("Expected 'Z' at viewport column 0"),
        }
    }

    #[test]
    fn test_composite_wide_virtual_terminal_output_and_cursor() {
        let physical_size = vp_size(vp_width(80), vp_height(25));
        let mut proc = Process::new_with_virtual_width(
            "bash",
            "bash",
            vec![],
            physical_size,
            Some(vp_width(1000)),
        );

        // Place 'W' at column 500 in the wide virtual terminal.
        assert!(
            proc.terminal_state
                .get_active_screen_buffer_mut()
                .set_char(
                    vp_pos(vp_col(500), vp_row(0)),
                    PixelChar::PlainText {
                        display_char: 'W',
                        style: TuiStyle::default(),
                    },
                )
                .is_ok()
        );

        // Set cursor at (col: 505, row: 0).
        proc.terminal_state
            .get_active_screen_buffer_mut()
            .set_cursor_vp_pos(vp_pos(vp_col(505), vp_row(0)));

        // Pan right by 500 columns (origin_col = 500).
        proc.pan_right_by(vp_width(500));

        let mut ofs_buf =
            OfsBuf::new(Flat2DArray::new_empty(physical_size, PixelChar::Spacer));

        // Composite PTY output.
        render_from_active_buffer_helpers::composite_pty_output(
            &mut ofs_buf,
            &proc.terminal_state,
            ScrollbackAmount::default(),
        );

        // 'W' from canvas column 500 should appear at physical viewport column 0.
        match ofs_buf[0][0] {
            PixelChar::PlainText { display_char, .. } => {
                assert_eq!(display_char, 'W');
            }
            _ => panic!("Expected 'W' at physical column 0"),
        }

        // Composite virtual cursor.
        render_from_active_buffer_helpers::composite_virtual_cursor(
            &mut ofs_buf,
            &proc.terminal_state,
            ScrollbackAmount::default(),
        );

        // Cursor should appear at physical column 5 (505 - 500).
        match ofs_buf[0][5] {
            PixelChar::PlainText { style, .. } => {
                assert!(style.attribs.reverse.is_some());
            }
            _ => panic!("Expected virtual cursor at physical column 5"),
        }
    }
}
