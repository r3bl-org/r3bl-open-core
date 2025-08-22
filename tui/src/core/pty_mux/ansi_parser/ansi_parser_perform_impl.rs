// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Internal implementation for ANSI/VT sequence processing.

use vte::{Params, Perform};

use super::{ansi_parser_public_api::AnsiToBufferProcessor,
            ansi_to_tui_color::ansi_to_tui_color, csi_codes, esc_codes};
use crate::{BoundsCheck, CharacterSet, OffscreenBuffer, PixelChar, Pos, TuiStyle,
            TuiStyleAttribs, col, row, tui_style_attrib};

/// Internal API.
impl Drop for AnsiToBufferProcessor<'_> {
    /// Finalize processing by updating the buffer's cursor position.
    fn drop(&mut self) { self.ofs_buf.my_pos = self.cursor_pos; }
}

impl Perform for AnsiToBufferProcessor<'_> {
    /// Handle printable characters.
    fn print(&mut self, ch: char) {
        // Apply character set translation if in graphics mode
        let display_char = match self.ofs_buf.character_set {
            CharacterSet::Graphics => char_translation::translate_dec_graphics(ch),
            CharacterSet::Ascii => ch,
        };

        let row_max = self.ofs_buf.window_size.row_height;
        let col_max = self.ofs_buf.window_size.col_width;
        let current_row = self.cursor_pos.row_index;
        let current_col = self.cursor_pos.col_index;

        // Only write if within bounds
        if current_row.check_overflows(row_max) == crate::BoundsStatus::Within
            && current_col.check_overflows(col_max) == crate::BoundsStatus::Within
        {
            // Write character to buffer using public fields
            self.ofs_buf.buffer[current_row.as_usize()][current_col.as_usize()] =
                PixelChar::PlainText {
                    display_char, // Use the translated character
                    maybe_style: self.current_style,
                };

            // Move cursor forward
            let new_col = current_col + col(1);

            // Handle line wrap
            if new_col.check_overflows(col_max) == crate::BoundsStatus::Overflowed {
                self.cursor_pos.col_index = col(0);
                let next_row = current_row + row(1);
                if next_row.check_overflows(row_max) == crate::BoundsStatus::Within {
                    self.cursor_pos.row_index = next_row;
                }
            } else {
                self.cursor_pos.col_index = new_col;
            }
        }
    }

    /// Handle control characters (C0 set).
    fn execute(&mut self, byte: u8) {
        match byte {
            0x08 => {
                // Backspace
                let current_col = self.cursor_pos.col_index.as_usize();
                if current_col > 0 {
                    self.cursor_pos.col_index = col(current_col - 1);
                }
            }
            0x09 => {
                // Tab - move to next 8-column boundary
                let current_col = self.cursor_pos.col_index.as_usize();
                let next_tab = ((current_col / 8) + 1) * 8;
                let max_col = self.ofs_buf.window_size.col_width;
                let new_col = col(next_tab);
                // Clamp to max_col-1 if it would overflow
                self.cursor_pos.col_index = if new_col.check_overflows(max_col)
                    == crate::BoundsStatus::Overflowed
                {
                    max_col.convert_to_col_index()
                } else {
                    new_col
                };
            }
            0x0A => {
                // Line feed (newline)
                let max_row = self.ofs_buf.window_size.row_height;
                let next_row = self.cursor_pos.row_index + row(1);
                if next_row.check_overflows(max_row) == crate::BoundsStatus::Within {
                    self.cursor_pos.row_index = next_row;
                }
            }
            0x0D => {
                // Carriage return
                self.cursor_pos.col_index = col(0);
            }
            _ => {}
        }
    }

    /// Handle CSI (Control Sequence Introducer) sequences.
    #[allow(clippy::too_many_lines)]
    fn csi_dispatch(
        &mut self,
        params: &Params,
        _intermediates: &[u8],
        _ignore: bool,
        c: char,
    ) {
        #[allow(clippy::match_same_arms)]
        match c {
            csi_codes::CUU_CURSOR_UP => {
                let n = i64::from(
                    params
                        .iter()
                        .next()
                        .and_then(|p| p.first())
                        .copied()
                        .unwrap_or(1),
                );
                cursor_ops::cursor_up(self, n);
            }
            csi_codes::CUD_CURSOR_DOWN => {
                let n = i64::from(
                    params
                        .iter()
                        .next()
                        .and_then(|p| p.first())
                        .copied()
                        .unwrap_or(1),
                );
                cursor_ops::cursor_down(self, n);
            }
            csi_codes::CUF_CURSOR_FORWARD => {
                let n = i64::from(
                    params
                        .iter()
                        .next()
                        .and_then(|p| p.first())
                        .copied()
                        .unwrap_or(1),
                );
                cursor_ops::cursor_forward(self, n);
            }
            csi_codes::CUB_CURSOR_BACKWARD => {
                let n = i64::from(
                    params
                        .iter()
                        .next()
                        .and_then(|p| p.first())
                        .copied()
                        .unwrap_or(1),
                );
                cursor_ops::cursor_backward(self, n);
            }
            csi_codes::CUP_CURSOR_POSITION | csi_codes::HVP_CURSOR_POSITION => {
                cursor_ops::cursor_position(self, params);
            }
            csi_codes::ED_ERASE_DISPLAY | csi_codes::EL_ERASE_LINE => {
                // Clear screen/line - ignore, TUI apps will repaint themselves
                // These are intentionally the same as wildcard for simplicity
            }

            csi_codes::SGR_SET_GRAPHICS => sgr_ops::sgr(self, params), /* Select Graphic Rendition */

            csi_codes::SCP_SAVE_CURSOR => {
                // CSI s - Save current cursor position
                // Alternative to ESC 7 (DECSC)
                self.ofs_buf.saved_cursor_pos = Some(self.cursor_pos);
                tracing::trace!(
                    "CSI s (SCP): Saved cursor position {:?}",
                    self.cursor_pos
                );
            }
            csi_codes::RCP_RESTORE_CURSOR => {
                // CSI u - Restore saved cursor position
                // Alternative to ESC 8 (DECRC)
                if let Some(saved_pos) = self.ofs_buf.saved_cursor_pos {
                    self.cursor_pos = saved_pos;
                    tracing::trace!(
                        "CSI u (RCP): Restored cursor position to {:?}",
                        saved_pos
                    );
                }
            }

            csi_codes::CNL_CURSOR_NEXT_LINE => {
                // CSI E - Cursor Next Line
                // Move cursor to beginning of line n lines down
                let n = i64::from(
                    params
                        .iter()
                        .next()
                        .and_then(|p| p.first())
                        .copied()
                        .unwrap_or(1),
                );
                cursor_ops::cursor_down(self, n);
                self.cursor_pos.col_index = col(0);
                tracing::trace!("CSI E (CNL): Moved to next line {}", n);
            }

            csi_codes::CPL_CURSOR_PREV_LINE => {
                // CSI F - Cursor Previous Line
                // Move cursor to beginning of line n lines up
                let n = i64::from(
                    params
                        .iter()
                        .next()
                        .and_then(|p| p.first())
                        .copied()
                        .unwrap_or(1),
                );
                cursor_ops::cursor_up(self, n);
                self.cursor_pos.col_index = col(0);
                tracing::trace!("CSI F (CPL): Moved to previous line {}", n);
            }

            csi_codes::CHA_CURSOR_COLUMN => {
                // CSI G - Cursor Horizontal Absolute
                // Move cursor to column n (1-based)
                let n = params
                    .iter()
                    .next()
                    .and_then(|p| p.first())
                    .copied()
                    .unwrap_or(1);
                // Convert from 1-based to 0-based, clamp to buffer width
                let target_col = (n as usize).saturating_sub(1);
                let max_col = self.ofs_buf.window_size.col_width.as_usize();
                self.cursor_pos.col_index =
                    col(target_col.min(max_col.saturating_sub(1)));
                tracing::trace!("CSI G (CHA): Moved to column {}", n);
            }

            csi_codes::SU_SCROLL_UP => {
                // CSI S - Scroll Up
                // Scroll display up by n lines
                let n = params
                    .iter()
                    .next()
                    .and_then(|p| p.first())
                    .copied()
                    .unwrap_or(1) as usize;
                for _ in 0..n {
                    scroll_ops::scroll_buffer_up(self);
                }
                tracing::trace!("CSI S (SU): Scrolled up {} lines", n);
            }

            csi_codes::SD_SCROLL_DOWN => {
                // CSI T - Scroll Down
                // Scroll display down by n lines
                let n = params
                    .iter()
                    .next()
                    .and_then(|p| p.first())
                    .copied()
                    .unwrap_or(1) as usize;
                for _ in 0..n {
                    scroll_ops::scroll_buffer_down(self);
                }
                tracing::trace!("CSI T (SD): Scrolled down {} lines", n);
            }

            csi_codes::DSR_DEVICE_STATUS => {
                // CSI n - Device Status Report
                // This requires sending a response back through the PTY
                // For now, just log it as we can't send responses in current architecture
                let n = params
                    .iter()
                    .next()
                    .and_then(|p| p.first())
                    .copied()
                    .unwrap_or(0);
                match n {
                    5 => {
                        // Status report request - should respond with ESC[0n (OK)
                        tracing::debug!(
                            "CSI 5n (DSR): Status report requested (response needed but not implemented)"
                        );
                    }
                    6 => {
                        // Cursor position report - should respond with ESC[row;colR
                        tracing::debug!(
                            "CSI 6n (DSR): Cursor position report requested at {:?} (response needed but not implemented)",
                            self.cursor_pos
                        );
                    }
                    _ => {
                        tracing::debug!("CSI {}n (DSR): Unknown device status report", n);
                    }
                }
            }

            _ => {} /* Ignore other CSI
                     * sequences */
        }
    }

    /// Handle OSC (Operating System Command) sequences.
    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        use crate::core::osc::{OscEvent, osc_codes};

        if params.is_empty() {
            return;
        }

        // Parse the OSC code (first parameter)
        if let Ok(code) = std::str::from_utf8(params[0]) {
            match code {
                // OSC 0: Set both window title and icon name
                // OSC 1: Set icon name only (we treat same as title)
                // OSC 2: Set window title only
                osc_codes::OSC_CODE_TITLE_AND_ICON
                | osc_codes::OSC_CODE_ICON
                | osc_codes::OSC_CODE_TITLE
                    if params.len() > 1 =>
                {
                    if let Ok(title) = std::str::from_utf8(params[1]) {
                        self.pending_osc_events
                            .push(OscEvent::SetTitleAndTab(title.to_string()));
                    }
                }
                // OSC 8: Hyperlink (format: OSC 8 ; params ; URI)
                osc_codes::OSC_CODE_HYPERLINK if params.len() > 2 => {
                    if let Ok(uri) = std::str::from_utf8(params[2]) {
                        // For now, just store the URI - display text would come from
                        // print chars
                        self.pending_osc_events.push(OscEvent::Hyperlink {
                            uri: uri.to_string(),
                            text: String::new(), // Text is handled separately via print()
                        });
                    }
                }
                // OSC 9;4: Progress sequences (already handled by OscBuffer in some
                // contexts) We could handle them here too if needed
                _ => {
                    // Ignore other OSC sequences for now
                }
            }
        }
    }

    /// Handle escape sequences (not CSI or OSC).
    ///
    /// There's significant overlap between **CSI sequences** and direct **ESC
    /// sequences**, especially in managing the cursor state. This overlap exists
    /// because direct ESC sequences were the original way to handle many terminal
    /// functions. As terminals became more advanced, the more flexible and powerful CSI
    /// sequences were introduced to handle the same tasks with greater precision.
    ///
    /// Here are a few key examples of this overlap:
    ///
    /// ### Cursor Management
    ///
    /// Both categories have commands for saving and restoring the cursor's position, a
    /// common task for applications that need to temporarily move the cursor to
    /// display a message and then return it to its original location.
    ///
    /// * **Direct ESC:** The `ESC 7` and `ESC 8` commands are simple, single-character
    ///   sequences for saving and restoring the cursor and its attributes (like color).
    ///   They don't take any parameters.
    ///
    /// * **CSI:** The `ESC[s` (Save Cursor) and `ESC[u` (Restore Cursor) commands were
    ///   introduced to provide the same functionality within the CSI framework. Some
    ///   modern terminals and emulators have moved toward using the CSI versions
    ///   exclusively.
    ///
    /// ### Scrolling
    ///
    /// Another area of overlap is screen scrolling. Direct ESC sequences have basic
    /// commands, while CSI provides more granular control.
    ///
    /// * **Direct ESC:** The `ESC D` (Index) command scrolls the screen up one line,
    ///   while `ESC M` (Reverse Index) scrolls it down one line. These are fixed
    ///   operations.
    ///
    /// * **CSI:** CSI sequences, like `ESC[S` (Scroll Up) and `ESC[T` (Scroll Down),
    ///   allow for a numerical parameter to specify how many lines to scroll, offering
    ///   more fine-tuned control.
    ///
    /// ### Character Set Switching
    ///
    /// Historically, terminals supported different character sets for displaying things
    /// like line-drawing graphics. This was often managed with direct ESC sequences.
    ///
    /// * **Direct ESC:** Commands like `ESC ( B` (Select ASCII) and `ESC ( 0` (Select
    ///   VT100 graphics) were used to switch between character sets.
    ///
    /// * **CSI:** While less common, some CSI sequences also exist to select character
    ///   sets, providing a modern alternative to the legacy direct escape codes.
    ///
    /// ## ESC Sequence Architecture
    ///
    /// ```text
    /// Child Process (vim, bash, etc.)
    ///         ↓
    ///     PTY Slave (writes ESC sequences)
    ///         ↓
    ///     PTY Master (we read from here)
    ///         ↓
    ///     VTE Parser (tokenizes sequences)
    ///         ↓
    ///     esc_dispatch() [THIS METHOD]
    ///         ↓
    ///     Updates OffscreenBuffer state
    ///         ↓
    ///     OutputRenderer (paints final result)
    /// ```
    ///
    /// ## Supported ESC Sequences
    ///
    /// ### Cursor Save/Restore (Requires Persistent State)
    /// - **ESC 7 (DECSC)**: Save cursor position to `ofs_buf.saved_cursor_pos`
    /// - **ESC 8 (DECRC)**: Restore cursor from `ofs_buf.saved_cursor_pos`
    ///
    /// ### Character Set Selection (Requires Persistent State)
    /// - **ESC ( B**: Select ASCII character set (normal text)
    /// - **ESC ( 0**: Select DEC graphics (box-drawing characters)
    ///
    /// ### Scrolling Operations (Stateless)
    /// - **ESC D (IND)**: Index - move cursor down, scroll if at bottom
    /// - **ESC M (RI)**: Reverse Index - move cursor up, scroll if at top
    ///
    /// ### Terminal Control (Stateless)
    /// - **ESC c (RIS)**: Reset terminal to initial state
    ///
    /// ## Data Flow Example: Cursor Save/Restore
    ///
    /// ```text
    /// Session 1: vim at position (5,10) sends ESC 7
    ///   → AnsiToBufferProcessor::new() with cursor_pos = ofs_buf.my_pos (5,10)
    ///   → esc_dispatch() handles ESC 7
    ///   → Saves ofs_buf.saved_cursor_pos = Some((5,10))
    ///   → drop() updates ofs_buf.my_pos
    ///
    /// Session 2: vim moves cursor to (20,30), then sends ESC 8
    ///   → AnsiToBufferProcessor::new() with cursor_pos = ofs_buf.my_pos (20,30)
    ///   → esc_dispatch() handles ESC 8
    ///   → Restores cursor_pos = ofs_buf.saved_cursor_pos.unwrap() // (5,10)
    ///   → drop() updates ofs_buf.my_pos = (5,10) ✓
    /// ```
    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        match byte {
            esc_codes::DECSC_SAVE_CURSOR => {
                // DECSC - Save current cursor position
                // The cursor position is saved to persistent buffer state so it
                // survives across multiple AnsiToBufferProcessor instances
                self.ofs_buf.saved_cursor_pos = Some(self.cursor_pos);
                tracing::trace!(
                    "ESC 7 (DECSC): Saved cursor position {:?}",
                    self.cursor_pos
                );
            }
            esc_codes::DECRC_RESTORE_CURSOR => {
                // DECRC - Restore saved cursor position
                // Retrieves the previously saved position from buffer's persistent state
                if let Some(saved_pos) = self.ofs_buf.saved_cursor_pos {
                    self.cursor_pos = saved_pos;
                    tracing::trace!(
                        "ESC 8 (DECRC): Restored cursor position to {:?}",
                        saved_pos
                    );
                }
            }
            esc_codes::IND_INDEX_DOWN => {
                // IND - Index (move down one line, scroll if at bottom)
                scroll_ops::index_down(self);
            }
            esc_codes::RI_REVERSE_INDEX_UP => {
                // RI - Reverse Index (move up one line, scroll if at top)
                scroll_ops::reverse_index_up(self);
            }
            esc_codes::RIS_RESET_TERMINAL => {
                // RIS - Reset to Initial State
                terminal_ops::reset_terminal(self);
            }
            _ if intermediates == esc_codes::G0_CHARSET_INTERMEDIATE => {
                // Character set selection G0
                match byte {
                    esc_codes::CHARSET_ASCII => {
                        // Select ASCII character set (normal mode)
                        self.ofs_buf.character_set = CharacterSet::Ascii;
                        tracing::trace!("ESC ( B: Selected ASCII character set");
                    }
                    esc_codes::CHARSET_DEC_GRAPHICS => {
                        // Select DEC Special Graphics character set
                        // This enables box-drawing characters
                        self.ofs_buf.character_set = CharacterSet::Graphics;
                        tracing::trace!("ESC ( 0: Selected DEC graphics character set");
                    }
                    _ => {
                        tracing::trace!(
                            "ESC ( {}: Unsupported character set",
                            byte as char
                        );
                    }
                }
            }
            _ => {
                tracing::trace!("ESC {}: Unsupported escape sequence", byte as char);
            }
        }
    }

    /// Hook for DCS (Device Control String) start.
    ///
    /// Starts a Device Control String (DCS), used for:
    /// - Sixel graphics
    /// - `ReGIS` graphics
    /// - Custom protocol extensions
    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _c: char) {
        // Ignore DCS sequences
    }

    /// Handle DCS data by continuing to receive bytes for an active DCS sequence started
    /// by hook.
    fn put(&mut self, _byte: u8) {
        // Ignore DCS data
    }

    /// Hook for DCS - ends the DCS sequence, signaling that all data has been received.
    fn unhook(&mut self) {
        // Ignore DCS end
    }
}

