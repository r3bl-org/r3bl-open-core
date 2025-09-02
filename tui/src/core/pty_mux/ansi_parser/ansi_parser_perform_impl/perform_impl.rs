// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Internal implementation for ANSI/VT sequence processing.
//!
//! This parser is based on the `vte` crate's `Perform` trait, and is [VT100
//! specifications](https://vt100.net/docs/vt100-ug/chapter3.html)
//! compliant. It provides support to parse ANSI escape sequences and update
//! an [`crate::OffscreenBuffer`] accordingly.

use vte::{Params, Perform};

use super::super::{ansi_parser_public_api::AnsiToBufferProcessor,
                   csi_codes::{self},
                   esc_codes};
use crate::{BoundsCheck,
            BoundsStatus::{Overflowed, Within},
            CharacterSet, PixelChar, col, row,
            core::osc::{OscEvent, osc_codes}};

// Import the operation modules.
use super::{char_translation, cursor_ops, device_ops, margin_ops, mode_ops, 
            scroll_ops, sgr_ops, terminal_ops};

/// Internal methods for `AnsiToBufferProcessor` to implement [`Perform`] trait.
impl Perform for AnsiToBufferProcessor<'_> {
    /// Handle printable characters.
    fn print(&mut self, ch: char) {
        // Apply character set translation if in graphics mode.
        let display_char = match self.ofs_buf.ansi_parser_support.character_set {
            CharacterSet::DECGraphics => char_translation::translate_dec_graphics(ch),
            CharacterSet::Ascii => ch,
        };

        let row_max = self.ofs_buf.window_size.row_height;
        let col_max = self.ofs_buf.window_size.col_width;
        let current_row = self.ofs_buf.my_pos.row_index;
        let current_col = self.ofs_buf.my_pos.col_index;

        // Only write if within bounds.
        if current_row.check_overflows(row_max) == Within
            && current_col.check_overflows(col_max) == Within
        {
            // Write character to buffer using public fields.
            self.ofs_buf.buffer[current_row.as_usize()][current_col.as_usize()] =
                PixelChar::PlainText {
                    display_char, // Use the translated character
                    maybe_style: self.ofs_buf.ansi_parser_support.current_style,
                };

            // Move cursor forward.
            let new_col = current_col + col(1);

            // Handle line wrap based on DECAWM (Auto Wrap Mode).
            if new_col.check_overflows(col_max) == Overflowed {
                if self.ofs_buf.ansi_parser_support.auto_wrap_mode {
                    // DECAWM enabled: wrap to next line (default behavior)
                    self.ofs_buf.my_pos.col_index = col(0);
                    let next_row = current_row + row(1);
                    if next_row.check_overflows(row_max) == Within {
                        self.ofs_buf.my_pos.row_index = next_row;
                    }
                } else {
                    // DECAWM disabled: stay at right margin (clamp cursor position)
                    self.ofs_buf.my_pos.col_index = col_max.convert_to_col_index();
                }
            } else {
                self.ofs_buf.my_pos.col_index = new_col;
            }
        }
    }

    /// Handle control characters (C0 set): backspace, tab, LF, CR
    fn execute(&mut self, byte: u8) {
        match byte {
            // Backspace
            esc_codes::BACKSPACE => {
                let current_col = self.ofs_buf.my_pos.col_index.as_usize();
                if current_col > 0 {
                    self.ofs_buf.my_pos.col_index = col(current_col - 1);
                }
            }
            // Tab - move to next tab stop boundary
            esc_codes::TAB => {
                let current_col = self.ofs_buf.my_pos.col_index.as_usize();
                let current_tab_zone = current_col / esc_codes::TAB_STOP_WIDTH;
                let next_tab_zone = current_tab_zone + 1;
                let next_tab_col = next_tab_zone * esc_codes::TAB_STOP_WIDTH;
                let max_col = self.ofs_buf.window_size.col_width;

                // Clamp to max valid column index if it would overflow
                self.ofs_buf.my_pos.col_index = col(usize::min(
                    next_tab_col,
                    max_col.convert_to_col_index().as_usize(),
                ));
            }
            // Line feed (newline)
            esc_codes::LINE_FEED => {
                let max_row = self.ofs_buf.window_size.row_height;
                let next_row = self.ofs_buf.my_pos.row_index + row(1);
                if next_row.check_overflows(max_row) == Within {
                    self.ofs_buf.my_pos.row_index = next_row;
                }
            }
            // Carriage return
            esc_codes::CARRIAGE_RETURN => {
                self.ofs_buf.my_pos.col_index = col(0);
            }
            _ => {}
        }
    }

    /// Handle CSI (Control Sequence Introducer) sequences.
    ///
    /// This method processes ANSI escape sequences that follow the pattern `ESC[...char`
    /// where `char` is the final dispatch character that determines the operation.
    ///
    /// ## Parameter Handling
    ///
    /// All cursor movement and scroll operations follow VT100 specification for parameter
    /// handling:
    /// - **Missing parameters** (None) default to 1
    /// - **Zero parameters** (Some(0)) are treated as 1
    /// - This ensures compatibility with real VT100 terminals and modern terminal
    ///   emulators
    ///
    /// ### Examples
    /// - `ESC[A` (no param) → move up 1 line
    /// - `ESC[0A` (zero param) → move up 1 line (0 treated as 1)
    /// - `ESC[5A` (explicit param) → move up 5 lines
    /// - `ESC[S` (no param) → scroll up 1 line
    /// - `ESC[0S` (zero param) → scroll up 1 line (0 treated as 1)
    ///
    /// ## Supported Operations
    /// - Cursor movements: CUU, CUD, CUF, CUB, CNL, CPL, CHA, CUP
    /// - Scrolling: SU (Scroll Up), SD (Scroll Down)
    /// - Display control: ED, EL
    /// - Cursor save/restore: SCP, RCP
    /// - Margins: DECSTBM
    /// - Modes: SM, RM (including private modes with ? prefix)
    /// - Graphics: SGR (Select Graphic Rendition)
    fn csi_dispatch(
        &mut self,
        params: &Params,
        intermediates: &[u8],
        _ignore: bool,
        dispatch_char: char,
    ) {
        match dispatch_char {
            // Cursor movement operations
            csi_codes::CUU_CURSOR_UP => cursor_ops::cursor_up(self, params),
            csi_codes::CUD_CURSOR_DOWN => cursor_ops::cursor_down(self, params),
            csi_codes::CUF_CURSOR_FORWARD => cursor_ops::cursor_forward(self, params),
            csi_codes::CUB_CURSOR_BACKWARD => cursor_ops::cursor_backward(self, params),
            csi_codes::CUP_CURSOR_POSITION | csi_codes::HVP_CURSOR_POSITION => {
                cursor_ops::cursor_position(self, params);
            }
            csi_codes::CNL_CURSOR_NEXT_LINE => cursor_ops::cursor_next_line(self, params),
            csi_codes::CPL_CURSOR_PREV_LINE => cursor_ops::cursor_prev_line(self, params),
            csi_codes::CHA_CURSOR_COLUMN => cursor_ops::cursor_horizontal_absolute(self, params),
            csi_codes::SCP_SAVE_CURSOR => cursor_ops::save_cursor_position(self),
            csi_codes::RCP_RESTORE_CURSOR => cursor_ops::restore_cursor_position(self),

            // Scrolling operations
            csi_codes::SU_SCROLL_UP => scroll_ops::scroll_up(self, params),
            csi_codes::SD_SCROLL_DOWN => scroll_ops::scroll_down(self, params),

            // Margin operations
            csi_codes::DECSTBM_SET_MARGINS => margin_ops::set_margins(self, params),

            // Device status operations
            csi_codes::DSR_DEVICE_STATUS => device_ops::device_status_report(self, params),

            // Mode operations
            csi_codes::SM_SET_MODE => mode_ops::set_mode(self, params, intermediates),
            csi_codes::RM_RESET_MODE => mode_ops::reset_mode(self, params, intermediates),

            // Graphics operations
            csi_codes::SGR_SET_GRAPHICS => sgr_ops::sgr(self, params),

            // Display control operations (ignored)
            _ => {
                // Clear screen/line - ignore, TUI apps will repaint themselves
            }
        }
    }

    /// Handle OSC (Operating System Command) sequences.
    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
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
                        self.ofs_buf
                            .ansi_parser_support
                            .pending_osc_events
                            .push(OscEvent::SetTitleAndTab(title.to_string()));
                    }
                }
                // OSC 8: Hyperlink (format: OSC 8 ; params ; URI)
                osc_codes::OSC_CODE_HYPERLINK if params.len() > 2 => {
                    if let Ok(uri) = std::str::from_utf8(params[2]) {
                        // For now, just store the URI - display text would come from
                        // print chars
                        self.ofs_buf.ansi_parser_support.pending_osc_events.push(
                            OscEvent::Hyperlink {
                                uri: uri.to_string(),
                                text: String::new(), /* Text is handled separately via
                                                      * print() */
                            },
                        );
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
    /// - **ESC 7 (DECSC)**: Save cursor position to
    ///   `ofs_buf.my_pos_for_esc_save_and_restore`
    /// - **ESC 8 (DECRC)**: Restore cursor from `ofs_buf.my_pos_for_esc_save_and_restore`
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
    ///   → AnsiToBufferProcessor::new() with ofs_buf.my_pos = (5,10)
    ///   → esc_dispatch() handles ESC 7
    ///   → Saves ofs_buf.ansi_parser_support.cursor_pos_for_esc_save_and_restore = Some((5,10))
    ///
    /// Session 2: vim moves cursor to (20,30), then sends ESC 8
    ///   → AnsiToBufferProcessor::new() with ofs_buf.my_pos = (20,30)
    ///   → esc_dispatch() handles ESC 8
    ///   → Restores ofs_buf.my_pos = cursor_pos_for_esc_save_and_restore.unwrap_or() // (5,10) ✓
    /// ```
    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        match byte {
            esc_codes::DECSC_SAVE_CURSOR => {
                // DECSC - Save current cursor position
                // The cursor position is saved to persistent buffer state so it
                // survives across multiple AnsiToBufferProcessor instances
                self.ofs_buf
                    .ansi_parser_support
                    .cursor_pos_for_esc_save_and_restore = Some(self.ofs_buf.my_pos);
                tracing::trace!(
                    "ESC 7 (DECSC): Saved cursor position {:?}",
                    self.ofs_buf.my_pos
                );
            }
            esc_codes::DECRC_RESTORE_CURSOR => {
                // DECRC - Restore saved cursor position
                // Retrieves the previously saved position from buffer's persistent state
                if let Some(saved_pos) = self
                    .ofs_buf
                    .ansi_parser_support
                    .cursor_pos_for_esc_save_and_restore
                {
                    self.ofs_buf.my_pos = saved_pos;
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
                        self.ofs_buf.ansi_parser_support.character_set =
                            CharacterSet::Ascii;
                        tracing::trace!("ESC ( B: Selected ASCII character set");
                    }
                    esc_codes::CHARSET_DEC_GRAPHICS => {
                        // Select DEC Special Graphics character set
                        // This enables box-drawing characters
                        self.ofs_buf.ansi_parser_support.character_set =
                            CharacterSet::DECGraphics;
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

