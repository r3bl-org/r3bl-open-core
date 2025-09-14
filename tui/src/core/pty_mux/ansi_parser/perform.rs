// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Internal implementation for ANSI/VT sequence processing.
//!
//! This parser is based on the `vte` crate's `Perform` trait, and is [VT100
//! specifications](https://vt100.net/docs/vt100-ug/chapter3.html)
//! compliant. It provides support to parse ANSI escape sequences and update
//! an [`crate::OffscreenBuffer`] accordingly.
//!
//! # PTY Output Processing Pipeline
//!
//! ```text
//! Child Process (vim, bash, etc.)
//!         ↓
//!     PTY Slave (writes various sequence types)
//!         ↓
//!     PTY Master (byte stream) <- in process_manager.rs
//!         ↓
//!     VTE Parser (tokenizes & identifies sequence types)
//!         ↓
//!     Perform trait methods [THIS MODULE]
//!         ↓
//!     Update OffscreenBuffer state
//! ```
//!
//! # Sequence Types & Dispatch Routing
//!
//! The VTE parser identifies different types of sequences and calls the appropriate
//! method on this `Perform` implementation:
//!
//! | Sequence Type | Pattern        | Example           | Dispatch Method    | Purpose          |
//! |---------------|----------------|-------------------|--------------------|------------------|
//! | **Printable** | Regular chars  | `"Hello"`         | [`print()`]        | Display text     |
//! | **Control**   | C0 bytes       | `\n`, `\t`, `\b`  | [`execute()`]      | Cursor control   |
//! | **CSI**       | `ESC[...char`  | `ESC[2A`, `ESC[m` | [`csi_dispatch()`] | Complex commands |
//! | **OSC**       | `ESC]...ST`    | `ESC]0;title`     | [`osc_dispatch()`] | OS integration   |
//! | **ESC**       | `ESC char`     | `ESC c`, `ESC 7`  | [`esc_dispatch()`] | Simple commands  |
//! | **DCS**       | `ESC P...ST`   | Ignored (stubs)   | [`hook()`]         | Device control   |
//!
//! # VTE Parser State Machine
//!
//! The [VTE parser](vte::Parser) uses a state machine to recognize sequence boundaries:
//!
//! - **Ground state**: Collects printable characters → calls [`print()`]
//! - **Escape state**: After `ESC`, determines sequence type
//! - **CSI state**: After `ESC[`, collects parameters → calls [`csi_dispatch()`]
//! - **OSC state**: After `ESC]`, collects string → calls [`osc_dispatch()`]
//! - **Control characters**: Immediate → calls [`execute()`]
//!
//! Each method contains detailed architecture diagrams showing the specific flow
//! for that sequence type.
//!
//! [`print()`]: AnsiToOfsBufPerformer::print
//! [`execute()`]: AnsiToOfsBufPerformer::execute
//! [`csi_dispatch()`]: AnsiToOfsBufPerformer::csi_dispatch
//! [`osc_dispatch()`]: AnsiToOfsBufPerformer::osc_dispatch
//! [`esc_dispatch()`]: AnsiToOfsBufPerformer::esc_dispatch
//! [`hook()`]: AnsiToOfsBufPerformer::hook

use std::cmp::min;

use vte::{Params, Perform};

// Import the operation modules.
use super::operations::{char_ops, cursor_ops, dsr_ops, line_ops, margin_ops, mode_ops,
                        scroll_ops, sgr_ops, terminal_ops};
use super::{ansi_parser_public_api::AnsiToOfsBufPerformer,
            protocols::{csi_codes::{self},
                        esc_codes}};
use crate::{BoundsCheck,
            BoundsOverflowStatus::{Overflowed, Within},
            CharacterSet, ColIndex, PixelChar, RowIndex, col,
            core::osc::{OscEvent, osc_codes}};

