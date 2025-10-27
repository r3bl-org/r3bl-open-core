// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Internal implementation for ANSI/VT sequence processing.
//!
//! This parser is based on the [vte] crate's [`Perform`] trait, and is [VT100
//! specifications] compliant. It provides support to parse ANSI escape sequences and
//! update an [`OffscreenBuffer`] accordingly.
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
//! method on this [`Perform`] implementation:
//!
//! | Sequence Type | Pattern         | Example            | Dispatch Method      | Purpose           |
//! |---------------|-----------------|--------------------|----------------------|-------------------|
//! | **Printable** | Regular chars   | `"Hello"`          | [`print()`]          | Display text      |
//! | **Control**   | C0 bytes        | `\n`, `\t`, `\b`   | [`execute()`]        | Cursor control    |
//! | **CSI**       | `ESC[...char`   | `ESC[2A`, `ESC[m`  | [`csi_dispatch()`]   | Complex commands  |
//! | **OSC**       | `ESC]...ST`     | `ESC]0;title`      | [`osc_dispatch()`]   | OS integration    |
//! | **ESC**       | `ESC char`      | `ESC c`, `ESC 7`   | [`esc_dispatch()`]   | Simple commands   |
//! | **DCS**       | `ESC P...ST`    | Ignored (stubs)    | [`hook()`]           | Device control    |
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
//! # Malformed Sequences and the `ignore` Parameter
//!
//! The VTE parser includes built-in limits to prevent resource exhaustion from
//! maliciously crafted or corrupted sequences. When these limits are exceeded, the
//! parser sets an `ignore` flag to signal that data was dropped during parsing.
//!
//! ## When `ignore` Becomes `true`
//!
//! The VTE parser sets `ignore = true` when either:
//! - **Too many intermediate bytes**: More than 2 intermediate characters (e.g.,
//!   `ESC[?>??>A`)
//! - **Too many parameters**: Parameter count exceeds the parser's internal limit
//!   (typically 16)
//!
//! When this happens, the parser:
//! 1. Collects as much data as it can (up to the limit)
//! 2. **Still calls the dispatch method** with the partial data
//! 3. Sets `ignore = true` to warn that some data was dropped
//!
//! ## Example of Malformed Sequence
//!
//! ```text
//! Application sends: ESC[1;2;3;4;5;6;7;8;9;10;11;12;13;14;15;16;17;18;19;20H
//!                    (Cursor position with 20 parameters - exceeds VTE's 16-param limit)
//!
//! VTE Parser processes:
//!   ✓ Collects params[0..15] = [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16]
//!   ✗ Drops params[16..19] = [17,18,19,20]
//!   ✓ Finds dispatch character 'H'
//!   → Calls csi_dispatch(params[0..15], [], ignore=true, 'H')
//!
//! Correct Response: Discard the entire sequence (don't execute with partial data)
//! Wrong Response:   Execute cursor_position(1,2) ignoring missing critical parameters
//! ```
//!
//! ## Industry Best Practices
//!
//! All major terminal emulators **discard malformed sequences** entirely rather than
//! risk executing commands with incorrect parameters:
//!
//! - **[Alacritty]**: Checks `ignore` flag, returns early without processing
//!   ([source][alacritty-pr-2664])
//! - **[kitty]**: Trashes entire control sequence when limits exceeded
//! - **[st (suckless terminal)]**: Trashes entire control sequence
//! - **[GNOME VTE]**: Trashes entire control sequence
//!
//! This approach is mandated by the [VT100 specification] and [ECMA-48 standard],
//! which require terminals to ignore malformed sequences rather than execute
//! potentially incorrect commands.
//!
//! ## Implementation in This Module
//!
//! The [`Perform`] trait methods that **process sequences** check the `ignore` parameter
//! and discard malformed sequences before processing:
//!
//! - [`csi_dispatch()`]: Checks `ignore` flag, logs and returns early if true
//! - [`esc_dispatch()`]: Checks `ignore` flag, logs and returns early if true
//! - [`hook()`]: Does NOT check `ignore` - DCS sequences are not implemented at all, so
//!   both well-formed and malformed DCS sequences are ignored equally
//!
//! This ensures robust handling of corrupted or malicious input while maintaining
//! compatibility with terminal emulator best practices.
//!
//! # Design Architecture
//!
//! The [`AnsiToOfsBufPerformer`] is deliberately designed as a **thin shim** that
//! delegates all operations to dedicated methods in [`OffscreenBuffer`]. This design
//! pattern is consistently implemented throughout the module:
//!
//! - The performer contains minimal logic and acts purely as a translation layer
//! - ANSI sequence parameters are parsed and translated into appropriate
//!   [`OffscreenBuffer`] method calls
//! - All actual terminal buffer operations (cursor movement, scrolling, text rendering)
//!   are implemented in [`OffscreenBuffer`] methods
//! - This separation ensures clear boundaries between ANSI protocol handling and buffer
//!   management
//!
//! This thin shim pattern provides clean separation of concerns: the performer handles
//! ANSI/VT protocol specifics while [`OffscreenBuffer`] handles terminal buffer
//! semantics.
//!
//! # Implementation Architecture - 1:1 File Mapping
//!
//! The parser operations follow a **perfect 1:1 mapping** between the parser layer
//! (this module) and the implementation layer in [`OffscreenBuffer`]. This provides
//! clear organization and predictable code navigation:
//!
//! ```text
//! vt_100_ansi_parser/operations/             offscreen_buffer/vt_100_ansi_impl/
//! ├── vt_100_shim_char_ops         →         ├── vt_100_impl_char_ops
//! ├── vt_100_shim_control_ops      →         ├── vt_100_impl_control_ops
//! ├── vt_100_shim_cursor_ops       →         ├── vt_100_impl_cursor_ops
//! ├── vt_100_shim_dsr_ops          →         ├── vt_100_impl_dsr_ops
//! ├── vt_100_shim_line_ops         →         ├── vt_100_impl_line_ops
//! ├── vt_100_shim_margin_ops       →         ├── vt_100_impl_margin_ops
//! ├── vt_100_shim_mode_ops         →         ├── vt_100_impl_mode_ops
//! ├── vt_100_shim_osc_ops          →         ├── vt_100_impl_osc_ops
//! ├── vt_100_shim_scroll_ops       →         ├── vt_100_impl_scroll_ops
//! ├── vt_100_shim_sgr_ops          →         ├── vt_100_impl_sgr_ops
//! └── vt_100_shim_terminal_ops     →         └── vt_100_impl_terminal_ops
//! ```
//!
//! Each operations file contains **thin shim functions** that:
//! 1. Parse ANSI sequence parameters, and act as the protocol boundary layer (using
//!    [`vte::Params`] and [`ParamsExt`])
//! 2. Delegate to the corresponding [`OffscreenBuffer`] implementation (which are not
//!    aware of [`vte::Params`])
//! 3. Provide clear documentation about the ANSI specification
//!
//! The [`OffscreenBuffer`] implementation files contain the **actual terminal logic**:
//! 1. Full VT100-compliant behavior implementation
//! 2. Comprehensive unit tests
//! 3. Detailed examples and edge case handling
//!
//! This architecture ensures that ANSI parsing concerns are separated from terminal
//! emulation logic while maintaining clear organizational correspondence.
//!
//! # Testing Strategy
//!
//! This module uses a delegation-based testing approach that differs from the codebase
//! norm:
//! - The operations in this module are thin wrappers that delegate to [`OffscreenBuffer`]
//!   methods
//! - [`OffscreenBuffer`] methods have comprehensive unit tests (following codebase
//!   convention)
//! - VT100 conformance tests in [`vt_100_ansi_conformance_tests`] verify end-to-end
//!   behavior
//!
//! This approach avoids redundant testing while ensuring both unit-level correctness
//! (in [`OffscreenBuffer`]) and system-level behavior (in conformance tests).
//!
//! See the [module-level documentation](super) for more details on the
//! architecture and testing strategy.
//!
//! [`OffscreenBuffer`]: crate::OffscreenBuffer
//! [`vt_100_ansi_conformance_tests`]: mod@super::vt_100_ansi_conformance_tests
//! [`ParamsExt`]: crate::ParamsExt
//! [`Perform`]: vte::Perform
//! [`print()`]: AnsiToOfsBufPerformer::print
//! [`execute()`]: AnsiToOfsBufPerformer::execute
//! [`csi_dispatch()`]: AnsiToOfsBufPerformer::csi_dispatch
//! [`osc_dispatch()`]: AnsiToOfsBufPerformer::osc_dispatch
//! [`esc_dispatch()`]: AnsiToOfsBufPerformer::esc_dispatch
//! [`hook()`]: AnsiToOfsBufPerformer::hook
//! [vte]: https://docs.rs/vte/latest/vte/
//! [Alacritty]: https://github.com/alacritty/alacritty
//! [alacritty-pr-2664]: https://github.com/alacritty/alacritty/pull/2664
//! [kitty]: https://sw.kovidgoyal.net/kitty/
//! [st (suckless terminal)]: https://st.suckless.org/
//! [GNOME VTE]: https://gitlab.gnome.org/GNOME/vte
//! [VT100 specification]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [ECMA-48 standard]: https://www.ecma-international.org/publications-and-standards/standards/ecma-48/