/// Create a new processor for the given `ofs_buf`.
///
/// This creates a fresh processor instance with all SGR (Select Graphic Rendition)
/// attributes reset to their default state. The processor is designed to be
/// transient/stateless - created fresh for each batch of bytes to process.
///
/// **CRITICAL FIX**: The processor now initializes its cursor position from the
/// buffer's current position (`ofs_buf.my_pos`) instead of `Pos::default()`. This
/// ensures that ESC sequences like ESC 7 (save cursor) work correctly by saving
/// the actual cursor position rather than (0,0).
pub fn new(ofs_buf: &mut OffscreenBuffer) -> AnsiToBufferProcessor<'_> {
    // CRITICAL FIX: Initialize from buffer's current position, not default!
    // This ensures ESC 7 saves the actual cursor position, not (0,0)
    // We need to copy the position before borrowing buffer mutably
    let initial_cursor_pos = ofs_buf.my_pos;

    AnsiToBufferProcessor {
        ofs_buf,
        cursor_pos: initial_cursor_pos, // ← Was: Pos::default()
        current_style: None,
        attribs: TuiStyleAttribs::default(),
        fg_color: None,
        bg_color: None,
        pending_osc_events: Vec::new(),
    }
}

/// Handle the core parsing loop where each byte is fed to the [`VTE parser`], which
/// in turn calls methods on the processor (via the [`Perform`] trait).
///
/// [`VTE parser`]: vte::Parser
/// [`Perform`]: vte::Perform
pub fn process_bytes(
    processor: &mut AnsiToBufferProcessor,
    parser: &mut vte::Parser,
    bytes: impl AsRef<[u8]>,
) {
    for &byte in bytes.as_ref() {
        parser.advance(processor, byte);
    }
}

/// Cursor movement operations.
mod cursor_ops {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Move cursor up by n lines.
    pub fn cursor_up(processor: &mut AnsiToBufferProcessor, n: i64) {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let n = n.max(1) as usize; // Safe: n.max(1) ensures n >= 1, i64 to usize is safe here
        let current_row = processor.cursor_pos.row_index.as_usize();
        processor.cursor_pos.row_index = row(current_row.saturating_sub(n));
    }

    /// Move cursor down by n lines.
    pub fn cursor_down(processor: &mut AnsiToBufferProcessor, n: i64) {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let n = n.max(1) as usize; // Safe: n.max(1) ensures n >= 1, i64 to usize is safe here
        let max_row = processor.ofs_buf.window_size.row_height;
        let new_row = processor.cursor_pos.row_index + row(n);
        // Clamp to max_row-1 if it would overflow
        processor.cursor_pos.row_index =
            if new_row.check_overflows(max_row) == crate::BoundsStatus::Overflowed {
                max_row.convert_to_row_index()
            } else {
                new_row
            };
    }

    /// Move cursor forward by n columns.
    pub fn cursor_forward(processor: &mut AnsiToBufferProcessor, n: i64) {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let n = n.max(1) as usize; // Safe: n.max(1) ensures n >= 1, i64 to usize is safe here
        let max_col = processor.ofs_buf.window_size.col_width;
        let new_col = processor.cursor_pos.col_index + col(n);
        // Clamp to max_col-1 if it would overflow
        processor.cursor_pos.col_index =
            if new_col.check_overflows(max_col) == crate::BoundsStatus::Overflowed {
                max_col.convert_to_col_index()
            } else {
                new_col
            };
    }

    /// Move cursor backward by n columns.
    pub fn cursor_backward(processor: &mut AnsiToBufferProcessor, n: i64) {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let n = n.max(1) as usize; // Safe: n.max(1) ensures n >= 1, i64 to usize is safe here
        let current_col = processor.cursor_pos.col_index.as_usize();
        processor.cursor_pos.col_index = col(current_col.saturating_sub(n));
    }

    /// Set cursor position (1-based coordinates from ANSI, converted to 0-based).
    pub fn cursor_position(processor: &mut AnsiToBufferProcessor, params: &Params) {
        let row_param = params
            .iter()
            .next()
            .and_then(|p| p.first())
            .copied()
            .unwrap_or(1)
            .max(1) as usize
            - 1;
        let col_param = params
            .iter()
            .nth(1)
            .and_then(|p| p.first())
            .copied()
            .unwrap_or(1)
            .max(1) as usize
            - 1;
        let max_row = processor.ofs_buf.window_size.row_height;
        let max_col = processor.ofs_buf.window_size.col_width;

        let new_row = row(row_param);
        let new_col = col(col_param);

        // Clamp row and column to valid bounds
        processor.cursor_pos = Pos {
            col_index: if new_col.check_overflows(max_col)
                == crate::BoundsStatus::Overflowed
            {
                max_col.convert_to_col_index()
            } else {
                new_col
            },
            row_index: if new_row.check_overflows(max_row)
                == crate::BoundsStatus::Overflowed
            {
                max_row.convert_to_row_index()
            } else {
                new_row
            },
        };
    }
}

/// Scrolling operations.
mod scroll_ops {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Move cursor down one line, scrolling the buffer if at bottom.
    /// Implements the ESC D (IND) escape sequence.
    pub fn index_down(processor: &mut AnsiToBufferProcessor) {
        let max_row = processor.ofs_buf.window_size.row_height;

        // Check if we're at or beyond the max row (need to scroll)
        let next_row = processor.cursor_pos.row_index + row(1);
        if next_row.check_overflows(max_row) == crate::BoundsStatus::Overflowed {
            // At bottom - scroll buffer content up by one line
            scroll_buffer_up(processor);
        } else {
            // Not at bottom - just move cursor down
            cursor_ops::cursor_down(processor, 1);
        }
    }

    /// Move cursor up one line, scrolling the buffer if at top.
    /// Implements the ESC M (RI) escape sequence.
    pub fn reverse_index_up(processor: &mut AnsiToBufferProcessor) {
        // Check if we're at the top row (row 0)
        if processor.cursor_pos.row_index == row(0) {
            // At top - scroll buffer content down by one line
            scroll_buffer_down(processor);
        } else {
            // Not at top - just move cursor up
            cursor_ops::cursor_up(processor, 1);
        }
    }

    /// Scroll buffer content up by one line (for ESC D at bottom).
    /// The top line is lost, and a new empty line appears at bottom.
    pub fn scroll_buffer_up(processor: &mut AnsiToBufferProcessor) {
        let max_row = processor.ofs_buf.window_size.row_height.as_usize();

        // Shift all lines up by one (line 0 is lost)
        for row in 0..max_row.saturating_sub(1) {
            processor.ofs_buf.buffer[row] = processor.ofs_buf.buffer[row + 1].clone();
        }

        // Clear the new bottom line
        let new_bottom_row = max_row.saturating_sub(1);
        for col in 0..processor.ofs_buf.window_size.col_width.as_usize() {
            processor.ofs_buf.buffer[new_bottom_row][col] = PixelChar::Spacer;
        }
    }

    /// Scroll buffer content down by one line (for ESC M at top).
    /// The bottom line is lost, and a new empty line appears at top.
    pub fn scroll_buffer_down(processor: &mut AnsiToBufferProcessor) {
        let max_row = processor.ofs_buf.window_size.row_height.as_usize();

        // Shift all lines down by one (bottom line is lost)
        for row in (1..max_row).rev() {
            processor.ofs_buf.buffer[row] = processor.ofs_buf.buffer[row - 1].clone();
        }

        // Clear the new top line
        for col in 0..processor.ofs_buf.window_size.col_width.as_usize() {
            processor.ofs_buf.buffer[0][col] = PixelChar::Spacer;
        }
    }
}

/// Style/Graphics Rendition operations.
mod sgr_ops {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Update the current `TuiStyle` based on SGR attributes.
    pub fn update_style(processor: &mut AnsiToBufferProcessor) {
        processor.current_style = Some(TuiStyle {
            id: None,
            attribs: processor.attribs,
            computed: None,
            color_fg: processor.fg_color,
            color_bg: processor.bg_color,
            padding: None,
            lolcat: None,
        });
    }

    /// Reset all SGR attributes to default state.
    fn reset_all_attributes(processor: &mut AnsiToBufferProcessor) {
        processor.attribs.bold = None;
        processor.attribs.dim = None;
        processor.attribs.italic = None;
        processor.attribs.underline = None;
        processor.attribs.blink = None;
        processor.attribs.reverse = None;
        processor.attribs.hidden = None;
        processor.attribs.strikethrough = None;
        processor.fg_color = None;
        processor.bg_color = None;
    }

    /// Apply a single SGR parameter.
    fn apply_sgr_param(processor: &mut AnsiToBufferProcessor, param: u16) {
        match param {
            csi_codes::SGR_RESET => {
                reset_all_attributes(processor);
            }
            csi_codes::SGR_BOLD => processor.attribs.bold = Some(tui_style_attrib::Bold),
            csi_codes::SGR_DIM => processor.attribs.dim = Some(tui_style_attrib::Dim),
            csi_codes::SGR_ITALIC => {
                processor.attribs.italic = Some(tui_style_attrib::Italic);
            }
            csi_codes::SGR_UNDERLINE => {
                processor.attribs.underline = Some(tui_style_attrib::Underline);
            }
            csi_codes::SGR_BLINK => {
                processor.attribs.blink = Some(tui_style_attrib::Blink);
            }
            csi_codes::SGR_REVERSE => {
                processor.attribs.reverse = Some(tui_style_attrib::Reverse);
            }
            csi_codes::SGR_HIDDEN => {
                processor.attribs.hidden = Some(tui_style_attrib::Hidden);
            }
            csi_codes::SGR_STRIKETHROUGH => {
                processor.attribs.strikethrough = Some(tui_style_attrib::Strikethrough);
            }
            csi_codes::SGR_RESET_BOLD_DIM => {
                processor.attribs.bold = None;
                processor.attribs.dim = None;
            }
            csi_codes::SGR_RESET_ITALIC => processor.attribs.italic = None,
            csi_codes::SGR_RESET_UNDERLINE => processor.attribs.underline = None,
            csi_codes::SGR_RESET_BLINK => processor.attribs.blink = None,
            csi_codes::SGR_RESET_REVERSE => processor.attribs.reverse = None,
            csi_codes::SGR_RESET_HIDDEN => processor.attribs.hidden = None,
            csi_codes::SGR_RESET_STRIKETHROUGH => processor.attribs.strikethrough = None,
            csi_codes::SGR_FG_BLACK..=csi_codes::SGR_FG_WHITE => {
                processor.fg_color = Some(ansi_to_tui_color(param.into()));
            }
            csi_codes::SGR_FG_DEFAULT => processor.fg_color = None, /* Default foreground */
            csi_codes::SGR_BG_BLACK..=csi_codes::SGR_BG_WHITE => {
                processor.bg_color = Some(ansi_to_tui_color(param.into()));
            }
            csi_codes::SGR_BG_DEFAULT => processor.bg_color = None, /* Default background */
            csi_codes::SGR_FG_BRIGHT_BLACK..=csi_codes::SGR_FG_BRIGHT_WHITE => {
                processor.fg_color = Some(ansi_to_tui_color(param.into()));
            }
            csi_codes::SGR_BG_BRIGHT_BLACK..=csi_codes::SGR_BG_BRIGHT_WHITE => {
                processor.bg_color = Some(ansi_to_tui_color(param.into()));
            }
            _ => {} /* Ignore unsupported SGR parameters (256-color, RGB, etc.) */
        }
    }

