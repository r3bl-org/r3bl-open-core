// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Internal implementation for ANSI/VT sequence processing.
//!
//! This parser is based on the `vte` crate's `Perform` trait, and is [VT100
//! specifications](https://vt100.net/docs/vt100-ug/chapter3.html)
//! compliant. It provides support to parse ANSI escape sequences and update
//! an [`crate::OffscreenBuffer`] accordingly.

use vte::{Params, Perform};

use super::{ansi_parser_public_api::AnsiToBufferProcessor,
            ansi_to_tui_color::ansi_to_tui_color,
            csi_codes::{self, DeviceStatusReportType, PrivateModeType},
            esc_codes,
            term_units::term_row};
use crate::{BoundsCheck,
            BoundsStatus::{Overflowed, Within},
            CharacterSet, PixelChar, Pos, TuiStyle, col,
            core::osc::{OscEvent, osc_codes},
            row, tui_style_attrib};

/// Internal methods for `AnsiToBufferProcessor` to implement [`Perform`] trait.
impl Perform for AnsiToBufferProcessor<'_> {
    /// Handle printable characters.
    fn print(&mut self, ch: char) {
        // Apply character set translation if in graphics mode
        let display_char = match self.ofs_buf.ansi_parser_support.character_set {
            CharacterSet::DECGraphics => char_translation::translate_dec_graphics(ch),
            CharacterSet::Ascii => ch,
        };

        let row_max = self.ofs_buf.window_size.row_height;
        let col_max = self.ofs_buf.window_size.col_width;
        let current_row = self.ofs_buf.my_pos.row_index;
        let current_col = self.ofs_buf.my_pos.col_index;

        // Only write if within bounds
        if current_row.check_overflows(row_max) == Within
            && current_col.check_overflows(col_max) == Within
        {
            // Write character to buffer using public fields
            self.ofs_buf.buffer[current_row.as_usize()][current_col.as_usize()] =
                PixelChar::PlainText {
                    display_char, // Use the translated character
                    maybe_style: self.ofs_buf.ansi_parser_support.current_style,
                };

            // Move cursor forward
            let new_col = current_col + col(1);

            // Handle line wrap based on DECAWM (Auto Wrap Mode)
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
    #[allow(clippy::too_many_lines)]
    fn csi_dispatch(
        &mut self,
        params: &Params,
        intermediates: &[u8],
        _ignore: bool,
        dispatch_char: char,
    ) {
        #[allow(clippy::match_same_arms)]
        match dispatch_char {
            csi_codes::CUU_CURSOR_UP => {
                let n = params
                    .iter()
                    .next()
                    .and_then(|p| p.first())
                    .copied()
                    .map_or(
                        /* None -> 1 */ 1,
                        /* Some(0) -> 1 */ |v| u16::max(v, 1),
                    );
                cursor_ops::cursor_up(self, n);
            }
            csi_codes::CUD_CURSOR_DOWN => {
                let n = params
                    .iter()
                    .next()
                    .and_then(|p| p.first())
                    .copied()
                    .map_or(
                        /* None -> 1 */ 1,
                        /* Some(0) -> 1 */ |v| u16::max(v, 1),
                    );
                cursor_ops::cursor_down(self, n);
            }
            csi_codes::CUF_CURSOR_FORWARD => {
                let n = params
                    .iter()
                    .next()
                    .and_then(|p| p.first())
                    .copied()
                    .map_or(
                        /* None -> 1 */ 1,
                        /* Some(0) -> 1 */ |v| u16::max(v, 1),
                    );
                cursor_ops::cursor_forward(self, n);
            }
            csi_codes::CUB_CURSOR_BACKWARD => {
                let n = params
                    .iter()
                    .next()
                    .and_then(|p| p.first())
                    .copied()
                    .map_or(
                        /* None -> 1 */ 1,
                        /* Some(0) -> 1 */ |v| u16::max(v, 1),
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
                self.ofs_buf
                    .ansi_parser_support
                    .cursor_pos_for_esc_save_and_restore = Some(self.ofs_buf.my_pos);
                tracing::trace!(
                    "CSI s (SCP): Saved cursor position {:?}",
                    self.ofs_buf.my_pos
                );
            }
            csi_codes::RCP_RESTORE_CURSOR => {
                // CSI u - Restore saved cursor position
                // Alternative to ESC 8 (DECRC)
                if let Some(saved_pos) = self
                    .ofs_buf
                    .ansi_parser_support
                    .cursor_pos_for_esc_save_and_restore
                {
                    self.ofs_buf.my_pos = saved_pos;
                    tracing::trace!(
                        "CSI u (RCP): Restored cursor position to {:?}",
                        saved_pos
                    );
                }
            }
            csi_codes::CNL_CURSOR_NEXT_LINE => {
                // CSI E - Cursor Next Line
                // Move cursor to beginning of line n lines down
                let n = params
                    .iter()
                    .next()
                    .and_then(|p| p.first())
                    .copied()
                    .map_or(
                        /* None -> 1 */ 1,
                        /* Some(0) -> 1 */ |v| u16::max(v, 1),
                    );
                cursor_ops::cursor_down(self, n);
                self.ofs_buf.my_pos.col_index = col(0);
                tracing::trace!("CSI E (CNL): Moved to next line {}", n);
            }
            csi_codes::CPL_CURSOR_PREV_LINE => {
                // CSI F - Cursor Previous Line
                // Move cursor to beginning of line n lines up
                let n = params
                    .iter()
                    .next()
                    .and_then(|p| p.first())
                    .copied()
                    .map_or(
                        /* None -> 1 */ 1,
                        /* Some(0) -> 1 */ |v| u16::max(v, 1),
                    );
                cursor_ops::cursor_up(self, n);
                self.ofs_buf.my_pos.col_index = col(0);
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
                    .map_or(
                        /* None -> 1 */ 1,
                        /* Some(0) -> 1 */ |v| u16::max(v, 1),
                    );
                // Convert from 1-based to 0-based, clamp to buffer width
                let target_col = n.saturating_sub(1) as usize;
                let max_col = self.ofs_buf.window_size.col_width.as_usize();
                self.ofs_buf.my_pos.col_index =
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
                    .map_or(
                        /* None -> 1 */ 1,
                        /* Some(0) -> 1 */ |v| u16::max(v, 1),
                    );
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
                    .map_or(
                        /* None -> 1 */ 1,
                        /* Some(0) -> 1 */ |v| u16::max(v, 1),
                    );
                for _ in 0..n {
                    scroll_ops::scroll_buffer_down(self);
                }
                tracing::trace!("CSI T (SD): Scrolled down {} lines", n);
            }
            csi_codes::DECSTBM_SET_MARGINS => {
                // CSI r - Set Top and Bottom Margins (DECSTBM)
                // ESC [ top ; bottom r
                let maybe_top = params
                    .iter()
                    .next()
                    .and_then(|params| params.first())
                    .copied();
                let maybe_bottom = params
                    .iter()
                    .nth(1)
                    .and_then(|params| params.first())
                    .copied();

                // Store terminal's 1-based coordinates (will be converted to 0-based when
                // used)
                let buffer_height: u16 = self.ofs_buf.window_size.row_height.into();

                match (maybe_top, maybe_bottom) {
                    // ESC [ r parsed as (Some(0), None) - Reset to full screen
                    (Some(0), None) => {
                        self.ofs_buf.ansi_parser_support.scroll_region_top = None;
                        self.ofs_buf.ansi_parser_support.scroll_region_bottom = None;
                        tracing::trace!(
                            "CSI r (DECSTBM): Reset scroll region to full screen (no params)"
                        );
                    }
                    // ESC [ 0 ; 0 r or other reset cases
                    (Some(0), Some(0)) | (None, None) => {
                        self.ofs_buf.ansi_parser_support.scroll_region_top = None;
                        self.ofs_buf.ansi_parser_support.scroll_region_bottom = None;
                        tracing::trace!(
                            "CSI r (DECSTBM): Reset scroll region to full screen"
                        );
                    }
                    // Check if we have no parameters (empty params) - also reset
                    _ if params.is_empty() => {
                        self.ofs_buf.ansi_parser_support.scroll_region_top = None;
                        self.ofs_buf.ansi_parser_support.scroll_region_bottom = None;
                        tracing::trace!(
                            "CSI r (DECSTBM): Reset scroll region to full screen (empty params)"
                        );
                    }
                    (top_param, bottom_param) => {
                        // Set scrolling region with bounds checking
                        let top_row = top_param.map_or(
                            /* None -> 1 */ 1,
                            /* Some(v) -> max(v,1) */ |v| v.max(1),
                        );
                        let bottom_row = bottom_param.map_or(
                            /* None -> buffer_height */ buffer_height,
                            /* Some(v) -> min(v,buffer_height) */
                            |v| v.min(buffer_height),
                        );

                        if top_row < bottom_row && bottom_row <= buffer_height {
                            self.ofs_buf.ansi_parser_support.scroll_region_top =
                                Some(term_row(top_row));
                            self.ofs_buf.ansi_parser_support.scroll_region_bottom =
                                Some(term_row(bottom_row));
                            tracing::trace!(
                                "CSI r (DECSTBM): Set scroll region from row {} to row {}",
                                top_row,
                                bottom_row
                            );
                        } else {
                            tracing::warn!(
                                "CSI r (DECSTBM): Invalid margins top={}, bottom={}, buffer_height={}",
                                top_row,
                                bottom_row,
                                buffer_height
                            );
                        }
                    }
                }
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
                let dsr_type = DeviceStatusReportType::from(n);
                match dsr_type {
                    DeviceStatusReportType::RequestStatus => {
                        // Status report request - should respond with ESC[0n (OK)
                        tracing::debug!(
                            "CSI 5n (DSR): Status report requested (response needed but not implemented)"
                        );
                    }
                    DeviceStatusReportType::RequestCursorPosition => {
                        // Cursor position report - should respond with ESC[row;colR
                        tracing::debug!(
                            "CSI 6n (DSR): Cursor position report requested at {:?} (response needed but not implemented)",
                            self.ofs_buf.my_pos
                        );
                    }
                    DeviceStatusReportType::Other(n) => {
                        tracing::debug!("CSI {}n (DSR): Unknown device status report", n);
                    }
                }
            }
            csi_codes::SM_SET_MODE => {
                // CSI h - Set Mode
                let is_private_mode = intermediates.contains(&b'?');
                if is_private_mode {
                    let mode_num = params
                        .iter()
                        .next()
                        .and_then(|p| p.first())
                        .copied()
                        .unwrap_or(0);
                    let mode = PrivateModeType::from(mode_num);
                    match mode {
                        PrivateModeType::AutoWrap => {
                            self.ofs_buf.ansi_parser_support.auto_wrap_mode = true;
                            tracing::trace!("ESC[?7h: Enabled auto-wrap mode (DECAWM)");
                        }
                        _ => tracing::debug!(
                            "CSI ?{}h: Unhandled private mode",
                            mode.as_u16()
                        ),
                    }
                } else {
                    tracing::debug!("CSI h: Standard mode setting not implemented");
                }
            }
            csi_codes::RM_RESET_MODE => {
                // CSI l - Reset Mode
                let is_private_mode = intermediates.contains(&b'?');
                if is_private_mode {
                    let mode_num = params
                        .iter()
                        .next()
                        .and_then(|p| p.first())
                        .copied()
                        .unwrap_or(0);
                    let mode = PrivateModeType::from(mode_num);
                    match mode {
                        PrivateModeType::AutoWrap => {
                            self.ofs_buf.ansi_parser_support.auto_wrap_mode = false;
                            tracing::trace!("ESC[?7l: Disabled auto-wrap mode (DECAWM)");
                        }
                        _ => tracing::debug!(
                            "CSI ?{}l: Unhandled private mode",
                            mode.as_u16()
                        ),
                    }
                } else {
                    tracing::debug!("CSI l: Standard mode reset not implemented");
                }
            }
            _ => {} /* Ignore other CSI sequences */
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

/// Cursor movement operations.
pub mod cursor_ops {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Move cursor up by n lines.
    /// Respects DECSTBM scroll region margins.
    pub fn cursor_up(processor: &mut AnsiToBufferProcessor, n: u16) {
        let n = n.max(1) as usize;
        let current_row = processor.ofs_buf.my_pos.row_index.as_usize();

        // Get scroll region boundaries (1-based to 0-based conversion)
        let scroll_top = processor
            .ofs_buf
            .ansi_parser_support
            .scroll_region_top
            .and_then(|t| t.to_zero_based()) // Convert 1-based to 0-based
            .map(|row| row.as_usize())
            .unwrap_or(0);

        // Clamp cursor movement to scroll region top
        let new_row = current_row.saturating_sub(n).max(scroll_top);
        processor.ofs_buf.my_pos.row_index = row(new_row);
    }

    /// Move cursor down by n lines.
    /// Respects DECSTBM scroll region margins.
    pub fn cursor_down(processor: &mut AnsiToBufferProcessor, n: u16) {
        let n = n.max(1) as usize;
        let current_row = processor.ofs_buf.my_pos.row_index.as_usize();
        let max_row = processor.ofs_buf.window_size.row_height.as_usize();

        // Get scroll region boundaries (1-based to 0-based conversion)
        let scroll_bottom = processor
            .ofs_buf
            .ansi_parser_support
            .scroll_region_bottom
            .and_then(|b| b.to_zero_based()) // Convert 1-based to 0-based
            .map(|row| row.as_usize())
            .unwrap_or(max_row.saturating_sub(1));

        // Clamp cursor movement to scroll region bottom
        let new_row = (current_row + n).min(scroll_bottom);
        processor.ofs_buf.my_pos.row_index = row(new_row);
    }

    /// Move cursor forward by n columns.
    pub fn cursor_forward(processor: &mut AnsiToBufferProcessor, n: u16) {
        let n = n.max(1) as usize;
        let max_col = processor.ofs_buf.window_size.col_width;
        let new_col = processor.ofs_buf.my_pos.col_index + col(n);
        // Clamp to max_col-1 if it would overflow
        processor.ofs_buf.my_pos.col_index =
            if new_col.check_overflows(max_col) == Overflowed {
                max_col.convert_to_col_index()
            } else {
                new_col
            };
    }

    /// Move cursor backward by n columns.
    pub fn cursor_backward(processor: &mut AnsiToBufferProcessor, n: u16) {
        let n = n.max(1) as usize;
        let current_col = processor.ofs_buf.my_pos.col_index.as_usize();
        processor.ofs_buf.my_pos.col_index = col(current_col.saturating_sub(n));
    }

    /// Set cursor position (1-based coordinates from ANSI, converted to 0-based).
    /// Respects DECSTBM scroll region margins.
    pub fn cursor_position(processor: &mut AnsiToBufferProcessor, params: &Params) {
        let row_param = params
            .iter()
            .next()
            .and_then(|p| p.first())
            .copied()
            .map_or(1, |v| v.max(1))
            .saturating_sub(1) as usize;
        let col_param = params
            .iter()
            .nth(1)
            .and_then(|p| p.first())
            .copied()
            .map_or(1, |v| v.max(1))
            .saturating_sub(1) as usize;
        let max_row = processor.ofs_buf.window_size.row_height.as_usize();
        let max_col = processor.ofs_buf.window_size.col_width.as_usize();

        // Get scroll region boundaries (1-based to 0-based conversion)
        let scroll_top = processor
            .ofs_buf
            .ansi_parser_support
            .scroll_region_top
            .and_then(|t| t.to_zero_based()) // Convert 1-based to 0-based
            .map(|row| row.as_usize())
            .unwrap_or(0);
        let scroll_bottom = processor
            .ofs_buf
            .ansi_parser_support
            .scroll_region_bottom
            .and_then(|b| b.to_zero_based()) // Convert 1-based to 0-based
            .map(|row| row.as_usize())
            .unwrap_or(max_row.saturating_sub(1));

        // Clamp row to scroll region bounds and column to buffer bounds
        let new_row = row_param.max(scroll_top).min(scroll_bottom);
        let new_col = col_param.min(max_col.saturating_sub(1));

        processor.ofs_buf.my_pos = Pos {
            col_index: col(new_col),
            row_index: row(new_row),
        };
    }
}

/// Scrolling operations.
mod scroll_ops {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Move cursor down one line, scrolling the buffer if at bottom.
    /// Implements the ESC D (IND) escape sequence.
    /// Respects DECSTBM scroll region margins.
    pub fn index_down(processor: &mut AnsiToBufferProcessor) {
        let max_row = processor.ofs_buf.window_size.row_height;
        let current_row = processor.ofs_buf.my_pos.row_index.as_usize();

        // Get scroll region boundaries (1-based to 0-based conversion)
        let scroll_bottom = processor
            .ofs_buf
            .ansi_parser_support
            .scroll_region_bottom
            .and_then(|b| b.to_zero_based()) // Convert 1-based to 0-based
            .map(|row| row.as_usize())
            .unwrap_or(max_row.as_usize().saturating_sub(1));

        // Check if we're at the bottom of the scroll region
        if current_row >= scroll_bottom {
            // At scroll region bottom - scroll buffer content up by one line
            scroll_buffer_up(processor);
        } else {
            // Not at scroll region bottom - just move cursor down
            cursor_ops::cursor_down(processor, 1);
        }
    }

    /// Move cursor up one line, scrolling the buffer if at top.
    /// Implements the ESC M (RI) escape sequence.
    /// Respects DECSTBM scroll region margins.
    pub fn reverse_index_up(processor: &mut AnsiToBufferProcessor) {
        let current_row = processor.ofs_buf.my_pos.row_index.as_usize();

        // Get scroll region boundaries (1-based to 0-based conversion)
        let scroll_top = processor
            .ofs_buf
            .ansi_parser_support
            .scroll_region_top
            .and_then(|t| t.to_zero_based()) // Convert 1-based to 0-based
            .map(|row| row.as_usize())
            .unwrap_or(0);

        // Check if we're at the top of the scroll region
        if current_row <= scroll_top {
            // At scroll region top - scroll buffer content down by one line
            scroll_buffer_down(processor);
        } else {
            // Not at scroll region top - just move cursor up
            cursor_ops::cursor_up(processor, 1);
        }
    }

    /// Scroll buffer content up by one line (for ESC D at bottom).
    /// The top line is lost, and a new empty line appears at bottom.
    /// Respects DECSTBM scroll region margins.
    pub fn scroll_buffer_up(processor: &mut AnsiToBufferProcessor) {
        let max_row = processor.ofs_buf.window_size.row_height.as_usize();

        // Get scroll region boundaries (1-based to 0-based conversion)
        let scroll_top = processor
            .ofs_buf
            .ansi_parser_support
            .scroll_region_top
            .and_then(|t| t.to_zero_based()) // Convert 1-based to 0-based
            .map(|row| row.as_usize())
            .unwrap_or(0);
        let scroll_bottom = processor
            .ofs_buf
            .ansi_parser_support
            .scroll_region_bottom
            .and_then(|b| b.to_zero_based()) // Convert 1-based to 0-based
            .map(|row| row.as_usize())
            .unwrap_or(max_row.saturating_sub(1));

        // Shift lines up within the scroll region only
        // For each row from top to (bottom-1), copy the row below it
        for row in scroll_top..scroll_bottom {
            processor.ofs_buf.buffer[row] = processor.ofs_buf.buffer[row + 1].clone();
        }

        // Clear the bottom line of the scroll region
        for col in 0..processor.ofs_buf.window_size.col_width.as_usize() {
            processor.ofs_buf.buffer[scroll_bottom][col] = PixelChar::Spacer;
        }
    }

    /// Scroll buffer content down by one line (for ESC M at top).
    /// The bottom line is lost, and a new empty line appears at top.
    /// Respects DECSTBM scroll region margins.
    pub fn scroll_buffer_down(processor: &mut AnsiToBufferProcessor) {
        let max_row = processor.ofs_buf.window_size.row_height.as_usize();

        // Get scroll region boundaries (1-based to 0-based conversion)
        let scroll_top = processor
            .ofs_buf
            .ansi_parser_support
            .scroll_region_top
            .and_then(|t| t.to_zero_based()) // Convert 1-based to 0-based
            .map(|row| row.as_usize())
            .unwrap_or(0);
        let scroll_bottom = processor
            .ofs_buf
            .ansi_parser_support
            .scroll_region_bottom
            .and_then(|b| b.to_zero_based()) // Convert 1-based to 0-based
            .map(|row| row.as_usize())
            .unwrap_or(max_row.saturating_sub(1));

        // Shift lines down within the scroll region only
        for row in (scroll_top + 1..=scroll_bottom).rev() {
            processor.ofs_buf.buffer[row] = processor.ofs_buf.buffer[row - 1].clone();
        }

        // Clear the new top line of the scroll region
        for col in 0..processor.ofs_buf.window_size.col_width.as_usize() {
            processor.ofs_buf.buffer[scroll_top][col] = PixelChar::Spacer;
        }
    }
}

/// Style/Graphics Rendition operations.
mod sgr_ops {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Update the current `TuiStyle` based on SGR attributes.
    pub fn update_style(processor: &mut AnsiToBufferProcessor) {
        processor.ofs_buf.ansi_parser_support.current_style = Some(TuiStyle {
            id: None,
            attribs: processor.ofs_buf.ansi_parser_support.attribs,
            computed: None,
            color_fg: processor.ofs_buf.ansi_parser_support.fg_color,
            color_bg: processor.ofs_buf.ansi_parser_support.bg_color,
            padding: None,
            lolcat: None,
        });
    }

    /// Reset all SGR attributes to default state.
    fn reset_all_attributes(processor: &mut AnsiToBufferProcessor) {
        processor.ofs_buf.ansi_parser_support.attribs.bold = None;
        processor.ofs_buf.ansi_parser_support.attribs.dim = None;
        processor.ofs_buf.ansi_parser_support.attribs.italic = None;
        processor.ofs_buf.ansi_parser_support.attribs.underline = None;
        processor.ofs_buf.ansi_parser_support.attribs.blink = None;
        processor.ofs_buf.ansi_parser_support.attribs.reverse = None;
        processor.ofs_buf.ansi_parser_support.attribs.hidden = None;
        processor.ofs_buf.ansi_parser_support.attribs.strikethrough = None;
        processor.ofs_buf.ansi_parser_support.fg_color = None;
        processor.ofs_buf.ansi_parser_support.bg_color = None;
    }

    /// Apply a single SGR parameter.
    fn apply_sgr_param(processor: &mut AnsiToBufferProcessor, param: u16) {
        match param {
            csi_codes::SGR_RESET => {
                reset_all_attributes(processor);
            }
            csi_codes::SGR_BOLD => {
                processor.ofs_buf.ansi_parser_support.attribs.bold =
                    Some(tui_style_attrib::Bold)
            }
            csi_codes::SGR_DIM => {
                processor.ofs_buf.ansi_parser_support.attribs.dim =
                    Some(tui_style_attrib::Dim)
            }
            csi_codes::SGR_ITALIC => {
                processor.ofs_buf.ansi_parser_support.attribs.italic =
                    Some(tui_style_attrib::Italic);
            }
            csi_codes::SGR_UNDERLINE => {
                processor.ofs_buf.ansi_parser_support.attribs.underline =
                    Some(tui_style_attrib::Underline);
            }
            csi_codes::SGR_BLINK | csi_codes::SGR_RAPID_BLINK => {
                processor.ofs_buf.ansi_parser_support.attribs.blink =
                    Some(tui_style_attrib::Blink);
            }
            csi_codes::SGR_REVERSE => {
                processor.ofs_buf.ansi_parser_support.attribs.reverse =
                    Some(tui_style_attrib::Reverse);
            }
            csi_codes::SGR_HIDDEN => {
                processor.ofs_buf.ansi_parser_support.attribs.hidden =
                    Some(tui_style_attrib::Hidden);
            }
            csi_codes::SGR_STRIKETHROUGH => {
                processor.ofs_buf.ansi_parser_support.attribs.strikethrough =
                    Some(tui_style_attrib::Strikethrough);
            }
            csi_codes::SGR_RESET_BOLD_DIM => {
                processor.ofs_buf.ansi_parser_support.attribs.bold = None;
                processor.ofs_buf.ansi_parser_support.attribs.dim = None;
            }
            csi_codes::SGR_RESET_ITALIC => {
                processor.ofs_buf.ansi_parser_support.attribs.italic = None
            }
            csi_codes::SGR_RESET_UNDERLINE => {
                processor.ofs_buf.ansi_parser_support.attribs.underline = None
            }
            csi_codes::SGR_RESET_BLINK => {
                processor.ofs_buf.ansi_parser_support.attribs.blink = None
            }
            csi_codes::SGR_RESET_REVERSE => {
                processor.ofs_buf.ansi_parser_support.attribs.reverse = None
            }
            csi_codes::SGR_RESET_HIDDEN => {
                processor.ofs_buf.ansi_parser_support.attribs.hidden = None
            }
            csi_codes::SGR_RESET_STRIKETHROUGH => {
                processor.ofs_buf.ansi_parser_support.attribs.strikethrough = None
            }
            csi_codes::SGR_FG_BLACK..=csi_codes::SGR_FG_WHITE => {
                processor.ofs_buf.ansi_parser_support.fg_color =
                    Some(ansi_to_tui_color(param.into()));
            }
            csi_codes::SGR_FG_DEFAULT => {
                processor.ofs_buf.ansi_parser_support.fg_color = None
            } /* Default foreground */
            csi_codes::SGR_BG_BLACK..=csi_codes::SGR_BG_WHITE => {
                processor.ofs_buf.ansi_parser_support.bg_color =
                    Some(ansi_to_tui_color(param.into()));
            }
            csi_codes::SGR_BG_DEFAULT => {
                processor.ofs_buf.ansi_parser_support.bg_color = None
            } /* Default background */
            csi_codes::SGR_FG_BRIGHT_BLACK..=csi_codes::SGR_FG_BRIGHT_WHITE => {
                processor.ofs_buf.ansi_parser_support.fg_color =
                    Some(ansi_to_tui_color(param.into()));
            }
            csi_codes::SGR_BG_BRIGHT_BLACK..=csi_codes::SGR_BG_BRIGHT_WHITE => {
                processor.ofs_buf.ansi_parser_support.bg_color =
                    Some(ansi_to_tui_color(param.into()));
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
        processor.ofs_buf.ansi_parser_support.current_style = None;
        processor.ofs_buf.ansi_parser_support.attribs.bold = None;
        processor.ofs_buf.ansi_parser_support.attribs.dim = None;
        processor.ofs_buf.ansi_parser_support.attribs.italic = None;
        processor.ofs_buf.ansi_parser_support.attribs.underline = None;
        processor.ofs_buf.ansi_parser_support.attribs.blink = None;
        processor.ofs_buf.ansi_parser_support.attribs.reverse = None;
        processor.ofs_buf.ansi_parser_support.attribs.hidden = None;
        processor.ofs_buf.ansi_parser_support.attribs.strikethrough = None;
        processor.ofs_buf.ansi_parser_support.fg_color = None;
        processor.ofs_buf.ansi_parser_support.bg_color = None;
    }

    /// Reset terminal to initial state (ESC c).
    /// Clears the buffer, resets cursor, and clears saved state.
    /// Clears DECSTBM scroll region margins.
    pub fn reset_terminal(processor: &mut AnsiToBufferProcessor) {
        clear_buffer(processor);

        // Reset cursor to home position
        processor.ofs_buf.my_pos = Pos::default();

        // Clear saved cursor state
        processor
            .ofs_buf
            .ansi_parser_support
            .cursor_pos_for_esc_save_and_restore = None;

        // Reset to ASCII character set
        processor.ofs_buf.ansi_parser_support.character_set = CharacterSet::Ascii;

        // Clear DECSTBM scroll region margins
        processor.ofs_buf.ansi_parser_support.scroll_region_top = None;
        processor.ofs_buf.ansi_parser_support.scroll_region_bottom = None;

        // Clear any SGR attributes
        reset_sgr_attributes(processor);

        tracing::trace!("ESC c: Terminal reset to initial state");
    }
}

/// Character set translation operations.
mod char_translation {
    /// Translate DEC Special Graphics characters to Unicode box-drawing characters.
    /// Used when `character_set` is DECGraphics (after ESC ( 0).
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