// Import the operation modules and public API.
use super::{ansi_parser_public_api::AnsiToOfsBufPerformer,
            operations::{vt_100_shim_char_ops, vt_100_shim_control_ops,
                         vt_100_shim_cursor_ops, vt_100_shim_dsr_ops,
                         vt_100_shim_line_ops, vt_100_shim_margin_ops,
                         vt_100_shim_mode_ops, vt_100_shim_osc_ops,
                         vt_100_shim_scroll_ops, vt_100_shim_sgr_ops,
                         vt_100_shim_terminal_ops}};
use crate::core::ansi::constants::{BACKSPACE, CARRIAGE_RETURN, CHA_CURSOR_COLUMN,
                                   CHARSET_ASCII, CHARSET_DEC_GRAPHICS,
                                   CNL_CURSOR_NEXT_LINE, CPL_CURSOR_PREV_LINE,
                                   CUB_CURSOR_BACKWARD, CUD_CURSOR_DOWN,
                                   CUF_CURSOR_FORWARD, CUP_CURSOR_POSITION,
                                   CUU_CURSOR_UP, DCH_DELETE_CHAR, DECRC_RESTORE_CURSOR,
                                   DECSC_SAVE_CURSOR, DECSTBM_SET_MARGINS,
                                   DL_DELETE_LINE, DSR_DEVICE_STATUS, ECH_ERASE_CHAR,
                                   ED_ERASE_DISPLAY, EL_ERASE_LINE,
                                   G0_CHARSET_INTERMEDIATE, HVP_CURSOR_POSITION,
                                   ICH_INSERT_CHAR, IL_INSERT_LINE, IND_INDEX_DOWN,
                                   LINE_FEED, RCP_RESTORE_CURSOR, RI_REVERSE_INDEX_UP,
                                   RIS_RESET_TERMINAL, RM_RESET_MODE, SCP_SAVE_CURSOR,
                                   SD_SCROLL_DOWN, SGR_SET_GRAPHICS, SM_SET_MODE,
                                   SU_SCROLL_UP, TAB, VPA_VERTICAL_POSITION};