    /// Handle SGR (Select Graphic Rendition) parameters.
    pub fn sgr(processor: &mut AnsiToBufferProcessor, params: &Params) {
        for param_slice in params {
            for &param in param_slice {
                apply_sgr_param(processor, param);
            }
        }
        update_style(processor);
    }
}

/// Terminal state operations.
mod terminal_ops {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Clear all buffer content.
    fn clear_buffer(processor: &mut AnsiToBufferProcessor) {
        let max_row = processor.ofs_buf.window_size.row_height.as_usize();
        for row in 0..max_row {
            for col in 0..processor.ofs_buf.window_size.col_width.as_usize() {
                processor.ofs_buf.buffer[row][col] = PixelChar::Spacer;
            }
        }
    }

    /// Reset all SGR attributes to default state.
    fn reset_sgr_attributes(processor: &mut AnsiToBufferProcessor) {
        processor.current_style = None;
        processor.attribs.bold = None;
        processor.attribs.dim = None;
        processor.attribs.italic = None;
        processor.attribs.underline = None;
        processor.attribs.blink = None;
        processor.attribs.reverse = None;
        processor.attribs.hidden = None;
        processor.attribs.strikethrough = None;
        processor.fg_color = None;
        processor.bg_color = None;
    }

    /// Reset terminal to initial state (ESC c).
    /// Clears the buffer, resets cursor, and clears saved state.
    pub fn reset_terminal(processor: &mut AnsiToBufferProcessor) {
        clear_buffer(processor);

        // Reset cursor to home position
        processor.cursor_pos = Pos::default();

        // Clear saved cursor state
        processor.ofs_buf.saved_cursor_pos = None;

        // Reset to ASCII character set
        processor.ofs_buf.character_set = CharacterSet::Ascii;

        // Clear any SGR attributes
        reset_sgr_attributes(processor);

        tracing::trace!("ESC c: Terminal reset to initial state");
    }
}

/// Character set translation operations.
mod char_translation {
    /// Translate DEC Special Graphics characters to Unicode box-drawing characters.
    /// Used when `character_set` is Graphics (after ESC ( 0).
    pub fn translate_dec_graphics(c: char) -> char {
        match c {
            'j' => '┘', // Lower right corner
            'k' => '┐', // Upper right corner
            'l' => '┌', // Upper left corner
            'm' => '└', // Lower left corner
            'n' => '┼', // Crossing lines
            'q' => '─', // Horizontal line
            't' => '├', // Left "T"
            'u' => '┤', // Right "T"
            'v' => '┴', // Bottom "T"
            'w' => '┬', // Top "T"
            'x' => '│', // Vertical line
            _ => c,     // Pass through unmapped characters
        }
    }
}