/// Internal methods for `AnsiToOfsBufPerformer` to implement [`Perform`] trait.
impl Perform for AnsiToOfsBufPerformer<'_> {
    /// Handle printable characters.
    ///
    /// See [module docs](self) for complete processing pipeline overview.
    ///
    /// ## Print Sequence Architecture
    ///
    /// ```text
    /// Application writes "Hello"
    ///         ↓
    ///     PTY Slave (character stream)
    ///         ↓
    ///     PTY Master (we read bytes) <- in process_manager.rs
    ///         ↓
    ///     VTE Parser (identifies printable chars)
    ///         ↓
    ///     print() [THIS METHOD]
    ///         ↓
    ///     Character Set Translation (if DEC graphics)
    ///         ↓
    ///     Bounds Check & Write to Buffer
    ///         ↓
    ///     Cursor Movement (with DECAWM wrap handling)
    /// ```
    ///
    /// ## Character Processing Flow
    /// 1. Receives printable character from VTE parser
    /// 2. Translates character if `DECGraphics` mode active (ESC ( 0)
    /// 3. Writes character to buffer at current cursor position
    /// 4. Advances cursor, handling line wrap based on DECAWM mode
    ///
    /// ## Example: Line Wrapping
    /// ```text
    /// Buffer width: 10 cols
    /// Cursor at col 9, DECAWM enabled:
    ///   print('A') → writes at col 9, cursor moves to col 10
    ///   print('B') → writes at col 10, wraps to next line col 0
    ///
    /// With DECAWM disabled:
    ///   print('A') → writes at col 9, cursor moves to col 10
    ///   print('B') → overwrites at col 10, cursor stays at col 10
    /// ```
    fn print(&mut self, ch: char) {
        // Apply character set translation if in graphics mode.
        let display_char = match self.ofs_buf.ansi_parser_support.character_set {
            CharacterSet::DECGraphics => translate_dec_graphics(ch),
            CharacterSet::Ascii => ch,
        };

        let row_max = self.ofs_buf.window_size.row_height;
        let col_max = self.ofs_buf.window_size.col_width;
        let current_row = self.ofs_buf.cursor_pos.row_index;
        let current_col = self.ofs_buf.cursor_pos.col_index;

        // Only write if within bounds.
        if current_row.check_overflows(row_max) == Within
            && current_col.check_overflows(col_max) == Within
        {
            self.ofs_buf.set_char(
                current_row + current_col,
                PixelChar::PlainText {
                    display_char, // Use the translated character
                    style: self.ofs_buf.ansi_parser_support.current_style,
                },
            );

            // Move cursor forward.
            let new_col: ColIndex = current_col + 1;

            // Handle line wrap based on DECAWM (Auto Wrap Mode).
            if new_col.check_overflows(col_max) == Overflowed {
                if self.ofs_buf.ansi_parser_support.auto_wrap_mode {
                    // DECAWM enabled: wrap to next line (default behavior)
                    self.ofs_buf.cursor_pos.col_index = col(0);
                    let next_row: RowIndex = current_row + 1;
                    if next_row.check_overflows(row_max) == Within {
                        self.ofs_buf.cursor_pos.row_index = next_row;
                    }
                } else {
                    // DECAWM disabled: stay at right margin (clamp cursor position)
                    self.ofs_buf.cursor_pos.col_index = col_max.convert_to_col_index();
                }
            } else {
                self.ofs_buf.cursor_pos.col_index = new_col;
            }
        }
    }

    /// Handle control characters (C0 set): backspace, tab, LF, CR.
    ///
    /// See [module docs](self) for complete processing pipeline overview.
    ///
    /// ## Control Character Architecture
    ///
    /// ```text
    /// Application sends '\n' (0x0A)
    ///         ↓
    ///     PTY Slave (control byte)
    ///         ↓
    ///     PTY Master (raw byte stream) <- in process_manager.rs
    ///         ↓
    ///     VTE Parser (identifies C0 control chars)
    ///         ↓
    ///     execute() [THIS METHOD]
    ///         ↓
    ///     Direct cursor manipulation
    ///         ↓
    ///     No buffer content changes
    /// ```
    ///
    /// ## Supported Control Characters
    /// - **BS (0x08)**: Move cursor left one position
    /// - **TAB (0x09)**: Move cursor to next 8-column tab stop
    /// - **LF (0x0A)**: Move cursor down one line
    /// - **CR (0x0D)**: Move cursor to start of current line
    ///
    /// ## Tab Stop Example
    /// ```text
    /// Tab stops at columns: 0, 8, 16, 24, 32
    /// Cursor at col 5 + TAB → moves to col 8
    /// Cursor at col 12 + TAB → moves to col 16
    /// ```
    fn execute(&mut self, byte: u8) {
        match byte {
            // Backspace
            esc_codes::BACKSPACE => {
                let current_col = self.ofs_buf.cursor_pos.col_index;
                if current_col > col(0) {
                    self.ofs_buf.cursor_pos.col_index = current_col - 1;
                }
            }
            // Tab - move to next tab stop boundary.
            esc_codes::TAB => {
                let current_col = self.ofs_buf.cursor_pos.col_index;
                let current_tab_zone = current_col.as_usize() / esc_codes::TAB_STOP_WIDTH;
                let next_tab_zone = current_tab_zone + 1;
                let next_tab_col = next_tab_zone * esc_codes::TAB_STOP_WIDTH;
                let max_col = self.ofs_buf.window_size.col_width;

                // Clamp to max valid column index if it would overflow.
                self.ofs_buf.cursor_pos.col_index =
                    col(min(next_tab_col, max_col.convert_to_col_index().as_usize()));
            }
            // Line feed (newline)
            esc_codes::LINE_FEED => {
                let max_row = self.ofs_buf.window_size.row_height;
                let next_row: RowIndex = self.ofs_buf.cursor_pos.row_index + 1;
                if next_row.check_overflows(max_row) == Within {
                    self.ofs_buf.cursor_pos.row_index = next_row;
                }
            }
            // Carriage return
            esc_codes::CARRIAGE_RETURN => {
                self.ofs_buf.cursor_pos.col_index = col(0);
            }
            _ => {}
        }
    }

    /// Handle CSI (Control Sequence Introducer) sequences.
    ///
    /// See [module docs](self) for complete processing pipeline overview.
    ///
    /// This method processes ANSI escape sequences that follow the pattern `ESC[...char`
    /// where `char` is the final dispatch character that determines the operation.
    ///
    /// ## CSI Sequence Architecture
    ///
    /// ```text
    /// Application sends "ESC[2A" (cursor up 2 lines)
    ///         ↓
    ///     PTY Slave (escape sequence)
    ///         ↓
    ///     PTY Master (byte stream) <- in process_manager.rs
    ///         ↓
    ///     VTE Parser (parses ESC[...char pattern)
    ///         ↓
    ///     csi_dispatch() [THIS METHOD]
    ///         ↓
    ///     Route to operation module:
    ///       - cursor_ops:: for movement (A,B,C,D,H)
    ///       - scroll_ops:: for scrolling (S,T)
    ///       - sgr_ops:: for styling (m)
    ///       - line_ops:: for lines (L,M)
    ///       - char_ops:: for chars (@,P,X)
    ///         ↓
    ///     Update OffscreenBuffer state
    /// ```
    ///
    /// ## CSI Dispatch Flow
    /// 1. Receives parsed CSI parameters from VTE
    /// 2. Matches final dispatch character to operation
    /// 3. Delegates to specialized operation module
    /// 4. Operation module updates buffer/cursor state
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
            // Cursor movement operations.
            csi_codes::CUU_CURSOR_UP => cursor_ops::cursor_up(self, params),
            csi_codes::CUD_CURSOR_DOWN => cursor_ops::cursor_down(self, params),
            csi_codes::CUF_CURSOR_FORWARD => cursor_ops::cursor_forward(self, params),
            csi_codes::CUB_CURSOR_BACKWARD => cursor_ops::cursor_backward(self, params),
            csi_codes::CUP_CURSOR_POSITION | csi_codes::HVP_CURSOR_POSITION => {
                cursor_ops::cursor_position(self, params);
            }
            csi_codes::CNL_CURSOR_NEXT_LINE => cursor_ops::cursor_next_line(self, params),
            csi_codes::CPL_CURSOR_PREV_LINE => cursor_ops::cursor_prev_line(self, params),
            csi_codes::CHA_CURSOR_COLUMN => {
                cursor_ops::cursor_column(self, params);
            }
            csi_codes::SCP_SAVE_CURSOR => cursor_ops::save_cursor_position(self),
            csi_codes::RCP_RESTORE_CURSOR => cursor_ops::restore_cursor_position(self),

            // Scrolling operations.
            csi_codes::SU_SCROLL_UP => scroll_ops::scroll_up(self, params),
            csi_codes::SD_SCROLL_DOWN => scroll_ops::scroll_down(self, params),

            // Margin operations.
            csi_codes::DECSTBM_SET_MARGINS => margin_ops::set_margins(self, params),

            // Device status operations.
            csi_codes::DSR_DEVICE_STATUS => dsr_ops::status_report(self, params),

            // Mode operations
            csi_codes::SM_SET_MODE => mode_ops::set_mode(self, params, intermediates),
            csi_codes::RM_RESET_MODE => mode_ops::reset_mode(self, params, intermediates),

            // Graphics operations.
            csi_codes::SGR_SET_GRAPHICS => sgr_ops::set_graphics_rendition(self, params),

            // Line operations
            csi_codes::IL_INSERT_LINE => line_ops::insert_lines(self, params),
            csi_codes::DL_DELETE_LINE => line_ops::delete_lines(self, params),

            // Character operations.
            csi_codes::DCH_DELETE_CHAR => char_ops::delete_chars(self, params),
            csi_codes::ICH_INSERT_CHAR => char_ops::insert_chars(self, params),
            csi_codes::ECH_ERASE_CHAR => char_ops::erase_chars(self, params),

            // Additional cursor positioning.
            csi_codes::VPA_VERTICAL_POSITION => {
                cursor_ops::vertical_position_absolute(self, params);
            }

            // Display control operations (explicitly ignored)
            csi_codes::ED_ERASE_DISPLAY | csi_codes::EL_ERASE_LINE => {
                // Clear screen/line - ignore, TUI apps will repaint themselves
                tracing::warn!(
                    "CSI {}: Clear display/line operation ignored",
                    dispatch_char
                );
            }

            // Other unimplemented CSI sequences.
            'I' => {
                // CHT (Cursor Horizontal Tab) - Move cursor forward N tab stops
                // Not needed: Tab handling is done via execute() with TAB character
                tracing::warn!("CSI I: Cursor Horizontal Tab not implemented");
            }
            'Z' => {
                // CBT (Cursor Backward Tab) - Move cursor backward N tab stops
                // Not needed: Reverse tab rarely used, complex tab stop tracking required
                tracing::warn!("CSI Z: Cursor Backward Tab not implemented");
            }
            'g' => {
                // TBC (Tab Clear) - Clear tab stops (0=current, 3=all)
                // Not needed: Tab stops are application-specific, TUI apps manage their
                // own
                tracing::warn!("CSI g: Tab Clear not implemented");
            }
            'a' => {
                // HPR (Horizontal Position Relative) - Same as CUF (Cursor Forward)
                // Not needed: CUF already implemented, this is redundant
                tracing::warn!(
                    "CSI a: Horizontal Position Relative not implemented (use CUF instead)"
                );
            }
            'e' => {
                // VPR (Vertical Position Relative) - Same as CUD (Cursor Down)
                // Not needed: CUD already implemented, this is redundant
                tracing::warn!(
                    "CSI e: Vertical Position Relative not implemented (use CUD instead)"
                );
            }
            '`' => {
                // HPA (Horizontal Position Absolute) - Same as CHA
                // Not needed: CHA already implemented, this is redundant
                tracing::warn!(
                    "CSI `: Horizontal Position Absolute not implemented (use CHA instead)"
                );
            }
            'U' => {
                // NP (Next Page) - Move to next page in page memory
                // Not needed: Page memory not supported in multiplexer
                tracing::warn!("CSI U: Next Page not supported in multiplexer");
            }
            'V' => {
                // PP (Preceding Page) - Move to previous page in page memory
                // Not needed: Page memory not supported in multiplexer
                tracing::warn!("CSI V: Preceding Page not supported in multiplexer");
            }
            '~' => {
                // DECLL (DEC Load LEDs) - Set keyboard LED indicators
                // Not needed: Hardware control not applicable in multiplexer
                tracing::warn!("CSI ~: DEC Load LEDs not supported in multiplexer");
            }
            '}' => {
                // DECIC (DEC Insert Column) - Insert blank columns at cursor
                // Not needed: Column insertion rarely used, complex for TUI apps
                tracing::warn!("CSI }}: DEC Insert Column not implemented");
            }
            '|' => {
                // DECDC (DEC Delete Column) - Delete columns at cursor
                // Not needed: Column deletion rarely used, complex for TUI apps
                tracing::warn!("CSI |: DEC Delete Column not implemented");
            }
            't' => {
                // Window manipulation (resize, move, iconify, etc.)
                // Not needed: Window ops handled by terminal emulator, not multiplexer
                tracing::warn!("CSI t: Window manipulation not supported in multiplexer");
            }
            'c' => {
                // DA (Device Attributes) - Request terminal type/capabilities
                // Not needed: Multiplexer doesn't respond to queries, parent terminal
                // does
                tracing::warn!(
                    "CSI c: Device Attributes query not supported in multiplexer"
                );
            }
            'q' => {
                // DECSCUSR (Set Cursor Style) - Change cursor shape/blink
                // Not needed: Cursor rendering handled by terminal emulator
                tracing::warn!("CSI q: Set Cursor Style not supported in multiplexer");
            }
            'p' => {
                // Various DEC private sequences (DECRQM, etc.)
                // Not needed: Private mode requests handled by parent terminal
                tracing::warn!(
                    "CSI p: DEC private sequences not supported in multiplexer"
                );
            }
            'x' => {
                // DECREQTPARM (Request Terminal Parameters) - Request terminal settings
                // Not needed: Terminal parameters managed by parent emulator
                tracing::warn!(
                    "CSI x: Request Terminal Parameters not supported in multiplexer"
                );
            }
            'z' => {
                // DECERA/DECSERA (DEC Erase/Selective Erase Rectangular Area)
                // Not needed: Rectangular operations complex, rarely used
                tracing::warn!("CSI z: DEC Rectangular Erase not implemented");
            }

            // Any other unrecognized sequences.
            _ => {
                // Unknown CSI sequence - safely ignore.
                // Multiplexer passes through raw data, parent terminal handles unknowns.
                tracing::warn!("CSI {}: Unknown CSI sequence", dispatch_char);
            }
        }
    }

    /// Handle OSC (Operating System Command) sequences.
    ///
    /// See [module docs](self) for complete processing pipeline overview.
    ///
    /// ## OSC Sequence Architecture
    ///
    /// ```text
    /// Application sends "ESC]0;My Title\007"
    ///         ↓
    ///     PTY Slave (OSC sequence)
    ///         ↓
    ///     PTY Master (byte stream) <- in process_manager.rs
    ///         ↓
    ///     VTE Parser (accumulates OSC params)
    ///         ↓
    ///     osc_dispatch() [THIS METHOD]
    ///         ↓
    ///     Parse OSC code & params
    ///         ↓
    ///     Queue OscEvent:
    ///       - SetTitleAndTab (OSC 0,1,2)
    ///       - Hyperlink (OSC 8)
    ///         ↓
    ///     Events consumed by OutputRenderer
    /// ```
    ///
    /// ## OSC Processing Flow
    /// 1. Receives complete OSC sequence from VTE
    /// 2. Parses first param as OSC code
    /// 3. Processes based on code (title, hyperlink, etc)
    /// 4. Queues events for later rendering
    ///
    /// ## Example: Window Title
    /// ```text
    /// OSC 0 ; "vim - file.rs" ST
    ///   ↓
    /// params[0] = "0" (code)
    /// params[1] = "vim - file.rs" (title)
    ///   ↓
    /// Pushes SetTitleAndTab event
    /// ```
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
                        // For now, just store the URI - display text would come from.
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
                // contexts) We could handle them here too if needed.
                _ => {
                    // Ignore other OSC sequences for now.
                }
            }
        }
    }

    /// Handle escape sequences (not CSI or OSC).
    ///
    /// See [module docs](self) for complete processing pipeline overview.
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
    ///     PTY Master (we read from here) <- in process_manager.rs
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
    ///   → AnsiToOfsBufPerformer::new() with ofs_buf.my_pos = (5,10)
    ///   → esc_dispatch() handles ESC 7
    ///   → Saves ofs_buf.ansi_parser_support.cursor_pos_for_esc_save_and_restore = Some((5,10))
    ///
    /// Session 2: vim moves cursor to (20,30), then sends ESC 8
    ///   → AnsiToOfsBufPerformer::new() with ofs_buf.my_pos = (20,30)
    ///   → esc_dispatch() handles ESC 8
    ///   → Restores ofs_buf.my_pos = cursor_pos_for_esc_save_and_restore.unwrap_or() // (5,10) ✓
    /// ```
    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        match byte {
            esc_codes::DECSC_SAVE_CURSOR => {
                // DECSC - Save current cursor position.
                // The cursor position is saved to persistent buffer state so it
                // survives across multiple AnsiToOfsBufPerformer instances.
                self.ofs_buf
                    .ansi_parser_support
                    .cursor_pos_for_esc_save_and_restore = Some(self.ofs_buf.cursor_pos);
            }
            esc_codes::DECRC_RESTORE_CURSOR => {
                // DECRC - Restore saved cursor position.
                // Retrieves the previously saved position from buffer's persistent state.
                if let Some(saved_pos) = self
                    .ofs_buf
                    .ansi_parser_support
                    .cursor_pos_for_esc_save_and_restore
                {
                    self.ofs_buf.cursor_pos = saved_pos;
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
                // RIS - Reset to Initial State.
                terminal_ops::reset_terminal(self);
            }
            _ if intermediates == esc_codes::G0_CHARSET_INTERMEDIATE => {
                // Character set selection G0.
                match byte {
                    esc_codes::CHARSET_ASCII => {
                        // Select ASCII character set (normal mode)
                        self.ofs_buf.ansi_parser_support.character_set =
                            CharacterSet::Ascii;
                    }
                    esc_codes::CHARSET_DEC_GRAPHICS => {
                        // Select DEC Special Graphics character set.
                        // This enables box-drawing characters.
                        self.ofs_buf.ansi_parser_support.character_set =
                            CharacterSet::DECGraphics;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    /// Hook for DCS (Device Control String) start.
    ///
    /// See [module docs](self) for complete processing pipeline overview.
    ///
    /// ## DCS Sequence Architecture (Not Implemented)
    ///
    /// ```text
    /// Application sends DCS sequence
    ///         ↓
    ///     VTE Parser identifies DCS
    ///         ↓
    ///     hook() → put() → unhook()
    ///         ↓
    ///     Currently ignored (no DCS support)
    /// ```
    ///
    /// DCS sequences are used for:
    /// - Sixel graphics
    /// - `ReGIS` graphics
    /// - Custom protocol extensions
    ///
    /// These are not needed for terminal multiplexing.
    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _c: char) {
        // Ignore DCS sequences.
    }

    /// Handle DCS data by continuing to receive bytes for an active DCS sequence started
    /// by hook.
    ///
    /// This method receives the actual data payload of a DCS sequence. For terminal
    /// multiplexing, DCS sequences are not processed, so this data is ignored.
    fn put(&mut self, _byte: u8) {
        // Ignore DCS data
    }

    /// Hook for DCS - ends the DCS sequence, signaling that all data has been received.
    ///
    /// This marks the end of a DCS sequence that began with `hook()` and had data
    /// transmitted via `put()`. Since DCS sequences are not processed by the terminal
    /// multiplexer, this simply completes the ignored sequence.
    fn unhook(&mut self) {
        // Ignore DCS end.
    }
}

/// Translate DEC Special Graphics characters to Unicode box-drawing characters.
/// Used when `character_set` is `DECGraphics` (after ESC ( 0).
#[must_use]
fn translate_dec_graphics(c: char) -> char {
    match c {
        'j' => '┘', // Lower right corner.
        'k' => '┐', // Upper right corner.
        'l' => '┌', // Upper left corner.
        'm' => '└', // Lower left corner.
        'n' => '┼', // Crossing lines.
        'q' => '─', // Horizontal line.
        't' => '├', // Left "T".
        'u' => '┤', // Right "T".
        'v' => '┴', // Bottom "T".
        'w' => '┬', // Top "T".
        'x' => '│', // Vertical line.
        _ => c,     // Pass through unmapped characters.
    }
}