use vte::{Params, Perform};

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
    fn print(&mut self, ch: char) { vt_100_shim_char_ops::print_char(self, ch); }

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
            BACKSPACE => {
                vt_100_shim_control_ops::handle_backspace(self);
            }
            TAB => {
                vt_100_shim_control_ops::handle_tab(self);
            }
            LINE_FEED => {
                vt_100_shim_control_ops::handle_line_feed(self);
            }
            CARRIAGE_RETURN => {
                vt_100_shim_control_ops::handle_carriage_return(self);
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
    /// - Cursor movements: CUU, CUD, CUF, CUB, CNL, CPL, CHA, CUP, HVP, VPA
    /// - Scrolling: SU (Scroll Up), SD (Scroll Down)
    /// - Line operations: IL (Insert Line), DL (Delete Line)
    /// - Character operations: ICH (Insert Char), DCH (Delete Char), ECH (Erase Char)
    /// - Display control: ED, EL (explicitly ignored - see comments)
    /// - Cursor save/restore: SCP, RCP
    /// - Margins: DECSTBM (Set Top and Bottom Margins)
    /// - Modes: SM, RM (including private modes with ? prefix)
    /// - Device status: DSR (Device Status Report)
    /// - Graphics: SGR (Select Graphic Rendition)
    ///
    /// ## Malformed Sequence Handling
    ///
    /// The `ignore` parameter signals when the VTE parser exceeded its limits during
    /// parsing (too many parameters or intermediates). When `ignore = true`, this method
    /// **discards the entire sequence** to prevent executing commands with incorrect
    /// parameters.
    ///
    /// See the [module-level
    /// documentation](self#malformed-sequences-and-the-ignore-parameter) for detailed
    /// explanation of why discarding malformed sequences is the correct
    /// approach, with references to industry best practices.
    #[allow(clippy::too_many_lines)]
    fn csi_dispatch(
        &mut self,
        params: &Params,
        intermediates: &[u8],
        ignore: bool,
        dispatch_char: char,
    ) {
        // Discard malformed sequences - see module docs for rationale
        if ignore {
            tracing::warn!(
                "CSI {}: Discarding malformed sequence (VTE parser exceeded limits)",
                dispatch_char
            );
            return;
        }

        #[allow(clippy::match_same_arms)]
        match dispatch_char {
            // Cursor movement operations.
            CUU_CURSOR_UP => vt_100_shim_cursor_ops::cursor_up(self, params),
            CUD_CURSOR_DOWN => {
                vt_100_shim_cursor_ops::cursor_down(self, params);
            }
            CUF_CURSOR_FORWARD => {
                vt_100_shim_cursor_ops::cursor_forward(self, params);
            }
            CUB_CURSOR_BACKWARD => {
                vt_100_shim_cursor_ops::cursor_backward(self, params);
            }
            CUP_CURSOR_POSITION | HVP_CURSOR_POSITION => {
                vt_100_shim_cursor_ops::cursor_position(self, params);
            }
            CNL_CURSOR_NEXT_LINE => {
                vt_100_shim_cursor_ops::cursor_next_line(self, params);
            }
            CPL_CURSOR_PREV_LINE => {
                vt_100_shim_cursor_ops::cursor_prev_line(self, params);
            }
            CHA_CURSOR_COLUMN => {
                vt_100_shim_cursor_ops::cursor_column(self, params);
            }
            SCP_SAVE_CURSOR => {
                vt_100_shim_cursor_ops::save_cursor_position(self);
            }
            RCP_RESTORE_CURSOR => {
                vt_100_shim_cursor_ops::restore_cursor_position(self);
            }

            // Scrolling operations.
            SU_SCROLL_UP => vt_100_shim_scroll_ops::scroll_up(self, params),
            SD_SCROLL_DOWN => {
                vt_100_shim_scroll_ops::scroll_down(self, params);
            }

            // Margin operations.
            DECSTBM_SET_MARGINS => {
                vt_100_shim_margin_ops::set_margins(self, params);
            }

            // Device status operations.
            DSR_DEVICE_STATUS => {
                vt_100_shim_dsr_ops::status_report(self, params);
            }

            // Mode operations.
            SM_SET_MODE => {
                vt_100_shim_mode_ops::set_mode(self, params, intermediates);
            }
            RM_RESET_MODE => {
                vt_100_shim_mode_ops::reset_mode(self, params, intermediates);
            }

            // Graphics operations.
            SGR_SET_GRAPHICS => {
                vt_100_shim_sgr_ops::set_graphics_rendition(self, params);
            }

            // Line operations.
            IL_INSERT_LINE => vt_100_shim_line_ops::insert_lines(self, params),
            DL_DELETE_LINE => vt_100_shim_line_ops::delete_lines(self, params),

            // Character operations.
            DCH_DELETE_CHAR => {
                vt_100_shim_char_ops::delete_chars(self, params);
            }
            ICH_INSERT_CHAR => {
                vt_100_shim_char_ops::insert_chars(self, params);
            }
            ECH_ERASE_CHAR => vt_100_shim_char_ops::erase_chars(self, params),

            // Additional cursor positioning.
            VPA_VERTICAL_POSITION => {
                vt_100_shim_cursor_ops::vertical_position_absolute(self, params);
            }

            // Display control operations (explicitly ignored).
            ED_ERASE_DISPLAY | EL_ERASE_LINE => {
                // Clear screen/line - ignore, TUI apps will repaint themselves
                tracing::warn!(
                    "CSI {}: Clear display/line operation ignored",
                    dispatch_char
                );
            }

            // Other unimplemented CSI sequences.
            'I' => {
                // CHT (Cursor Horizontal Tab) - Move cursor forward N tab stops
                // Not needed: Tab handling is done via execute() with TAB character.
                // See [mod-level docs](crate::vt_100_ansi_parser) for rationale
                tracing::warn!("CSI I: Cursor Horizontal Tab not implemented");
            }
            'Z' => {
                // CBT (Cursor Backward Tab) - Move cursor backward N tab stops
                // Not needed: Reverse tab rarely used, complex tab stop tracking required
                // See [mod-level docs](crate::vt_100_ansi_parser) for rationale
                tracing::warn!("CSI Z: Cursor Backward Tab not implemented");
            }
            'g' => {
                // TBC (Tab Clear) - Clear tab stops (0=current, 3=all)
                // Not needed: Tab stops are application-specific, TUI apps manage their
                // own. See [mod-level docs](crate::vt_100_ansi_parser) for rationale
                tracing::warn!("CSI g: Tab Clear not implemented");
            }
            'a' => {
                // HPR (Horizontal Position Relative) - Same as CUF (Cursor Forward)
                // Not needed: CUF already implemented, this is redundant
                // See [mod-level docs](crate::vt_100_ansi_parser) for rationale
                tracing::warn!(
                    "CSI a: Horizontal Position Relative not implemented (use CUF instead)"
                );
            }
            'e' => {
                // VPR (Vertical Position Relative) - Same as CUD (Cursor Down)
                // Not needed: CUD already implemented, this is redundant
                // See [mod-level docs](crate::vt_100_ansi_parser) for rationale
                tracing::warn!(
                    "CSI e: Vertical Position Relative not implemented (use CUD instead)"
                );
            }
            '`' => {
                // HPA (Horizontal Position Absolute) - Same as CHA
                // Not needed: CHA already implemented, this is redundant
                // See [mod-level docs](crate::vt_100_ansi_parser) for rationale
                tracing::warn!(
                    "CSI `: Horizontal Position Absolute not implemented (use CHA instead)"
                );
            }
            'U' => {
                // NP (Next Page) - Move to next page in page memory
                // Not needed: Page memory not supported in multiplexer.
                // See [mod-level docs](crate::vt_100_ansi_parser) for rationale
                tracing::warn!("CSI U: Next Page not supported in multiplexer");
            }
            'V' => {
                // PP (Preceding Page) - Move to previous page in page memory
                // Not needed: Page memory not supported in multiplexer.
                // See [mod-level docs](crate::vt_100_ansi_parser) for rationale
                tracing::warn!("CSI V: Preceding Page not supported in multiplexer");
            }
            '~' => {
                // DECLL (DEC Load LEDs) - Set keyboard LED indicators
                // Not needed: Hardware control not applicable in multiplexer.
                // See [mod-level docs](crate::vt_100_ansi_parser) for rationale
                tracing::warn!("CSI ~: DEC Load LEDs not supported in multiplexer");
            }
            '}' => {
                // DECIC (DEC Insert Column) - Insert blank columns at cursor
                // Not needed: Column insertion rarely used, complex for TUI apps
                // See [mod-level docs](crate::vt_100_ansi_parser) for rationale
                tracing::warn!("CSI }}: DEC Insert Column not implemented");
            }
            '|' => {
                // DECDC (DEC Delete Column) - Delete columns at cursor
                // Not needed: Column deletion rarely used, complex for TUI apps
                // See [mod-level docs](crate::vt_100_ansi_parser) for rationale
                tracing::warn!("CSI |: DEC Delete Column not implemented");
            }
            't' => {
                // Window manipulation (resize, move, iconify, etc.)
                // Not needed: Window ops handled by terminal emulator, not multiplexer
                // See [mod-level docs](crate::vt_100_ansi_parser) for rationale
                tracing::warn!("CSI t: Window manipulation not supported in multiplexer");
            }
            'c' => {
                // DA (Device Attributes) - Request terminal type/capabilities
                // Not needed: Multiplexer doesn't respond to queries, parent terminal
                // does. See [mod-level docs](crate::vt_100_ansi_parser) for rationale
                tracing::warn!(
                    "CSI c: Device Attributes query not supported in multiplexer"
                );
            }
            'q' => {
                // DECSCUSR (Set Cursor Style) - Change cursor shape/blink
                // Not needed: Cursor rendering handled by terminal emulator.
                // See [mod-level docs](crate::vt_100_ansi_parser) for rationale
                tracing::warn!("CSI q: Set Cursor Style not supported in multiplexer");
            }
            'p' => {
                // Various DEC private sequences (DECRQM, etc.)
                // Not needed: Private mode requests handled by parent terminal.
                // See [mod-level docs](crate::vt_100_ansi_parser) for rationale
                tracing::warn!(
                    "CSI p: DEC private sequences not supported in multiplexer"
                );
            }
            'x' => {
                // DECREQTPARM (Request Terminal Parameters) - Request terminal settings
                // Not needed: Terminal parameters managed by parent emulator.
                // See [mod-level docs](crate::vt_100_ansi_parser) for rationale
                tracing::warn!(
                    "CSI x: Request Terminal Parameters not supported in multiplexer"
                );
            }
            'z' => {
                // DECERA/DECSERA (DEC Erase/Selective Erase Rectangular Area)
                // Not needed: Rectangular operations complex, rarely used
                // See [mod-level docs](crate::vt_100_ansi_parser) for rationale
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
    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
        vt_100_shim_osc_ops::dispatch_osc(self, params, bell_terminated);
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
    ///
    /// ## Malformed Sequence Handling
    ///
    /// The `ignore` parameter signals when the VTE parser exceeded its limits during
    /// parsing (too many intermediate bytes). When `ignore = true`, this method
    /// **discards the entire sequence** to prevent executing commands with incorrect
    /// parameters.
    ///
    /// See the [module-level
    /// documentation](self#malformed-sequences-and-the-ignore-parameter) for detailed
    /// explanation of why discarding malformed sequences is the correct
    /// approach, with references to industry best practices.
    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
        // Discard malformed sequences - see module docs for rationale
        if ignore {
            tracing::warn!(
                "ESC {}: Discarding malformed sequence (VTE parser exceeded limits)",
                byte as char
            );
            return;
        }

        match byte {
            DECSC_SAVE_CURSOR => {
                // DECSC - Save current cursor position.
                vt_100_shim_cursor_ops::save_cursor_position(self);
            }
            DECRC_RESTORE_CURSOR => {
                // DECRC - Restore saved cursor position.
                vt_100_shim_cursor_ops::restore_cursor_position(self);
            }
            IND_INDEX_DOWN => {
                // IND - Index (move down one line, scroll if at bottom).
                vt_100_shim_scroll_ops::index_down(self);
            }
            RI_REVERSE_INDEX_UP => {
                // RI - Reverse Index (move up one line, scroll if at top).
                vt_100_shim_scroll_ops::reverse_index_up(self);
            }
            RIS_RESET_TERMINAL => {
                // RIS - Reset to Initial State.
                vt_100_shim_terminal_ops::reset_terminal(self);
            }
            _ if intermediates == G0_CHARSET_INTERMEDIATE => {
                // Character set selection G0.
                match byte {
                    CHARSET_ASCII => {
                        // Select ASCII character set (normal mode).
                        vt_100_shim_terminal_ops::select_ascii_character_set(self);
                    }
                    CHARSET_DEC_GRAPHICS => {
                        // Select DEC Special Graphics character set.
                        // This enables box-drawing characters.
                        vt_100_shim_terminal_ops::select_dec_graphics_character_set(self);
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
    ///
    /// ## Malformed Sequence Handling
    ///
    /// The `ignore` parameter is not checked in this method because DCS sequences
    /// are not implemented at all in this multiplexer. Both well-formed and malformed
    /// DCS sequences result in the same action: do nothing. The `ignore` flag is only
    /// meaningful when deciding whether to process a sequence.
    ///
    /// See the [module-level
    /// documentation](self#malformed-sequences-and-the-ignore-parameter)
    /// for information about how `ignore` is used in `csi_dispatch()` and
    /// `esc_dispatch()`, where it affects the decision to process or discard
    /// sequences.
    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _c: char) {
        // All DCS sequences are ignored - not implemented in terminal multiplexing
    }

    /// Handle DCS data by continuing to receive bytes for an active DCS sequence started
    /// by hook.
    ///
    /// This method receives the actual data payload of a DCS sequence. For terminal
    /// multiplexing, DCS sequences are not processed, so this data is ignored.
    fn put(&mut self, _byte: u8) {
        // Ignore DCS data.
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