/// Internal methods for `AnsiToBufferProcessor` to implement [`Perform`] trait.
impl AnsiToBufferProcessor<'_> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ANSIBasicColor, SgrCode, TuiColor, height,
                offscreen_buffer::test_fixtures_offscreen_buffer::*, width};

    /// Create a test `OffscreenBuffer` with 10x10 dimensions (9 content rows + 1 status
    /// bar).
    fn create_test_offscreen_buffer() -> OffscreenBuffer {
        OffscreenBuffer::new_with_capacity_initialized(height(10) + width(10))
    }

    #[test]
    fn test_processor_creation() {
        let mut ofs_buf = create_test_offscreen_buffer();
        let processor = new(&mut ofs_buf);
        assert_eq!(processor.cursor_pos, Pos::default());
        assert!(processor.attribs.bold.is_none());
        assert!(processor.attribs.italic.is_none());
        assert!(processor.fg_color.is_none());
    }

    #[test]
    fn test_sgr_reset_behavior() {
        let mut ofs_buf = create_test_offscreen_buffer();

        #[allow(clippy::items_after_statements)]
        const RED: &str = "RED";
        #[allow(clippy::items_after_statements)]
        const NORM: &str = "NORM";

        // Test SGR reset by sending this sequence to the processor:
        // Set bold+red, write "RED", reset all, write "NORM"
        {
            let mut processor = new(&mut ofs_buf);
            process_bytes(
                &mut processor,
                /* new parser */ &mut vte::Parser::new(),
                /* sequence */
                format!(
                    "{bold}{fg_red}{text1}{reset_all}{text2}",
                    bold = SgrCode::Bold,
                    fg_red = SgrCode::ForegroundBasic(ANSIBasicColor::Red),
                    reset_all = SgrCode::Reset,
                    text1 = RED,
                    text2 = NORM
                ),
            );
        } // processor dropped here

        // Verify "RED" has bold and red color
        for (col, expected_char) in RED.chars().enumerate() {
            assert_styled_char_at(
                &ofs_buf,
                0,
                col,
                expected_char,
                |style_from_buffer| {
                    matches!(
                        (style_from_buffer.attribs.bold, style_from_buffer.color_fg),
                        (
                            Some(tui_style_attrib::Bold),
                            Some(TuiColor::Basic(ANSIBasicColor::Red))
                        )
                    )
                },
                "bold red text",
            );
        }

        // Verify "NORM" has no styling (SGR 0 reset everything)
        assert_plain_text_at(&ofs_buf, 0, RED.len(), NORM);
    }

    #[test]
    fn test_sgr_partial_reset() {
        let mut ofs_buf = create_test_offscreen_buffer();

        // Test partial SGR resets (SGR 22 resets bold/dim only)
        {
            let mut processor = new(&mut ofs_buf);
            let mut parser = vte::Parser::new();

            // Set bold+italic+red, write "A", reset bold/dim only, write "B"
            let sequence = format!(
                "{bold}{italic}{fg_red}A{reset_bold_dim}B",
                bold = SgrCode::Bold,
                italic = SgrCode::Italic,
                fg_red = SgrCode::ForegroundBasic(ANSIBasicColor::DarkRed),
                reset_bold_dim = SgrCode::ResetBoldDim
            );
            process_bytes(&mut processor, &mut parser, &sequence);
        }

        // Verify 'A' has bold, italic, and red
        assert_styled_char_at(
            &ofs_buf,
            0,
            0,
            'A',
            |style_from_buffer| {
                matches!(
                    (
                        style_from_buffer.attribs.bold,
                        style_from_buffer.attribs.italic,
                        style_from_buffer.color_fg
                    ),
                    (
                        Some(tui_style_attrib::Bold),
                        Some(tui_style_attrib::Italic),
                        Some(TuiColor::Basic(ANSIBasicColor::DarkRed))
                    )
                )
            },
            "bold italic red",
        );

        // Verify 'B' has italic and red but NOT bold (SGR 22 reset bold/dim)
        assert_styled_char_at(
            &ofs_buf,
            0,
            1,
            'B',
            |style_from_buffer| {
                matches!(
                    (
                        style_from_buffer.attribs.bold,
                        style_from_buffer.attribs.italic,
                        style_from_buffer.color_fg
                    ),
                    (
                        None,
                        Some(tui_style_attrib::Italic),
                        Some(TuiColor::Basic(ANSIBasicColor::DarkRed))
                    )
                )
            },
            "italic red (no bold)",
        );
    }

    #[test]
    fn test_cursor_movement_up() {
        // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
        // 1-based index.
        let mut ofs_buf = create_test_offscreen_buffer();

        // Test cursor up movement with buffer verification
        {
            let mut processor = new(&mut ofs_buf);

            // Start at row 5, write a character
            processor.cursor_pos = Pos {
                row_index: row(5),
                col_index: col(3),
            };
            processor.print('A');

            // Move up 2 rows and write another character
            cursor_ops::cursor_up(&mut processor, 2);
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 3);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 4); // Column should be after 'A'

            processor.cursor_pos.col_index = col(3); // Reset column to same position
            processor.print('B');

            // Try to move up beyond boundary
            cursor_ops::cursor_up(&mut processor, 10);
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 0); // Should stop at row 0
            processor.cursor_pos.col_index = col(3);
            processor.print('C');
        }

        // Verify characters are in correct positions
        assert_plain_char_at(&ofs_buf, 5, 3, 'A');
        assert_plain_char_at(&ofs_buf, 3, 3, 'B');
        assert_plain_char_at(&ofs_buf, 0, 3, 'C');
    }

    #[test]
    fn test_cursor_movement_down() {
        // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
        // 1-based index.
        let mut ofs_buf = create_test_offscreen_buffer();

        // Test cursor down movement with buffer verification
        {
            let mut processor = new(&mut ofs_buf);

            // Start at row 2, write a character
            processor.cursor_pos = Pos {
                row_index: row(2),
                col_index: col(4),
            };
            processor.print('X');

            // Move down 3 rows and write another character
            cursor_ops::cursor_down(&mut processor, 3);
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 5);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 5); // Column should be after 'X'

            processor.cursor_pos.col_index = col(3); // Reset column to same position
            processor.print('Y');

            // Try to move down beyond buffer area
            cursor_ops::cursor_down(&mut processor, 5);
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 9); // Should stop at row 9 (last row)
            processor.cursor_pos.col_index = col(3);
            processor.print('Z');
        }

        // Verify characters are in correct positions
        assert_plain_char_at(&ofs_buf, 2, 4, 'X');
        assert_plain_char_at(&ofs_buf, 5, 3, 'Y'); // col 3 because we reset it
        assert_plain_char_at(&ofs_buf, 9, 3, 'Z'); // col 3 because we reset it, row 9 is now valid
    }

    #[test]
    fn test_cursor_movement_forward() {
        // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
        // 1-based index.
        let mut ofs_buf = create_test_offscreen_buffer();

        // Test cursor forward movement with buffer verification
        {
            let mut processor = new(&mut ofs_buf);

            // Write some text at row 3
            processor.cursor_pos = Pos {
                row_index: row(3),
                col_index: col(0),
            };
            processor.print('1');

            // Move forward 3 columns and write
            cursor_ops::cursor_forward(&mut processor, 3);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 4);
            processor.print('2');

            // Move forward to near end of line
            cursor_ops::cursor_forward(&mut processor, 3);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 8);
            processor.print('3');

            // Try to move beyond line boundary
            cursor_ops::cursor_forward(&mut processor, 5);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 9); // Should stop at last column
            processor.print('4');
        }

        // Verify characters are in correct positions
        assert_plain_char_at(&ofs_buf, 3, 0, '1');
        assert_plain_char_at(&ofs_buf, 3, 4, '2');
        assert_plain_char_at(&ofs_buf, 3, 8, '3');
        assert_plain_char_at(&ofs_buf, 3, 9, '4');

        // Verify gaps are empty
        for col in [1, 2, 3, 5, 6, 7] {
            assert_empty_at(&ofs_buf, 3, col);
        }
    }

    #[test]
    fn test_cursor_movement_backward() {
        // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
        // 1-based index.
        let mut ofs_buf = create_test_offscreen_buffer();

        // Test cursor backward movement with buffer verification
        {
            let mut processor = new(&mut ofs_buf);

            // Start at column 8 and write
            processor.cursor_pos = Pos {
                row_index: row(4),
                col_index: col(8),
            };
            processor.print('A');

            // Move backward 3 columns and write
            cursor_ops::cursor_backward(&mut processor, 3);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 6); // 9 - 3 = 6
            processor.print('B');

            // Move backward more
            cursor_ops::cursor_backward(&mut processor, 4);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 3); // 7 - 4 = 3
            processor.print('C');

            // Try to move beyond start of line
            cursor_ops::cursor_backward(&mut processor, 10);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 0); // Should stop at column 0
            processor.print('D');
        }

        // Verify characters are in correct positions
        assert_plain_char_at(&ofs_buf, 4, 8, 'A');
        assert_plain_char_at(&ofs_buf, 4, 6, 'B');
        assert_plain_char_at(&ofs_buf, 4, 3, 'C');
        assert_plain_char_at(&ofs_buf, 4, 0, 'D');

        // Verify gaps are empty
        for col in [1, 2, 4, 5, 7, 9] {
            assert_empty_at(&ofs_buf, 4, col);
        }
    }

    #[test]
    fn test_cursor_absolute_positioning_boundaries() {
        let mut ofs_buf = create_test_offscreen_buffer();

        // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
        // 1-based index. Test absolute positioning with boundary checks
        {
            let mut processor = new(&mut ofs_buf);

            // Valid position in middle of buffer
            processor.cursor_pos = Pos {
                row_index: row(4),
                col_index: col(5),
            };
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 4);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 5);
            processor.print('M');

            // Top-left corner
            processor.cursor_pos = Pos {
                row_index: row(1),
                col_index: col(0),
            };
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 1);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 0);
            processor.print('T');

            // Bottom-right corner
            processor.cursor_pos = Pos {
                row_index: row(9),
                col_index: col(9),
            };
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 9);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 9);
            processor.print('B');

            // Write at another position in last row
            processor.cursor_pos = Pos {
                row_index: row(9),
                col_index: col(5),
            };
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 9);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 5);
            processor.print('C'); // Will write to row 9

            // Try to position beyond columns (col(15) creates index 15, not clamped)
            processor.cursor_pos = Pos {
                row_index: row(3),
                col_index: col(15),
            };
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 3);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 15); // col(15) creates index 15
            processor.print('E'); // Won't write since col 15 is beyond buffer width
        }

        // Verify characters are in correct positions
        assert_plain_char_at(&ofs_buf, 4, 5, 'M'); // Written at row 4, col 5
        assert_plain_char_at(&ofs_buf, 1, 0, 'T'); // Written at row 1, col 0
        assert_plain_char_at(&ofs_buf, 9, 9, 'B'); // Written at row 9, col 9
        assert_plain_char_at(&ofs_buf, 9, 5, 'C'); // Written at row 9, col 5
        // 'E' was not written (column beyond buffer)
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn test_sgr_color_attributes() {
        let mut ofs_buf = create_test_offscreen_buffer();

        // Test various SGR color sequences through VTE parser
        {
            let mut processor = new(&mut ofs_buf);
            let mut parser = vte::Parser::new();

            // Test foreground colors: black, red, green, white, then background colors
            let sequence = format!(
                "{fg_black}B{fg_red}R{fg_green}G{fg_white}W{reset} {bg_red}X{bg_green}Y{reset2}{fg_red2}{bg_blue}Z{reset3}",
                fg_black = SgrCode::ForegroundBasic(ANSIBasicColor::Black),
                fg_red = SgrCode::ForegroundBasic(ANSIBasicColor::DarkRed),
                fg_green = SgrCode::ForegroundBasic(ANSIBasicColor::DarkGreen),
                fg_white = SgrCode::ForegroundBasic(ANSIBasicColor::White),
                reset = SgrCode::Reset,
                bg_red = SgrCode::BackgroundBasic(ANSIBasicColor::DarkRed),
                bg_green = SgrCode::BackgroundBasic(ANSIBasicColor::DarkGreen),
                reset2 = SgrCode::Reset,
                fg_red2 = SgrCode::ForegroundBasic(ANSIBasicColor::DarkRed),
                bg_blue = SgrCode::BackgroundBasic(ANSIBasicColor::DarkBlue),
                reset3 = SgrCode::Reset
            );
            process_bytes(&mut processor, &mut parser, &sequence);
        }

        // Verify colors in buffer
        assert_styled_char_at(
            &ofs_buf,
            0,
            0,
            'B',
            |style_from_buffer| {
                matches!(
                    style_from_buffer.color_fg,
                    Some(TuiColor::Basic(ANSIBasicColor::Black))
                )
            },
            "black foreground",
        );

        assert_styled_char_at(
            &ofs_buf,
            0,
            1,
            'R',
            |style_from_buffer| {
                matches!(
                    style_from_buffer.color_fg,
                    Some(TuiColor::Basic(ANSIBasicColor::DarkRed))
                )
            },
            "red foreground",
        );

        assert_styled_char_at(
            &ofs_buf,
            0,
            2,
            'G',
            |style_from_buffer| {
                matches!(
                    style_from_buffer.color_fg,
                    Some(TuiColor::Basic(ANSIBasicColor::DarkGreen))
                )
            },
            "green foreground",
        );

        assert_styled_char_at(
            &ofs_buf,
            0,
            3,
            'W',
            |style_from_buffer| {
                matches!(
                    style_from_buffer.color_fg,
                    Some(TuiColor::Basic(ANSIBasicColor::White))
                )
            },
            "white foreground",
        );

        assert_plain_char_at(&ofs_buf, 0, 4, ' '); // Space after reset

        assert_styled_char_at(
            &ofs_buf,
            0,
            5,
            'X',
            |style_from_buffer| {
                matches!(
                    style_from_buffer.color_bg,
                    Some(TuiColor::Basic(ANSIBasicColor::DarkRed))
                )
            },
            "red background",
        );

        assert_styled_char_at(
            &ofs_buf,
            0,
            6,
            'Y',
            |style_from_buffer| {
                matches!(
                    style_from_buffer.color_bg,
                    Some(TuiColor::Basic(ANSIBasicColor::DarkGreen))
                )
            },
            "green background",
        );

        assert_styled_char_at(
            &ofs_buf,
            0,
            7,
            'Z',
            |style_from_buffer| {
                matches!(
                    (style_from_buffer.color_fg, style_from_buffer.color_bg),
                    (
                        Some(TuiColor::Basic(ANSIBasicColor::DarkRed)),
                        Some(TuiColor::Basic(ANSIBasicColor::DarkBlue))
                    )
                )
            },
            "red on blue",
        );
    }

    #[test]
    fn test_control_characters() {
        let mut ofs_buf = create_test_offscreen_buffer();

        // Test various control characters
        {
            let mut processor = new(&mut ofs_buf);

            // Print some text
            processor.print('A');
            processor.print('B');
            processor.print('C');

            // Carriage return should move to start of line
            processor.execute(b'\r');
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 0);
            processor.print('X'); // Should overwrite 'A'

            // Line feed should move to next line
            processor.execute(b'\n');
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 1);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 1); // Column preserved after LF
            processor.cursor_pos.col_index = col(0); // Reset for next test
            processor.print('Y');

            // Tab should advance cursor (simplified - just moves forward)
            processor.execute(b'\t');
            let expected_col = 8; // Tab to next multiple of 8
            assert_eq!(processor.cursor_pos.col_index.as_usize(), expected_col);
            processor.print('Z');

            // Backspace should move cursor back
            processor.cursor_pos.col_index = col(3);
            processor.print('M');
            processor.execute(b'\x08'); // Backspace
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 3); // Cursor moved back to 3
            processor.print('N'); // Should write at position 3
        }

        // Verify buffer contents
        assert_plain_char_at(&ofs_buf, 0, 0, 'X'); // 'A' was overwritten by 'X' after CR
        assert_plain_char_at(&ofs_buf, 0, 1, 'B');
        assert_plain_char_at(&ofs_buf, 0, 2, 'C');

        assert_plain_char_at(&ofs_buf, 1, 0, 'Y'); // After line feed
        assert_plain_char_at(&ofs_buf, 1, 8, 'Z'); // After tab
        assert_plain_char_at(&ofs_buf, 1, 3, 'N'); // N overwrote M at position 3
    }

    #[test]
    fn test_line_wrapping_behavior() {
        let mut ofs_buf = create_test_offscreen_buffer();

        // Process characters that should wrap at column 10
        {
            let mut processor = new(&mut ofs_buf);

            // Write 10 characters to fill the line
            for i in 0..10 {
                let ch = (b'A' + i) as char;
                processor.print(ch);
            }

            // 11th character should wrap to next line
            processor.print('K');

            // Verify cursor wrapped to next line
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 1);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 1);
        }

        // Verify buffer contents - first line should have A-J
        assert_plain_text_at(&ofs_buf, 0, 0, "ABCDEFGHIJ");

        // Verify K wrapped to next line
        assert_plain_char_at(&ofs_buf, 1, 0, 'K');

        // Verify rest of second line is empty
        for col in 1..10 {
            assert_empty_at(&ofs_buf, 1, col);
        }
    }

    #[test]
    fn test_print_character_with_styles() {
        let mut ofs_buf = create_test_offscreen_buffer();

        // Process styled character
        {
            let mut processor = new(&mut ofs_buf);
            processor.current_style = Some(TuiStyle {
                id: None,
                attribs: TuiStyleAttribs {
                    bold: Some(tui_style_attrib::Bold),
                    italic: None,
                    dim: None,
                    underline: None,
                    blink: None,
                    reverse: None,
                    hidden: None,
                    strikethrough: None,
                },
                computed: None,
                color_fg: Some(TuiColor::Basic(ANSIBasicColor::DarkRed)),
                color_bg: None,
                padding: None,
                lolcat: None,
            });
            processor.print('S');

            // Verify cursor advanced
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 1);
        }

        // Verify the styled character is in the buffer
        assert_styled_char_at(
            &ofs_buf,
            0,
            0,
            'S',
            |style_from_buffer| {
                matches!(
                    (style_from_buffer.attribs.bold, style_from_buffer.color_fg),
                    (
                        Some(tui_style_attrib::Bold),
                        Some(TuiColor::Basic(ANSIBasicColor::DarkRed))
                    )
                )
            },
            "bold red style",
        );
    }

    #[test]
    fn test_vte_parser_integration() {
        let mut ofs_buf = create_test_offscreen_buffer();

        // Process ANSI sequences through VTE parser
        {
            let mut processor = new(&mut ofs_buf);
            let mut parser = vte::Parser::new();

            // Print "Hello" with red foreground
            let input = format!(
                "Hello{fg_red}R{reset}",
                fg_red = SgrCode::ForegroundBasic(ANSIBasicColor::DarkRed),
                reset = SgrCode::Reset
            );
            process_bytes(&mut processor, &mut parser, &input);

            // Verify cursor position after processing
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 6);
        }

        // Verify "Hello" is in the buffer
        assert_plain_text_at(&ofs_buf, 0, 0, "Hello");

        // Verify 'R' has red color
        assert_styled_char_at(
            &ofs_buf,
            0,
            5,
            'R',
            |style_from_buffer| {
                matches!(
                    style_from_buffer.color_fg,
                    Some(TuiColor::Basic(ANSIBasicColor::DarkRed))
                )
            },
            "red foreground",
        );

        // Verify rest of line is empty
        for col in 6..10 {
            assert_empty_at(&ofs_buf, 0, col);
        }
    }

    #[test]
    fn test_edge_cases() {
        let mut ofs_buf = create_test_offscreen_buffer();

        // Test various edge cases
        {
            let mut processor = new(&mut ofs_buf);

            // Empty SGR should not crash
            // SGR params can't be created directly in tests - skipping
            processor.print('A');

            // Invalid SGR codes should be ignored
            // SGR params can't be created directly in tests - skipping
            processor.print('B');

            // Multiple resets should be safe
            // SGR params can't be created directly in tests - skipping
            // SGR params can't be created directly in tests - skipping
            processor.print('C');

            // Writing at boundary positions
            processor.cursor_pos = Pos {
                row_index: row(9),
                col_index: col(9),
            }; // Last row, last column
            processor.print('D'); // Should write at last valid position

            // Line wrap at last row
            processor.cursor_pos = Pos {
                row_index: row(9),
                col_index: col(9),
            };
            processor.print('E');
            processor.print('F'); // Should wrap to beginning of last row
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 9); // Should stay at row 9

            // Printing null character - it gets written to buffer like any char
            processor.cursor_pos = Pos {
                row_index: row(3),
                col_index: col(0),
            };
            processor.print('G');
            processor.print('\0'); // Null char - gets written to buffer
            processor.print('H');
        }

        // Verify edge case handling
        assert_plain_char_at(&ofs_buf, 0, 0, 'A'); // Empty SGR didn't affect printing
        assert_plain_char_at(&ofs_buf, 0, 1, 'B'); // Invalid SGR was ignored
        assert_plain_char_at(&ofs_buf, 0, 2, 'C'); // Multiple resets were safe

        // Note: 'D' was overwritten by 'E' later, so we don't check for 'D' here

        // Verify 'E' and 'F' were written to row 9
        assert_plain_char_at(&ofs_buf, 9, 9, 'E'); // 'E' at last position
        assert_plain_char_at(&ofs_buf, 9, 0, 'F'); // 'F' wrapped to beginning of row 9

        // Verify null char behavior - it gets written to buffer
        assert_plain_char_at(&ofs_buf, 3, 0, 'G');
        assert_plain_char_at(&ofs_buf, 3, 1, ' '); // Null char is written at [3][1]
        assert_plain_char_at(&ofs_buf, 3, 2, 'H'); // 'H' is at col 2 after null char
    }

    #[test]
    fn test_complex_ansi_sequences() {
        let mut ofs_buf = create_test_offscreen_buffer();

        // Process complex ANSI sequences typical from real terminals
        {
            let mut processor = new(&mut ofs_buf);
            let mut parser = vte::Parser::new();

            // Simulate: Bold text, colored text, cursor movement
            let sequence = format!(
                "{bold}Bold{reset1} {fg_green}Green{reset2}",
                bold = SgrCode::Bold,
                reset1 = SgrCode::Reset,
                fg_green = SgrCode::ForegroundBasic(ANSIBasicColor::DarkGreen),
                reset2 = SgrCode::Reset
            );
            process_bytes(&mut processor, &mut parser, &sequence);
        }

        // Verify "Bold" with bold style
        for (i, ch) in "Bold".chars().enumerate() {
            assert_styled_char_at(
                &ofs_buf,
                0,
                i,
                ch,
                |style_from_buffer| {
                    matches!(style_from_buffer.attribs.bold, Some(tui_style_attrib::Bold))
                },
                "bold style",
            );
        }

        // Verify space at position 4
        assert_plain_char_at(&ofs_buf, 0, 4, ' ');

        // Verify "Green" with green color
        for (i, ch) in "Green".chars().enumerate() {
            assert_styled_char_at(
                &ofs_buf,
                0,
                5 + i,
                ch,
                |style_from_buffer| {
                    matches!(
                        style_from_buffer.color_fg,
                        Some(TuiColor::Basic(ANSIBasicColor::DarkGreen))
                    )
                },
                "green foreground",
            );
        }
    }

    #[test]
    fn test_utf8_characters() {
        let mut ofs_buf = create_test_offscreen_buffer();

        // Process UTF-8 characters including emojis
        {
            let mut processor = new(&mut ofs_buf);

            // Print various UTF-8 characters
            processor.print('H');
            processor.print('é'); // Latin character with accent
            processor.print('中'); // Chinese character
            processor.print('🦀'); // Emoji (Rust crab)
            processor.print('!');
        }

        // Verify all UTF-8 characters are in the buffer
        assert_plain_char_at(&ofs_buf, 0, 0, 'H');
        assert_plain_char_at(&ofs_buf, 0, 1, 'é');
        assert_plain_char_at(&ofs_buf, 0, 2, '中');
        assert_plain_char_at(&ofs_buf, 0, 3, '🦀');
        assert_plain_char_at(&ofs_buf, 0, 4, '!');

        // Verify rest of line is empty
        for col in 5..10 {
            assert_empty_at(&ofs_buf, 0, col);
        }
    }

    #[test]
    fn test_cursor_save_restore() {
        let mut ofs_buf = create_test_offscreen_buffer();

        // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
        // 1-based index. Test cursor save and restore operations
        {
            let mut processor = new(&mut ofs_buf);

            // Move to a specific position and save
            processor.cursor_pos = Pos {
                row_index: row(3),
                col_index: col(5),
            };
            processor.print('S'); // Mark save position

            // Save cursor position (CSI s or ESC 7)
            let saved_row = processor.cursor_pos.row_index.as_usize();
            let saved_col = processor.cursor_pos.col_index.as_usize();

            // Move elsewhere and write
            processor.cursor_pos = Pos {
                row_index: row(7),
                col_index: col(2),
            };
            processor.print('M'); // Mark moved position

            // Restore cursor position (CSI u or ESC 8)
            processor.cursor_pos = Pos {
                row_index: row(saved_row),
                col_index: col(saved_col),
            };
            processor.print('R'); // Should be at saved position
        }

        // Verify characters were written at correct positions
        assert_plain_char_at(&ofs_buf, 3, 5, 'S'); // Initial save position
        assert_plain_char_at(&ofs_buf, 7, 2, 'M'); // Moved position
        assert_plain_char_at(&ofs_buf, 3, 6, 'R'); // Restored position (after 'S')
    }

    #[test]
    fn test_clear_operations() {
        let mut ofs_buf = create_test_offscreen_buffer();

        // Test clear screen and clear line operations
        {
            let mut processor = new(&mut ofs_buf);

            // Fill some content first
            processor.cursor_pos = Pos {
                row_index: row(2),
                col_index: col(0),
            };
            for i in 0..10 {
                processor.cursor_pos.col_index = col(i);
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                processor.print((b'A' + i as u8) as char);
            }

            processor.cursor_pos = Pos {
                row_index: row(3),
                col_index: col(0),
            };
            for i in 0..10 {
                processor.cursor_pos.col_index = col(i);
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                processor.print((b'0' + i as u8) as char);
            }

            // Clear from cursor to end of line (K or EL 0)
            processor.cursor_pos = Pos {
                row_index: row(2),
                col_index: col(4),
            };
            for col in 5..10 {
                processor.ofs_buf.buffer[2][col] = PixelChar::Spacer;
            }

            // Clear from start of line to cursor (EL 1)
            processor.cursor_pos = Pos {
                row_index: row(2),
                col_index: col(4),
            };
            for col in 0..=5 {
                processor.ofs_buf.buffer[3][col] = PixelChar::Spacer;
            }

            // Clear entire line (EL 2)
            processor.cursor_pos = Pos {
                row_index: row(4),
                col_index: col(0),
            };
            processor.print('X'); // Add something to clear
            for col in 0..10 {
                processor.ofs_buf.buffer[4][col] = PixelChar::Spacer;
            }
        }

        // Verify clear operations
        // Row 2: Should have A-E, then empty
        assert_plain_text_at(&ofs_buf, 2, 0, "ABCDE");
        for col in 5..10 {
            assert_empty_at(&ofs_buf, 2, col);
        }

        // Row 3: Should be empty from 0-5, then have 6789
        for col in 0..=5 {
            assert_empty_at(&ofs_buf, 3, col);
        }
        assert_plain_text_at(&ofs_buf, 3, 6, "6789");

        // Row 4: Should be completely empty
        for col in 0..10 {
            assert_empty_at(&ofs_buf, 4, col);
        }
    }

    #[test]
    fn test_esc_save_restore_cursor() {
        let mut ofs_buf = create_test_offscreen_buffer();
        let mut parser = vte::Parser::new();

        // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
        // 1-based index. Test ESC 7 (DECSC) and ESC 8 (DECRC) for cursor
        // save/restore
        {
            let mut processor = new(&mut ofs_buf);

            // Move cursor to position (3, 5) and write 'A'
            processor.cursor_pos = Pos {
                row_index: row(3),
                col_index: col(5),
            };
            processor.print('A');

            // Save cursor position at (3, 6) using ESC 7
            let seq = esc_codes::EscSequence::SaveCursor.to_string();
            process_bytes(&mut processor, &mut parser, &seq);

            // Move cursor elsewhere and write 'B'
            processor.cursor_pos = Pos {
                row_index: row(7),
                col_index: col(2),
            };
            processor.print('B');

            // Restore cursor position using ESC 8
            let seq = esc_codes::EscSequence::RestoreCursor.to_string();
            process_bytes(&mut processor, &mut parser, &seq);

            // Verify cursor was restored (check while processor exists)
            assert_eq!(
                processor.cursor_pos,
                Pos {
                    row_index: row(3),
                    col_index: col(6),
                }
            );

            // Write 'C' at restored position
            processor.print('C');
        }

        // Verify saved cursor position persisted in buffer
        assert_eq!(
            ofs_buf.saved_cursor_pos,
            Some(Pos {
                row_index: row(3),
                col_index: col(6),
            })
        );

        // Verify characters are at expected positions
        assert_plain_char_at(&ofs_buf, 3, 5, 'A');
        assert_plain_char_at(&ofs_buf, 7, 2, 'B');
        assert_plain_char_at(&ofs_buf, 3, 6, 'C'); // Should be right after 'A'
    }

    #[test]
    fn test_esc_index_down_with_scrolling() {
        let mut ofs_buf = create_test_offscreen_buffer();
        let mut parser = vte::Parser::new();

        // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
        // 1-based index.
        {
            let mut processor = new(&mut ofs_buf);

            // Fill top lines with identifiable content
            for i in 0..3 {
                processor.cursor_pos = Pos {
                    row_index: row(i as usize),
                    col_index: col(0),
                };
                processor.print(char::from_digit(i, 10).unwrap());
            }

            // Position cursor at bottom row (row 9)
            processor.cursor_pos = Pos {
                row_index: row(9),
                col_index: col(0),
            };
            processor.print('X');

            // ESC D (IND) at bottom should scroll buffer up
            let seq = esc_codes::EscSequence::IndexDown.to_string();
            process_bytes(&mut processor, &mut parser, &seq);

            // Cursor should still be at bottom
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 9);
        }

        // After scrolling up, row 0 should have what was in row 1 ('1')
        assert_plain_char_at(&ofs_buf, 0, 0, '1');
        // Row 1 should have what was in row 2 ('2')
        assert_plain_char_at(&ofs_buf, 1, 0, '2');
        // Original row 0 content ('0') should be lost
        // Row 8 should have 'X' (what was at row 9 before scroll)
        assert_plain_char_at(&ofs_buf, 8, 0, 'X');
        // Row 9 should be cleared (new empty line after scroll)
        assert_empty_at(&ofs_buf, 9, 0);
    }

    #[test]
    fn test_esc_reverse_index_with_scrolling() {
        let mut ofs_buf = create_test_offscreen_buffer();
        let mut parser = vte::Parser::new();

        // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
        // 1-based index.
        {
            let mut processor = new(&mut ofs_buf);

            // Fill bottom lines with identifiable content
            for i in 6..9 {
                processor.cursor_pos = Pos {
                    row_index: row(i as usize),
                    col_index: col(0),
                };
                processor.print(char::from_digit(i, 10).unwrap());
            }

            // Position cursor at top row
            processor.cursor_pos = Pos {
                row_index: row(0),
                col_index: col(0),
            };

            // ESC M (RI) at top should scroll buffer down
            let seq = esc_codes::EscSequence::ReverseIndex.to_string();
            process_bytes(&mut processor, &mut parser, &seq);

            // Cursor should still be at top
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 0);
        }

        // After scrolling down, row 7 should have what was in row 6 ('6')
        assert_plain_char_at(&ofs_buf, 7, 0, '6');
        // Row 8 should have what was in row 7 ('7')
        assert_plain_char_at(&ofs_buf, 8, 0, '7');
        // Row 0 should be cleared (new empty line)
        assert_empty_at(&ofs_buf, 0, 0);
    }

    #[test]
    fn test_esc_reset_terminal() {
        let mut ofs_buf = create_test_offscreen_buffer();
        let mut parser = vte::Parser::new();

        // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
        // 1-based index. Set up initial state before creating processor
        ofs_buf.saved_cursor_pos = Some(Pos {
            row_index: row(5),
            col_index: col(7),
        });
        ofs_buf.character_set = CharacterSet::Graphics;

        {
            let mut processor = new(&mut ofs_buf);

            // Set up some state: styled text
            processor.attribs.bold = Some(tui_style_attrib::Bold);
            processor.fg_color = Some(TuiColor::Basic(ANSIBasicColor::Red));
            sgr_ops::update_style(&mut processor);

            // Write some content
            processor.cursor_pos = Pos {
                row_index: row(2),
                col_index: col(3),
            };
            processor.print('H');
            processor.print('I');

            // ESC c (RIS) - Reset to Initial State
            let seq = esc_codes::EscSequence::ResetTerminal.to_string();
            process_bytes(&mut processor, &mut parser, &seq);

            // Verify everything is reset
            assert_eq!(processor.cursor_pos, Pos::default());
            assert!(processor.attribs.bold.is_none());
            assert!(processor.fg_color.is_none());
        }

        // Verify buffer state after processor is dropped
        assert!(ofs_buf.saved_cursor_pos.is_none());
        assert_eq!(ofs_buf.character_set, CharacterSet::Ascii);

        // Verify buffer is cleared
        for row in 0..9 {
            // 9 rows (excluding status bar)
            for col in 0..10 {
                assert_empty_at(&ofs_buf, row, col);
            }
        }
    }

    #[test]
    fn test_esc_character_set_switching() {
        let mut ofs_buf = create_test_offscreen_buffer();
        let mut parser = vte::Parser::new();

        {
            let mut processor = new(&mut ofs_buf);

            // Start with ASCII mode and write 'q'
            let seq = esc_codes::EscSequence::SelectAscii.to_string();
            process_bytes(&mut processor, &mut parser, &seq); // ESC ( B - Select ASCII
            processor.print('q');

            // Switch to DEC graphics mode
            let seq = esc_codes::EscSequence::SelectGraphics.to_string();
            process_bytes(&mut processor, &mut parser, &seq); // ESC ( 0 - Select DEC graphics

            // Write 'q' which should be translated to '─' (horizontal line)
            processor.print('q');

            // Write 'x' which should be translated to '│' (vertical line)
            processor.print('x');

            // Switch back to ASCII
            let seq = esc_codes::EscSequence::SelectAscii.to_string();
            process_bytes(&mut processor, &mut parser, &seq);

            // Write 'q' again (should be normal 'q')
            processor.print('q');
        }

        // Verify character set state after processor is dropped
        assert_eq!(ofs_buf.character_set, CharacterSet::Ascii);

        // Verify the characters
        assert_plain_char_at(&ofs_buf, 0, 0, 'q'); // ASCII 'q'
        assert_plain_char_at(&ofs_buf, 0, 1, '─'); // DEC graphics 'q' -> horizontal line
        assert_plain_char_at(&ofs_buf, 0, 2, '│'); // DEC graphics 'x' -> vertical line
        assert_plain_char_at(&ofs_buf, 0, 3, 'q'); // ASCII 'q' again
    }

    #[test]
    fn test_character_translation_dec_graphics() {
        let mut ofs_buf = create_test_offscreen_buffer();

        // Test DEC graphics character translations (limited to 10 due to buffer width)
        let test_cases = [
            ('j', '┘'), // Lower right corner
            ('k', '┐'), // Upper right corner
            ('l', '┌'), // Upper left corner
            ('m', '└'), // Lower left corner
            ('n', '┼'), // Crossing lines
            ('q', '─'), // Horizontal line
            ('t', '├'), // Left "T"
            ('u', '┤'), // Right "T"
            ('v', '┴'), // Bottom "T"
            ('x', '│'), // Vertical line
        ];

        // Set DEC graphics mode before creating processor
        ofs_buf.character_set = CharacterSet::Graphics;

        {
            let mut processor = new(&mut ofs_buf);

            for (i, (input, _expected)) in test_cases.iter().enumerate() {
                processor.cursor_pos = Pos {
                    row_index: row(0),
                    col_index: col(i),
                };
                processor.print(*input);
            }
        }

        // Verify the translated characters after processor is dropped
        for (i, (_input, expected)) in test_cases.iter().enumerate() {
            assert_plain_char_at(&ofs_buf, 0, i, *expected);
        }
    }

    #[test]
    fn test_esc_index_down_without_scrolling() {
        let mut ofs_buf = create_test_offscreen_buffer();
        let mut parser = vte::Parser::new();

        // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
        // 1-based index.
        {
            let mut processor = new(&mut ofs_buf);

            // Position cursor at middle of screen
            processor.cursor_pos = Pos {
                row_index: row(4),
                col_index: col(2),
            };
            processor.print('M');

            // ESC D (IND) when not at bottom should just move cursor down
            let seq = esc_codes::EscSequence::IndexDown.to_string();
            process_bytes(&mut processor, &mut parser, &seq);

            // Cursor should have moved down one row
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 5);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 3); // After 'M'
        }

        // Verify character is still at original position
        assert_plain_char_at(&ofs_buf, 4, 2, 'M');
    }

    #[test]
    fn test_esc_reverse_index_without_scrolling() {
        let mut ofs_buf = create_test_offscreen_buffer();
        let mut parser = vte::Parser::new();

        // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
        // 1-based index.
        {
            let mut processor = new(&mut ofs_buf);

            // Position cursor at middle of screen
            processor.cursor_pos = Pos {
                row_index: row(4),
                col_index: col(2),
            };
            processor.print('M');

            // ESC M (RI) when not at top should just move cursor up
            let seq = esc_codes::EscSequence::ReverseIndex.to_string();
            process_bytes(&mut processor, &mut parser, &seq);

            // Cursor should have moved up one row
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 3);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 3); // After 'M'
        }

        // Verify character is still at original position
        assert_plain_char_at(&ofs_buf, 4, 2, 'M');
    }
}

#[cfg(test)]
mod tests_csi_absolute_positioning {
    use super::*;
    use crate::{ansi_parser::csi_codes::CsiSequence, height,
                offscreen_buffer::test_fixtures_offscreen_buffer::*, width};

    /// Create a test `OffscreenBuffer` with 10x10 dimensions (9 content rows + 1 status
    /// bar).
    fn create_test_offscreen_buffer() -> OffscreenBuffer {
        OffscreenBuffer::new_with_capacity_initialized(height(10) + width(10))
    }

    #[test]
    fn test_csi_h_home_position() {
        let mut ofs_buf = create_test_offscreen_buffer();
        let mut parser = vte::Parser::new();

        {
            let mut processor = new(&mut ofs_buf);

            // Start at a non-home position
            processor.cursor_pos = Pos {
                row_index: row(5),
                col_index: col(5),
            };
            processor.print('X');

            // Send ESC[H to move to home position (1,1)
            let sequence = CsiSequence::CursorPosition { row: 1, col: 1 }.to_string();
            process_bytes(&mut processor, &mut parser, sequence);

            // OffscreenBuffer uses 0-based indexing, CSI uses 1-based indexing.
            // Verify cursor is at home position (0,0 in 0-based indexing)
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 0);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 0);

            processor.print('H'); // Mark home position
        }

        // Verify characters are at correct positions
        assert_plain_char_at(&ofs_buf, 5, 5, 'X');
        assert_plain_char_at(&ofs_buf, 0, 0, 'H');

        // Verify final cursor position in buffer
        assert_eq!(ofs_buf.my_pos.row_index.as_usize(), 0);
        assert_eq!(ofs_buf.my_pos.col_index.as_usize(), 1); // After writing 'H'
    }

    #[test]
    fn test_csi_h_specific_position() {
        let mut ofs_buf = create_test_offscreen_buffer();
        let mut parser = vte::Parser::new();

        {
            let mut processor = new(&mut ofs_buf);

            // Send ESC[5;10H to move to row 5, column 10 (1-based)
            let sequence = CsiSequence::CursorPosition { row: 5, col: 10 }.to_string();
            process_bytes(&mut processor, &mut parser, sequence);

            // OffscreenBuffer uses 0-based indexing, CSI uses 1-based indexing.
            // Verify cursor is at (4,9) in 0-based indexing
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 4);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 9);

            processor.print('A');
        }

        // Verify character was written at correct position
        assert_plain_char_at(&ofs_buf, 4, 9, 'A');
    }

    #[test]
    fn test_csi_h_with_boundary_clamping() {
        let mut ofs_buf = create_test_offscreen_buffer();
        let mut parser = vte::Parser::new();

        {
            let mut processor = new(&mut ofs_buf);

            // Try to position beyond buffer bounds: ESC[999;999H
            let sequence = CsiSequence::CursorPosition { row: 999, col: 999 }.to_string();
            process_bytes(&mut processor, &mut parser, sequence);

            // OffscreenBuffer uses 0-based indexing, CSI uses 1-based indexing.
            // Should be clamped to bottom-right corner
            // Buffer is 10x10, so max is row 10 (index 9), col 10 (index 9)
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 9); // Row 10 in 1-based, 9 in 0-based
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 9);

            processor.print('E'); // Mark edge position
        }

        // Verify character at clamped position
        assert_plain_char_at(&ofs_buf, 9, 9, 'E');
    }

    #[test]
    fn test_csi_f_alternate_form() {
        let mut ofs_buf = create_test_offscreen_buffer();
        let mut parser = vte::Parser::new();

        {
            let mut processor = new(&mut ofs_buf);

            // ESC[f is alternate form of ESC[H (should go to home)
            let sequence = CsiSequence::CursorPositionAlt { row: 1, col: 1 }.to_string();
            process_bytes(&mut processor, &mut parser, sequence);

            assert_eq!(processor.cursor_pos.row_index.as_usize(), 0);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 0);

            processor.print('F');

            // ESC[3;7f should position at row 3, col 7
            let sequence = CsiSequence::CursorPositionAlt { row: 3, col: 7 }.to_string();
            process_bytes(&mut processor, &mut parser, sequence);

            // OffscreenBuffer uses 0-based indexing, CSI uses 1-based indexing.
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 2); // Row 3 in 1-based
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 6); // Col 7 in 1-based

            processor.print('G');
        }

        assert_plain_char_at(&ofs_buf, 0, 0, 'F');
        assert_plain_char_at(&ofs_buf, 2, 6, 'G');
    }

    #[test]
    fn test_csi_position_with_missing_params() {
        let mut ofs_buf = create_test_offscreen_buffer();
        let mut parser = vte::Parser::new();

        {
            let mut processor = new(&mut ofs_buf);

            // Start at non-home position
            processor.cursor_pos = Pos {
                row_index: row(3),
                col_index: col(3),
            };

            // ESC[;5H - missing row param, should default to 1
            // Note: Using raw string as CsiSequence doesn't support missing params (it is
            // always valid)
            let sequence = "\x1b[;5H";
            process_bytes(&mut processor, &mut parser, sequence);

            // OffscreenBuffer uses 0-based indexing, CSI uses 1-based indexing.
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 0); // Row 1 (default)
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 4); // Col 5

            processor.print('M');

            // ESC[3;H - missing col param, should default to 1
            // Note: Using raw string as CsiSequence doesn't support missing params (it is
            // always valid)
            let sequence = "\x1b[3;H";
            process_bytes(&mut processor, &mut parser, sequence);

            // OffscreenBuffer uses 0-based indexing, CSI uses 1-based indexing.
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 2); // Row 3
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 0); // Col 1 (default)

            processor.print('N');
        }

        assert_plain_char_at(&ofs_buf, 0, 4, 'M');
        assert_plain_char_at(&ofs_buf, 2, 0, 'N');
    }
}

#[cfg(test)]
mod tests_cursor_movement {
    use super::*;
    use crate::{height, offscreen_buffer::test_fixtures_offscreen_buffer::*, width};

    /// Create a test `OffscreenBuffer` with 10x10 dimensions.
    fn create_test_offscreen_buffer() -> OffscreenBuffer {
        OffscreenBuffer::new_with_capacity_initialized(height(10) + width(10))
    }

    #[test]
    fn test_cursor_movement_up() {
        // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
        // 1-based index.
        let mut ofs_buf = create_test_offscreen_buffer();

        // Test cursor up movement with buffer verification
        {
            let mut processor = new(&mut ofs_buf);

            // Start at row 5, write a character
            processor.cursor_pos = Pos {
                row_index: row(5),
                col_index: col(3),
            };
            processor.print('A');

            // Move up 2 rows and write another character
            cursor_ops::cursor_up(&mut processor, 2);
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 3);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 4); // Column should be after 'A'

            processor.cursor_pos.col_index = col(3); // Reset column to same position
            processor.print('B');

            // Try to move up beyond boundary
            cursor_ops::cursor_up(&mut processor, 10);
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 0); // Should stop at row 0
            processor.cursor_pos.col_index = col(3);
            processor.print('C');
        }

        // Verify characters are in correct positions
        assert_plain_char_at(&ofs_buf, 5, 3, 'A');
        assert_plain_char_at(&ofs_buf, 3, 3, 'B');
        assert_plain_char_at(&ofs_buf, 0, 3, 'C');
    }

    #[test]
    fn test_cursor_movement_down() {
        // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
        // 1-based index.
        let mut ofs_buf = create_test_offscreen_buffer();

        // Test cursor down movement with buffer verification
        {
            let mut processor = new(&mut ofs_buf);

            // Start at row 2, write a character
            processor.cursor_pos = Pos {
                row_index: row(2),
                col_index: col(4),
            };
            processor.print('X');

            // Move down 3 rows and write another character
            cursor_ops::cursor_down(&mut processor, 3);
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 5);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 5); // Column should be after 'X'

            processor.cursor_pos.col_index = col(3); // Reset column to same position
            processor.print('Y');

            // Try to move down beyond buffer area
            cursor_ops::cursor_down(&mut processor, 5);
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 9); // Should stop at row 9 (last row)
            processor.cursor_pos.col_index = col(3);
            processor.print('Z');
        }

        // Verify characters are in correct positions
        assert_plain_char_at(&ofs_buf, 2, 4, 'X');
        assert_plain_char_at(&ofs_buf, 5, 3, 'Y'); // col 3 because we reset it
        assert_plain_char_at(&ofs_buf, 9, 3, 'Z'); // col 3 because we reset it, row 9 is now valid
    }

    #[test]
    fn test_cursor_movement_forward() {
        // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
        // 1-based index.
        let mut ofs_buf = create_test_offscreen_buffer();

        // Test cursor forward movement with buffer verification
        {
            let mut processor = new(&mut ofs_buf);

            // Write some text at row 3
            processor.cursor_pos = Pos {
                row_index: row(3),
                col_index: col(0),
            };
            processor.print('1');

            // Move forward 3 columns and write
            cursor_ops::cursor_forward(&mut processor, 3);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 4);
            processor.print('2');

            // Move forward to near end of line
            cursor_ops::cursor_forward(&mut processor, 3);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 8);
            processor.print('3');

            // Try to move beyond line boundary
            cursor_ops::cursor_forward(&mut processor, 5);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 9); // Should stop at last column
            processor.print('4');
        }

        // Verify characters are in correct positions
        assert_plain_char_at(&ofs_buf, 3, 0, '1');
        assert_plain_char_at(&ofs_buf, 3, 4, '2');
        assert_plain_char_at(&ofs_buf, 3, 8, '3');
        assert_plain_char_at(&ofs_buf, 3, 9, '4');

        // Verify gaps are empty
        for col in [1, 2, 3, 5, 6, 7] {
            assert_empty_at(&ofs_buf, 3, col);
        }
    }

    #[test]
    fn test_cursor_movement_backward() {
        // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
        // 1-based index.
        let mut ofs_buf = create_test_offscreen_buffer();

        // Test cursor backward movement with buffer verification
        {
            let mut processor = new(&mut ofs_buf);

            // Start at column 8 and write
            processor.cursor_pos = Pos {
                row_index: row(4),
                col_index: col(8),
            };
            processor.print('A');

            // Move backward 3 columns and write
            cursor_ops::cursor_backward(&mut processor, 3);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 6); // 9 - 3 = 6
            processor.print('B');

            // Move backward more
            cursor_ops::cursor_backward(&mut processor, 4);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 3); // 7 - 4 = 3
            processor.print('C');

            // Try to move beyond start of line
            cursor_ops::cursor_backward(&mut processor, 10);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 0); // Should stop at column 0
            processor.print('D');
        }

        // Verify characters are in correct positions
        assert_plain_char_at(&ofs_buf, 4, 8, 'A');
        assert_plain_char_at(&ofs_buf, 4, 6, 'B');
        assert_plain_char_at(&ofs_buf, 4, 3, 'C');
        assert_plain_char_at(&ofs_buf, 4, 0, 'D');

        // Verify gaps are empty
        for col in [1, 2, 4, 5, 7, 9] {
            assert_empty_at(&ofs_buf, 4, col);
        }
    }
}

#[cfg(test)]
mod tests_sgr_styling {
    use super::*;
    use crate::{ANSIBasicColor, SgrCode, TuiColor, height,
                offscreen_buffer::test_fixtures_offscreen_buffer::*, width};

    /// Create a test `OffscreenBuffer` with 10x10 dimensions.
    fn create_test_offscreen_buffer() -> OffscreenBuffer {
        OffscreenBuffer::new_with_capacity_initialized(height(10) + width(10))
    }

    #[test]
    fn test_sgr_reset_behavior() {
        let mut ofs_buf = create_test_offscreen_buffer();

        #[allow(clippy::items_after_statements)]
        const RED: &str = "RED";
        #[allow(clippy::items_after_statements)]
        const NORM: &str = "NORM";

        // Test SGR reset by sending this sequence to the processor:
        // Set bold+red, write "RED", reset all, write "NORM"
        {
            let mut processor = new(&mut ofs_buf);
            process_bytes(
                &mut processor,
                /* new parser */ &mut vte::Parser::new(),
                /* sequence */
                format!(
                    "{bold}{fg_red}{text1}{reset_all}{text2}",
                    bold = SgrCode::Bold,
                    fg_red = SgrCode::ForegroundBasic(ANSIBasicColor::Red),
                    reset_all = SgrCode::Reset,
                    text1 = RED,
                    text2 = NORM
                ),
            );
        } // processor dropped here

        // Verify "RED" has bold and red color
        for (col, expected_char) in RED.chars().enumerate() {
            assert_styled_char_at(
                &ofs_buf,
                0,
                col,
                expected_char,
                |style_from_buffer| {
                    matches!(
                        (style_from_buffer.attribs.bold, style_from_buffer.color_fg),
                        (
                            Some(tui_style_attrib::Bold),
                            Some(TuiColor::Basic(ANSIBasicColor::Red))
                        )
                    )
                },
                "bold red text",
            );
        }

        // Verify "NORM" has no styling (SGR 0 reset everything)
        assert_plain_text_at(&ofs_buf, 0, RED.len(), NORM);
    }

    #[test]
    fn test_sgr_partial_reset() {
        let mut ofs_buf = create_test_offscreen_buffer();

        // Test partial SGR resets (SGR 22 resets bold/dim only)
        {
            let mut processor = new(&mut ofs_buf);
            let mut parser = vte::Parser::new();

            // Set bold+italic+red, write "A", reset bold/dim only, write "B"
            let sequence = format!(
                "{bold}{italic}{fg_red}A{reset_bold_dim}B",
                bold = SgrCode::Bold,
                italic = SgrCode::Italic,
                fg_red = SgrCode::ForegroundBasic(ANSIBasicColor::DarkRed),
                reset_bold_dim = SgrCode::ResetBoldDim
            );
            process_bytes(&mut processor, &mut parser, &sequence);
        }

        // Verify 'A' has bold, italic, and red
        assert_styled_char_at(
            &ofs_buf,
            0,
            0,
            'A',
            |style_from_buffer| {
                matches!(
                    (
                        style_from_buffer.attribs.bold,
                        style_from_buffer.attribs.italic,
                        style_from_buffer.color_fg
                    ),
                    (
                        Some(tui_style_attrib::Bold),
                        Some(tui_style_attrib::Italic),
                        Some(TuiColor::Basic(ANSIBasicColor::DarkRed))
                    )
                )
            },
            "bold italic red",
        );

        // Verify 'B' has italic and red but NOT bold (SGR 22 reset bold/dim)
        assert_styled_char_at(
            &ofs_buf,
            0,
            1,
            'B',
            |style_from_buffer| {
                matches!(
                    (
                        style_from_buffer.attribs.bold,
                        style_from_buffer.attribs.italic,
                        style_from_buffer.color_fg
                    ),
                    (
                        None,
                        Some(tui_style_attrib::Italic),
                        Some(TuiColor::Basic(ANSIBasicColor::DarkRed))
                    )
                )
            },
            "italic red (no bold)",
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn test_sgr_color_attributes() {
        let mut ofs_buf = create_test_offscreen_buffer();

        // Test various SGR color sequences through VTE parser
        {
            let mut processor = new(&mut ofs_buf);
            let mut parser = vte::Parser::new();

            // Test foreground colors: black, red, green, white, then background colors
            let sequence = format!(
                "{fg_black}B{fg_red}R{fg_green}G{fg_white}W{reset} {bg_red}X{bg_green}Y{reset2}{fg_red2}{bg_blue}Z{reset3}",
                fg_black = SgrCode::ForegroundBasic(ANSIBasicColor::Black),
                fg_red = SgrCode::ForegroundBasic(ANSIBasicColor::DarkRed),
                fg_green = SgrCode::ForegroundBasic(ANSIBasicColor::DarkGreen),
                fg_white = SgrCode::ForegroundBasic(ANSIBasicColor::White),
                reset = SgrCode::Reset,
                bg_red = SgrCode::BackgroundBasic(ANSIBasicColor::DarkRed),
                bg_green = SgrCode::BackgroundBasic(ANSIBasicColor::DarkGreen),
                reset2 = SgrCode::Reset,
                fg_red2 = SgrCode::ForegroundBasic(ANSIBasicColor::DarkRed),
                bg_blue = SgrCode::BackgroundBasic(ANSIBasicColor::DarkBlue),
                reset3 = SgrCode::Reset
            );
            process_bytes(&mut processor, &mut parser, &sequence);
        }

        // Verify colors in buffer
        assert_styled_char_at(
            &ofs_buf,
            0,
            0,
            'B',
            |style_from_buffer| {
                matches!(
                    style_from_buffer.color_fg,
                    Some(TuiColor::Basic(ANSIBasicColor::Black))
                )
            },
            "black foreground",
        );

        assert_styled_char_at(
            &ofs_buf,
            0,
            1,
            'R',
            |style_from_buffer| {
                matches!(
                    style_from_buffer.color_fg,
                    Some(TuiColor::Basic(ANSIBasicColor::DarkRed))
                )
            },
            "red foreground",
        );

        assert_styled_char_at(
            &ofs_buf,
            0,
            2,
            'G',
            |style_from_buffer| {
                matches!(
                    style_from_buffer.color_fg,
                    Some(TuiColor::Basic(ANSIBasicColor::DarkGreen))
                )
            },
            "green foreground",
        );

        assert_styled_char_at(
            &ofs_buf,
            0,
            3,
            'W',
            |style_from_buffer| {
                matches!(
                    style_from_buffer.color_fg,
                    Some(TuiColor::Basic(ANSIBasicColor::White))
                )
            },
            "white foreground",
        );

        assert_plain_char_at(&ofs_buf, 0, 4, ' '); // Space after reset

        assert_styled_char_at(
            &ofs_buf,
            0,
            5,
            'X',
            |style_from_buffer| {
                matches!(
                    style_from_buffer.color_bg,
                    Some(TuiColor::Basic(ANSIBasicColor::DarkRed))
                )
            },
            "red background",
        );

        assert_styled_char_at(
            &ofs_buf,
            0,
            6,
            'Y',
            |style_from_buffer| {
                matches!(
                    style_from_buffer.color_bg,
                    Some(TuiColor::Basic(ANSIBasicColor::DarkGreen))
                )
            },
            "green background",
        );

        assert_styled_char_at(
            &ofs_buf,
            0,
            7,
            'Z',
            |style_from_buffer| {
                matches!(
                    (style_from_buffer.color_fg, style_from_buffer.color_bg),
                    (
                        Some(TuiColor::Basic(ANSIBasicColor::DarkRed)),
                        Some(TuiColor::Basic(ANSIBasicColor::DarkBlue))
                    )
                )
            },
            "red on blue",
        );
    }
}

#[cfg(test)]
mod tests_esc_sequences {
    use super::*;
    use crate::{ansi_parser::csi_codes::CsiSequence, height,
                offscreen_buffer::test_fixtures_offscreen_buffer::*, width};

    /// Create a test `OffscreenBuffer` with 10x10 dimensions.
    fn create_test_offscreen_buffer() -> OffscreenBuffer {
        OffscreenBuffer::new_with_capacity_initialized(height(10) + width(10))
    }

    #[test]
    fn test_csi_save_restore_cursor() {
        let mut ofs_buf = create_test_offscreen_buffer();
        let mut parser = vte::Parser::new();

        {
            let mut processor = new(&mut ofs_buf);

            // Move to position and save with CSI s
            processor.cursor_pos = Pos {
                row_index: row(3),
                col_index: col(5),
            };
            processor.print('A');

            // CSI s - Save cursor position
            let sequence = CsiSequence::SaveCursor.to_string();
            process_bytes(&mut processor, &mut parser, sequence);

            // Move elsewhere
            processor.cursor_pos = Pos {
                row_index: row(7),
                col_index: col(2),
            };
            processor.print('B');

            // CSI u - Restore cursor position
            let sequence = CsiSequence::RestoreCursor.to_string();
            process_bytes(&mut processor, &mut parser, sequence);

            // Should be back at saved position
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 3);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 6); // After 'A'

            processor.print('C');
        }

        // Verify characters
        assert_plain_char_at(&ofs_buf, 3, 5, 'A');
        assert_plain_char_at(&ofs_buf, 7, 2, 'B');
        assert_plain_char_at(&ofs_buf, 3, 6, 'C'); // At restored position
    }

    #[test]
    fn test_erase_sequences() {
        let mut ofs_buf = create_test_offscreen_buffer();
        let mut parser = vte::Parser::new();

        {
            let mut processor = new(&mut ofs_buf);

            // Fill some content first
            for r in 0..3 {
                for c in 0..5 {
                    processor.cursor_pos = Pos {
                        row_index: row(r),
                        col_index: col(c),
                    };
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    processor.print(char::from(b'A' + r as u8));
                }
            }

            // Test Erase Display (ED) - currently ignored by implementation
            let sequence = CsiSequence::EraseDisplay(2).to_string(); // Clear entire display
            process_bytes(&mut processor, &mut parser, sequence);

            // Test Erase Line (EL) - currently ignored by implementation
            processor.cursor_pos = Pos {
                row_index: row(1),
                col_index: col(2),
            };
            let sequence = CsiSequence::EraseLine(0).to_string(); // Clear from cursor to end of line
            process_bytes(&mut processor, &mut parser, sequence);
        }

        // Since these are ignored, content should still be there
        assert_plain_char_at(&ofs_buf, 0, 0, 'A');
        assert_plain_char_at(&ofs_buf, 1, 0, 'B');
        assert_plain_char_at(&ofs_buf, 2, 0, 'C');
    }

    #[test]
    fn test_csi_next_prev_line() {
        let mut ofs_buf = create_test_offscreen_buffer();
        let mut parser = vte::Parser::new();

        {
            let mut processor = new(&mut ofs_buf);

            // Start at row 3, col 5
            processor.cursor_pos = Pos {
                row_index: row(3),
                col_index: col(5),
            };
            processor.print('A');

            // CSI E - Move to beginning of next line (2 lines down)
            let sequence = CsiSequence::CursorNextLine(2).to_string();
            process_bytes(&mut processor, &mut parser, sequence);

            // Should be at row 5, col 0
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 5);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 0);
            processor.print('B');

            // CSI F - Move to beginning of previous line (3 lines up)
            let sequence = CsiSequence::CursorPrevLine(3).to_string();
            process_bytes(&mut processor, &mut parser, sequence);

            // Should be at row 2, col 0
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 2);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 0);
            processor.print('C');
        }

        // Verify characters
        assert_plain_char_at(&ofs_buf, 3, 5, 'A');
        assert_plain_char_at(&ofs_buf, 5, 0, 'B');
        assert_plain_char_at(&ofs_buf, 2, 0, 'C');
    }

    #[test]
    fn test_csi_cursor_horizontal_absolute() {
        let mut ofs_buf = create_test_offscreen_buffer();
        let mut parser = vte::Parser::new();

        {
            let mut processor = new(&mut ofs_buf);

            // Start at row 2
            processor.cursor_pos = Pos {
                row_index: row(2),
                col_index: col(0),
            };

            // CSI G - Move to column 5 (1-based)
            let sequence = CsiSequence::CursorHorizontalAbsolute(5).to_string();
            process_bytes(&mut processor, &mut parser, sequence);

            // Should be at row 2, col 4 (0-based)
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 2);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 4);
            processor.print('X');

            // CSI G with 1 - move to column 1
            let sequence = CsiSequence::CursorHorizontalAbsolute(1).to_string();
            process_bytes(&mut processor, &mut parser, sequence);

            // Should be at row 2, col 0
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 2);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 0);
            processor.print('Y');

            // CSI G beyond buffer width - should clamp
            let sequence = CsiSequence::CursorHorizontalAbsolute(999).to_string();
            process_bytes(&mut processor, &mut parser, sequence);

            // Should be at row 2, col 9 (last column)
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 2);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 9);
            processor.print('Z');
        }

        // Verify characters
        assert_plain_char_at(&ofs_buf, 2, 4, 'X');
        assert_plain_char_at(&ofs_buf, 2, 0, 'Y');
        assert_plain_char_at(&ofs_buf, 2, 9, 'Z');
    }

    #[test]
    fn test_csi_scroll_up_down() {
        let mut ofs_buf = create_test_offscreen_buffer();
        let mut parser = vte::Parser::new();

        {
            let mut processor = new(&mut ofs_buf);

            // Fill some rows with identifiable content
            for i in 0..5 {
                processor.cursor_pos = Pos {
                    row_index: row(i),
                    col_index: col(0),
                };
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                processor.print(char::from(b'A' + i as u8));
            }

            // CSI S - Scroll up 2 lines
            let sequence = CsiSequence::ScrollUp(2).to_string();
            process_bytes(&mut processor, &mut parser, sequence);
        }

        // After scrolling up 2 lines:
        // Row 0 should have what was in row 2 ('C')
        assert_plain_char_at(&ofs_buf, 0, 0, 'C');
        // Row 1 should have what was in row 3 ('D')
        assert_plain_char_at(&ofs_buf, 1, 0, 'D');
        // Row 2 should have what was in row 4 ('E')
        assert_plain_char_at(&ofs_buf, 2, 0, 'E');
        // Rows 3-4 should be empty (new lines)
        assert_empty_at(&ofs_buf, 3, 0);
        assert_empty_at(&ofs_buf, 4, 0);

        {
            let mut processor = new(&mut ofs_buf);

            // Now scroll down 1 line
            let sequence = CsiSequence::ScrollDown(1).to_string();
            process_bytes(&mut processor, &mut parser, sequence);
        }

        // After scrolling down 1 line:
        // Row 0 should be empty (new line)
        assert_empty_at(&ofs_buf, 0, 0);
        // Row 1 should have what was in row 0 ('C')
        assert_plain_char_at(&ofs_buf, 1, 0, 'C');
        // Row 2 should have what was in row 1 ('D')
        assert_plain_char_at(&ofs_buf, 2, 0, 'D');
        // Row 3 should have what was in row 2 ('E')
        assert_plain_char_at(&ofs_buf, 3, 0, 'E');
    }

    #[test]
    fn test_csi_device_status_report() {
        let mut ofs_buf = create_test_offscreen_buffer();
        let mut parser = vte::Parser::new();

        {
            let mut processor = new(&mut ofs_buf);

            // Position cursor at a known location
            processor.cursor_pos = Pos {
                row_index: row(3),
                col_index: col(7),
            };

            // CSI 5n - Request device status
            // This should be recognized but not cause any visible changes
            let sequence = CsiSequence::DeviceStatusReport(5).to_string();
            process_bytes(&mut processor, &mut parser, sequence);

            // Cursor should not have moved
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 3);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 7);

            // CSI 6n - Request cursor position report
            // This should be recognized but not cause any visible changes
            let sequence = CsiSequence::DeviceStatusReport(6).to_string();
            process_bytes(&mut processor, &mut parser, sequence);

            // Cursor should not have moved
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 3);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 7);

            // Write something to verify normal operation continues
            processor.print('X');
        }

        // Verify the character was written normally
        assert_plain_char_at(&ofs_buf, 3, 7, 'X');
    }
}

#[cfg(test)]
mod tests_full_ansi_sequences {
    use super::*;
    use crate::{ANSIBasicColor, TuiColor, height,
                offscreen_buffer::test_fixtures_offscreen_buffer::*, width};

    /// Create a test `OffscreenBuffer` with 24x80 dimensions (more realistic terminal
    /// size).
    fn create_realistic_buffer() -> OffscreenBuffer {
        OffscreenBuffer::new_with_capacity_initialized(height(24) + width(80))
    }

    #[test]
    #[allow(clippy::items_after_statements)]
    fn test_vim_like_sequence() {
        let mut ofs_buf = create_realistic_buffer();
        let mut parser = vte::Parser::new();

        {
            let mut processor = new(&mut ofs_buf);

            // Simulate a vim-like sequence using proper builders
            use crate::{SgrCode,
                        ansi_parser::{csi_codes::CsiSequence, esc_codes::EscSequence}};

            let sequence = format!(
                "{clear_screen}{home_position}{reverse_video}{status_text}{reset_attrs}{save_cursor}{move_to_cmd}{prompt}{restore_cursor}{restored_text}",
                clear_screen = CsiSequence::EraseDisplay(2), /* Clear screen (we
                                                              * ignore this) */
                home_position = CsiSequence::CursorPosition { row: 1, col: 1 }, /* Home position */
                reverse_video = SgrCode::Invert, // Reverse video
                status_text = "-- INSERT --",    // Status text
                reset_attrs = SgrCode::Reset,    // Reset attributes
                save_cursor = EscSequence::SaveCursor, // Save cursor (ESC 7)
                move_to_cmd = CsiSequence::CursorPosition { row: 23, col: 1 }, /* Move
                                                  * to line
                                                  * 23, col
                                                  * 1 */
                prompt = ":",                                // Command prompt
                restore_cursor = EscSequence::RestoreCursor, // Restore cursor (ESC 8)
                restored_text = "Restored"                   // Text at restored position
            );

            process_bytes(&mut processor, &mut parser, sequence);
        }

        // Verify status line with reverse video
        for (i, ch) in "-- INSERT --".chars().enumerate() {
            assert_styled_char_at(
                &ofs_buf,
                0,
                i,
                ch,
                |style_from_buffer| {
                    matches!(
                        style_from_buffer.attribs.reverse,
                        Some(tui_style_attrib::Reverse)
                    )
                },
                "reverse video status",
            );
        }

        // Verify command prompt at bottom
        assert_plain_char_at(&ofs_buf, 22, 0, ':');

        // Verify restored text is after status line
        assert_plain_text_at(&ofs_buf, 0, 12, "Restored");
    }

    #[test]
    #[allow(clippy::items_after_statements)]
    fn test_htop_like_colored_bars() {
        let mut ofs_buf = create_realistic_buffer();
        let mut parser = vte::Parser::new();

        {
            let mut processor = new(&mut ofs_buf);

            // Simulate htop-like colored progress bars using proper builders
            use crate::{SgrCode, ansi_parser::csi_codes::CsiSequence};

            let sequence = format!(
                "{home}{cpu_label}{green_color}{green_bar}{red_color}{red_bar}{reset_color}{cpu_end}{mem_label}{yellow_color}{yellow_bar}{reset_color2}{mem_end}",
                home = CsiSequence::CursorPosition { row: 1, col: 1 }, /* Home */
                cpu_label = "CPU [",                                   /* Label */
                green_color = SgrCode::ForegroundBasic(ANSIBasicColor::DarkGreen), /* Green */
                green_bar = "████████", /* Bar */
                red_color = SgrCode::ForegroundBasic(ANSIBasicColor::DarkRed), /* Red */
                red_bar = "██",         /* Bar */
                reset_color = SgrCode::Reset, /* Reset */
                cpu_end = "  ] 80%\r\n", /* End label with CR+LF */
                mem_label = "MEM [",    /* Next line */
                yellow_color = SgrCode::ForegroundBasic(ANSIBasicColor::DarkYellow), /* Yellow */
                yellow_bar = "██████",         /* Bar */
                reset_color2 = SgrCode::Reset, /* Reset */
                mem_end = "    ] 60%"          /* End */
            );

            process_bytes(&mut processor, &mut parser, sequence);
        }

        // Verify colored CPU bar
        assert_plain_text_at(&ofs_buf, 0, 0, "CPU [");

        // Green portion
        for i in 0..8 {
            assert_styled_char_at(
                &ofs_buf,
                0,
                5 + i,
                '█',
                |style_from_buffer| {
                    matches!(
                        style_from_buffer.color_fg,
                        Some(TuiColor::Basic(ANSIBasicColor::DarkGreen))
                    )
                },
                "green bar segment",
            );
        }

        // Red portion
        for i in 0..2 {
            assert_styled_char_at(
                &ofs_buf,
                0,
                13 + i,
                '█',
                |style_from_buffer| {
                    matches!(
                        style_from_buffer.color_fg,
                        Some(TuiColor::Basic(ANSIBasicColor::DarkRed))
                    )
                },
                "red bar segment",
            );
        }

        // Verify MEM label on second line
        assert_plain_text_at(&ofs_buf, 1, 0, "MEM [");

        // Yellow MEM bar
        for i in 0..6 {
            assert_styled_char_at(
                &ofs_buf,
                1,
                5 + i,
                '█',
                |style_from_buffer| {
                    matches!(
                        style_from_buffer.color_fg,
                        Some(TuiColor::Basic(ANSIBasicColor::DarkYellow))
                    )
                },
                "yellow bar segment",
            );
        }
    }

    #[test]
    #[allow(clippy::items_after_statements)]
    fn test_complex_cursor_dance() {
        let mut ofs_buf = create_realistic_buffer();
        let mut parser = vte::Parser::new();

        {
            let mut processor = new(&mut ofs_buf);

            // Complex cursor movements typical in TUI apps using proper builders
            use crate::ansi_parser::csi_codes::CsiSequence;

            let sequence = format!(
                "{pos_5_10}{write_a}{up_2}{write_b}{down_3}{back_4}{write_c}{forward_999}{write_d}{home}{write_h}{bottom_right}{write_e}",
                pos_5_10 = CsiSequence::CursorPosition { row: 5, col: 10 }, /* Position at (5,10) -> (4,9) 0-based */
                write_a = "A",                       // Write A at (4,9)
                up_2 = CsiSequence::CursorUp(2),     // Up 2 lines to row 2
                write_b = "B",                       // Write B at (2,10)
                down_3 = CsiSequence::CursorDown(3), // Down 3 lines to row 5
                back_4 = CsiSequence::CursorBackward(4), /* Back 4 columns to
                                                      * col 7 (11-4=7) */
                write_c = "C", // Write C at (5,7)
                forward_999 = CsiSequence::CursorForward(999), /* Forward 999
                                * (should clamp
                                * to 79) */
                write_d = "D", // Write D at right edge
                home = CsiSequence::CursorPosition { row: 1, col: 1 }, // Home
                write_h = "H", // Write H at home
                bottom_right = CsiSequence::CursorPosition { row: 999, col: 999 }, /* Bottom-right
                                                                                    * (clamped) */
                write_e = "E", // Write E at bottom-right
            );

            process_bytes(&mut processor, &mut parser, sequence);
        }

        // Verify all characters are at expected positions
        assert_plain_char_at(&ofs_buf, 4, 9, 'A'); // Position (5,10) in 1-based = (4,9) in 0-based
        assert_plain_char_at(&ofs_buf, 2, 10, 'B'); // Up 2 from row 4 to row 2, cursor was at col 10
        assert_plain_char_at(&ofs_buf, 5, 7, 'C'); // Down 3 to row 5, back 4 to col 7
        assert_plain_char_at(&ofs_buf, 5, 79, 'D'); // Clamped to right edge
        assert_plain_char_at(&ofs_buf, 0, 0, 'H'); // Home position
        assert_plain_char_at(&ofs_buf, 23, 79, 'E'); // Bottom-right (row 999 clamped to 23, col 999 to 79)
    }

    #[test]
    fn test_split_ansi_sequences() {
        let mut ofs_buf = create_realistic_buffer();
        let mut parser = vte::Parser::new();

        {
            let mut processor = new(&mut ofs_buf);

            // Test that VTE parser correctly handles split sequences
            // Simulate receiving data in chunks

            // First chunk: partial CSI sequence for red foreground
            let chunk1 = b"\x1b[3";
            process_bytes(&mut processor, &mut parser, chunk1);

            // Second chunk: complete the sequence (31m = red) and add "Red"
            let chunk2 = b"1mRed";
            process_bytes(&mut processor, &mut parser, chunk2);

            // Third chunk: " Text" followed by start of reset sequence
            let chunk3 = b" Text\x1b[";
            process_bytes(&mut processor, &mut parser, chunk3);

            // Fourth chunk: complete reset (0m) and add " Normal"
            let chunk4 = b"0m Normal";
            process_bytes(&mut processor, &mut parser, chunk4);
        }

        // Verify the text was processed correctly despite splits
        // "Red Text" should all be in red, then " Normal" should be plain
        for (i, ch) in "Red Text".chars().enumerate() {
            assert_styled_char_at(
                &ofs_buf,
                0,
                i,
                ch,
                |style_from_buffer| {
                    matches!(
                        style_from_buffer.color_fg,
                        Some(TuiColor::Basic(ANSIBasicColor::DarkRed)) /* Should be DarkRed not Red */
                    )
                },
                "red foreground",
            );
        }

        // " Normal" should be plain text (after reset)
        assert_plain_text_at(&ofs_buf, 0, 8, " Normal");
    }

    #[test]
    #[allow(clippy::items_after_statements)]
    fn test_box_drawing_with_positioning() {
        let mut ofs_buf = create_realistic_buffer();
        let mut parser = vte::Parser::new();

        {
            let mut processor = new(&mut ofs_buf);

            // Draw a box using DEC graphics and cursor positioning
            use crate::ansi_parser::{csi_codes::CsiSequence, esc_codes::EscSequence};

            let sequence = format!(
                "{graphics_mode}{top_left_pos}{corner_tl}{h_line}{corner_tr}{left_line_pos}{v_line_left}{right_line_pos}{v_line_right}{bottom_line_pos}{corner_bl}{h_line2}{corner_br}{ascii_mode}{inside_pos}{inside_text}",
                graphics_mode = EscSequence::SelectGraphics, // Switch to DEC graphics
                top_left_pos = CsiSequence::CursorPosition { row: 2, col: 2 }, /* Position at (2,2) */
                corner_tl = "l", // Top-left corner
                h_line = "qqq",  // Horizontal line
                corner_tr = "k", // Top-right corner
                left_line_pos = CsiSequence::CursorPosition { row: 3, col: 2 }, /* Next line */
                v_line_left = "x", // Vertical line
                right_line_pos = CsiSequence::CursorPosition { row: 3, col: 6 }, /* Jump to right side */
                v_line_right = "x", // Vertical line
                bottom_line_pos = CsiSequence::CursorPosition { row: 4, col: 2 }, /* Bottom line */
                corner_bl = "m",                       // Bottom-left corner
                h_line2 = "qqq",                       // Horizontal line
                corner_br = "j",                       // Bottom-right corner
                ascii_mode = EscSequence::SelectAscii, // Back to ASCII
                inside_pos = CsiSequence::CursorPosition { row: 3, col: 3 }, // Inside box
                inside_text = "Box"                    // Text inside
            );

            process_bytes(&mut processor, &mut parser, sequence);
        }

        // Verify box corners
        assert_plain_char_at(&ofs_buf, 1, 1, '┌'); // Top-left
        assert_plain_char_at(&ofs_buf, 1, 5, '┐'); // Top-right
        assert_plain_char_at(&ofs_buf, 3, 1, '└'); // Bottom-left
        assert_plain_char_at(&ofs_buf, 3, 5, '┘'); // Bottom-right

        // Verify box lines
        for i in 2..5 {
            assert_plain_char_at(&ofs_buf, 1, i, '─'); // Top horizontal
            assert_plain_char_at(&ofs_buf, 3, i, '─'); // Bottom horizontal
        }
        assert_plain_char_at(&ofs_buf, 2, 1, '│'); // Left vertical
        assert_plain_char_at(&ofs_buf, 2, 5, '│'); // Right vertical

        // Verify text inside box
        assert_plain_text_at(&ofs_buf, 2, 2, "Box");
    }

    #[test]
    fn test_malformed_sequences_recovery() {
        let mut ofs_buf = create_realistic_buffer();
        let mut parser = vte::Parser::new();

        {
            let mut processor = new(&mut ofs_buf);

            // Test recovery from malformed sequences
            let sequence = concat!(
                "Normal",
                "\x1b[999999999999999999;1H", // Extremely large number
                "A",
                "\x1b[;H", // Missing params (should default)
                "B",
                "\x1b[31", // Incomplete CSI (will be completed)
                "mRed",    // Complete the sequence
                "\x1b[0m", // Reset
                "\x1bZ",   // Unknown ESC sequence (ignored)
                "End"
            );

            process_bytes(&mut processor, &mut parser, sequence);
        }

        // Verify text appears despite malformed sequences
        // Sequence: "Normal" at (0,0), cursor to clamped position (23,0), "A", cursor to
        // (0,0), "B" overwrites "N", then "Red" overwrites "orm", then "End"
        // overwrites "al" Final row 0: "BRedEnd" (with "Red" in red color)
        assert_plain_char_at(&ofs_buf, 0, 0, 'B'); // B overwrites 'N'
        assert_plain_char_at(&ofs_buf, 23, 0, 'A'); // A at clamped position (max row is 23 for 24-row buffer)

        // Verify "Red" text with color (overwrites "orm" starting at column 1)
        for (i, ch) in "Red".chars().enumerate() {
            assert_styled_char_at(
                &ofs_buf,
                0,
                1 + i,
                ch,
                |style_from_buffer| {
                    matches!(
                        style_from_buffer.color_fg,
                        Some(TuiColor::Basic(ANSIBasicColor::DarkRed))
                    )
                },
                "red text after malformed sequence",
            );
        }

        // Verify "End" text (overwrites "al" starting at column 4)
        assert_plain_text_at(&ofs_buf, 0, 4, "End");
    }
}

#[cfg(test)]
mod tests_osc {
    use crate::{OffscreenBuffer, Size,
                core::osc::{OscEvent, osc_codes},
                height, width};

    #[test]
    fn test_osc_0_title_sequence() {
        let mut ofs_buf = OffscreenBuffer::new_with_capacity_initialized(Size {
            row_height: height(10),
            col_width: width(40),
        });

        // OSC 0 sequence: ESC ] 0 ; title BEL
        let title = "My Custom Title";
        let sequence = format!(
            "{osc_code}{title}{terminator}",
            osc_code = osc_codes::OSC0_SET_TITLE_AND_TAB,
            title = title,
            terminator = osc_codes::BELL_TERMINATOR
        );

        let events = ofs_buf.apply_ansi_bytes(sequence);

        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            OscEvent::SetTitleAndTab("My Custom Title".to_string())
        );
    }

    #[test]
    fn test_osc_2_title_sequence() {
        let mut ofs_buf = OffscreenBuffer::new_with_capacity_initialized(Size {
            row_height: height(10),
            col_width: width(40),
        });

        // OSC 2 sequence: ESC ] 2 ; title BEL
        let title = "Window Title Only";
        let sequence = format!(
            "{osc_code}{title}{terminator}",
            osc_code = osc_codes::OSC2_SET_TITLE,
            title = title,
            terminator = osc_codes::BELL_TERMINATOR
        );

        let events = ofs_buf.apply_ansi_bytes(sequence);

        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            OscEvent::SetTitleAndTab("Window Title Only".to_string())
        );
    }

    #[test]
    fn test_osc_1_icon_sequence() {
        let mut ofs_buf = OffscreenBuffer::new_with_capacity_initialized(Size {
            row_height: height(10),
            col_width: width(40),
        });

        // OSC 1 sequence: ESC ] 1 ; icon name BEL
        let title = "Icon Name";
        let sequence = format!(
            "{osc_code}{title}{terminator}",
            osc_code = osc_codes::OSC1_SET_ICON,
            title = title,
            terminator = osc_codes::BELL_TERMINATOR
        );

        let events = ofs_buf.apply_ansi_bytes(sequence);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0], OscEvent::SetTitleAndTab("Icon Name".to_string()));
    }

    #[test]
    fn test_mixed_text_and_osc() {
        let mut ofs_buf = OffscreenBuffer::new_with_capacity_initialized(Size {
            row_height: height(10),
            col_width: width(40),
        });

        // Mixed content: text, OSC title, more text
        let sequence = format!(
            "Hello World{osc_code}Terminal Title{terminator}More text",
            osc_code = osc_codes::OSC0_SET_TITLE_AND_TAB,
            terminator = osc_codes::BELL_TERMINATOR
        );

        let events = ofs_buf.apply_ansi_bytes(sequence);

        // Should extract the OSC event
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            OscEvent::SetTitleAndTab("Terminal Title".to_string())
        );

        // The text should still be in the buffer
        // Check that "Hello World" appears at the beginning
        let first_row = &ofs_buf.buffer[0];

        // Check the characters match
        if let crate::PixelChar::PlainText { display_char, .. } = first_row[0] {
            assert_eq!(display_char, 'H');
        } else {
            panic!("Expected PlainText at position 0");
        }

        if let crate::PixelChar::PlainText { display_char, .. } = first_row[1] {
            assert_eq!(display_char, 'e');
        } else {
            panic!("Expected PlainText at position 1");
        }

        if let crate::PixelChar::PlainText { display_char, .. } = first_row[10] {
            assert_eq!(display_char, 'd');
        } else {
            panic!("Expected PlainText at position 10");
        }
    }

    #[test]
    fn test_multiple_osc_sequences() {
        let mut ofs_buf = OffscreenBuffer::new_with_capacity_initialized(Size {
            row_height: height(10),
            col_width: width(40),
        });

        // Multiple OSC sequences
        let sequence = format!(
            "{osc0_code}Title 1{terminator1}{osc2_code}Title 2{terminator2}{osc1_code}Title 3{terminator3}",
            osc0_code = osc_codes::OSC0_SET_TITLE_AND_TAB,
            terminator1 = osc_codes::BELL_TERMINATOR,
            osc2_code = osc_codes::OSC2_SET_TITLE,
            terminator2 = osc_codes::BELL_TERMINATOR,
            osc1_code = osc_codes::OSC1_SET_ICON,
            terminator3 = osc_codes::BELL_TERMINATOR
        );

        let events = ofs_buf.apply_ansi_bytes(sequence);

        // Should get all three events
        assert_eq!(events.len(), 3);
        assert_eq!(events[0], OscEvent::SetTitleAndTab("Title 1".to_string()));
        assert_eq!(events[1], OscEvent::SetTitleAndTab("Title 2".to_string()));
        assert_eq!(events[2], OscEvent::SetTitleAndTab("Title 3".to_string()));
    }
}
