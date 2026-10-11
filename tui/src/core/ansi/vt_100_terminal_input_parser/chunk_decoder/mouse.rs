// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Mouse input event [1-based coordinates] parsing from [`ANSI`]/[`CSI`] sequences.
//!
//! This module handles conversion of mouse-related [`ANSI`] escape sequences into mouse
//! events, including support for multiple mouse protocols.
//!
//! ## Where You Are in the Pipeline
//!
//! For the full data flow, see the [parent module documentation]. This diagram shows
//! where `mouse.rs` fits:
//!
//! ```text
//! MioPollWorker (reads stdin into read_buffer)
//!    │
//!    │ ChunkFramer::process_incoming_bytes(read_buffer, maybe_more)
//!    ▼
//! chunk_decoder (try_decode_input_event)
//!    │ (routes mouse sequences here)
//! ┌──▼───────────────────────────────────────┐  ┌──────────────────┐
//! │  mouse.rs                                ◄──┤ **YOU ARE HERE** │
//! │  • Parse `SGR` protocol (modern)         │  └──────────────────┘
//! │  • Parse `X10`/Legacy (legacy)           │
//! │  • Parse `RXVT` protocol (legacy)        │
//! │  • Detect clicks/drags/scroll/motion     │
//! │  • Extract position & modifiers          │
//! └──────────────────────────────────────────┘
//!    │
//!    ▼
//! VT100InputEventIR::Mouse { button, pos, action, modifiers }
//!    │
//!    ▼
//! convert_input_event() → InputEvent (returned to application)
//! ```
//!
//! **Navigate**:
//! - ⬆️ **Up**: [`chunk_decoder`] - Main sequence decoding entry point
//! - ➡️ **Peer**: [`keyboard`], [`terminal_events`], [`utf8`] - Other specialized
//!   decoders
//! - 📚 **Types**: [`VT100MouseButtonIR`], [`VT100MouseActionIR`], [`TermPos`]
//! - 📤 **Converted by**: [`convert_input_event()`] in `protocol_conversion.rs` (not this
//!   module)
//!
//! ## Supported Mouse Protocols
//! - **[`SGR`] (Selective Graphic Rendition) Protocol**: Modern standard format
//! - Format: `ESC [ < Cb ; Cx ; Cy M / m`
//! - Button detection (left=0, middle=1, right=2)
//! - Drag detection (button with flag 32)
//! - Scroll events (buttons 64/65 for vertical, 66/67 for horizontal)
//! - **[`X10`]/Legacy Protocol**: Legacy formats (Format: `ESC [ M Cb Cx Cy`)
//! - **[`RXVT`] Protocol**: Alternative legacy format (Format: `ESC [ Cb ; Cx ; Cy M`)
//! - **Click Events**: Press (M) and Release (m)
//! - **Drag Events**: Motion while button held
//! - **Motion Events**: Movement without buttons
//! - **Modifier Keys**: Shift, Ctrl, Alt detection
//!
//! # Life Of A Mouse Event
//!
//! Let's deep dive into how Motion Events (aka tracking mouse hover movement) work; this
//! is applicable for other mouse events as well.
//!
//! 1. **Full TUI Setup & Terminal Awareness:** The user opens a terminal emulator app
//!    (e.g., [`WezTerm`]), and runs a full-TUI app. The app boots via
//!    [`crate::tui::TerminalWindow::main_event_loop()`]. The `r3bl_tui` framework
//!    automatically puts the terminal in [Raw Mode], spins up the [Resilient Reactor
//!    Thread] (RRT) for [`mio`], and emits [`ANSI`] sequences like `ESC[ ? 1003h` (Enable
//!    Any-Event Mouse Tracking) to [`stdout`], which tells the terminal emulator app that
//!    we want to hover coordinates sent back via [`stdin`].
//! 2. **Physical Action:** A user moves their mouse or touchpad or trackball or
//!    trackpoint, specifically "hover-moving" over the terminal emulator window running
//!    our full TUI app.
//! 3. **OS Routing:** The OS (via Wayland) determines the terminal emulator window has
//!    focus and fires a UI event (using whatever UI toolkit the emulator is written in).
//! 4. **[`ANSI`] Serialization:** The terminal packs the _buttons and modifiers_ into a
//!    single byte payload using a bitwise OR mask. Here are the mappings:
//!    - Bits 0-1 define the button (0=Left, 1=Middle, 2=Right, 3=Release/Unknown)
//!    - Bit 2 (4) is Shift
//!    - Bit 3 (8) is Alt
//!    - Bit 4 (16) is Ctrl
//!    - Bit 5 (32) is the Motion flag
//!
//!    The terminal emulator app then takes this payload byte, along with the X/Y
//!    coordinates, and converts them all into a human-readable [`ASCII`] text string
//!    (e.g., `ESC[ < 35 ; 12 ; 24M`). Here's the binary math for how the `35` payload
//!    byte only is calculated, using big endian notation (most significant bit first),
//!    and using 0-indexed bit positions (meaning Bit 5 is the 6th bit from the right,
//!    scanning right to left):
//!
//!      ```text
//!      `76543210` - Bit positions
//!      `00100000` (Decimal `32`) : Motion Flag (Bit 5 is set)
//!      `00000011` (Decimal `3`) : Unknown Button (Bits 0 and 1 are set)
//!      `--------`
//!      `00100011` (Decimal `35`) : Final payload byte (`32 | 3`)
//!      ```
//! 5. **Process Delivery:** The terminal emulator app writes this string to the [`stdin`]
//!    of our TUI app's process.
//! 6. **Event Loop:** Our asynchronous [`mio`] event loop (running inside
//!    [`mio_poll_worker`]) reads these bytes and routes them into this file (`mouse.rs`).
//! 7. **Parsing:** The parser identifies it as an [`SGR`] [`1006`] mouse event and safely
//!    unpacks the `35` payload byte into an [`Unknown`] button with a motion flag.
//! 8. **Type-Safe Delivery:** It delivers a clean, type-safe `InputEvent::Mouse(MouseMove
//!    { x: 12, y: 24 })` through the framework directly into the developer's
//!    [`App::app_handle_input_event()`] implementation!
//!
//! # Verifying Coordinate Systems
//!
//! **[`VT-100`] mouse coordinates are 1-based**, where (1, 1) represents the top-left
//! corner. This was confirmed through ground truth discovery via the validation tests,
//! which capture raw bytes from actual terminal interactions. For details on how this was
//! verified, see the [parent module's testing strategy documentation].
//!
//! # Terminal Limitations
//!
//! ## Shift+Click Not Reported
//!
//! Most terminal emulators intercept **Shift+Click** combinations for their own use (text
//! selection, block selection, etc.) and never report these events to the application.
//! This is a terminal-level limitation, not an issue with this parser.
//!
//! **Affected combinations:**
//! - Shift+Click
//! - Ctrl+Shift+Click
//! - Ctrl+Alt+Shift+Click
//!
//! **Working combinations:**
//! - Ctrl+Click ✓
//! - Alt+Click ✓
//! - Alt+Ctrl+Click ✓
//!
//! This limitation is consistent across most terminal emulators ([`xterm`],
//! [`gnome-terminal`], [`iTerm2`], etc.) because Shift+Click is reserved for text
//! selection by the terminal. See the test fixtures for mouse event generation details
//! and validation tests.
//!
//! [1-based coordinates]: #verifying-coordinate-systems
//! [`1006`]: crate::core::ansi::SGR_MOUSE_MODE
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`App::app_handle_input_event()`]: crate::tui::App::app_handle_input_event
//! [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
//! [`chunk_decoder`]: mod@super
//! [`convert_input_event()`]: crate::direct_to_ansi::input::protocol_conversion::convert_input_event
//! [`crate::tui::TerminalWindow::main_event_loop()`]: crate::tui::TerminalWindow::main_event_loop
//! [`CSI`]: crate::CsiSequence
//! [`gnome-terminal`]: https://en.wikipedia.org/wiki/GNOME_Terminal
//! [`iTerm2`]: https://iterm2.com/
//! [`keyboard`]: mod@super::keyboard
//! [`mio_poll_worker`]: crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::mio_poll_worker
//! [`mio`]: mio
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`RXVT`]: https://en.wikipedia.org/wiki/Rxvt
//! [`SGR`]: crate::SgrCode
//! [`stdin`]: std::io::stdin
//! [`stdout`]: std::io::stdout
//! [`terminal_events`]: mod@super::terminal_events
//! [`TermPos`]: crate::vt_100_ansi_coords::TermPos
//! [`Unknown`]: crate::core::ansi::vt_100_terminal_input_parser::VT100MouseButtonIR::Unknown
//! [`utf8`]: mod@super::utf8
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [`VT100MouseActionIR`]: crate::core::ansi::vt_100_terminal_input_parser::VT100MouseActionIR
//! [`VT100MouseButtonIR`]: crate::core::ansi::vt_100_terminal_input_parser::VT100MouseButtonIR
//! [`WezTerm`]: https://wezfurlong.org/wezterm/
//! [`X10`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
//! [`xterm`]: https://en.wikipedia.org/wiki/Xterm
//! [parent module documentation]: mod@crate::vt_100_terminal_input_parser
//! [parent module's testing strategy documentation]: mod@crate::vt_100_terminal_input_parser#testing-strategy
//! [Raw Mode]: crate::core::ansi::terminal_raw_mode
//! [Resilient Reactor Thread]: crate::core::resilient_reactor_thread

use super::{super::ir_event_types::{ParsedInputEventIR, VT100InputEventIR,
                                    VT100KeyModifiersIR, VT100MouseActionIR,
                                    VT100MouseButtonIR, VT100ScrollDirectionIR},
            csi_scanner::{parse_decimal_digits, strip_csi_numeric_prefix}};
use crate::{ByteOffset, KeyState, NarrowingCastToU16, TermPos, WideningCastToU16,
            byte_offset,
            core::ansi::constants::{ANSI_CSI_BRACKET, ANSI_ESC, ANSI_PARAM_SEPARATOR,
                                    CSI_PREFIX_LEN, MOUSE_BASE_BUTTON_MASK,
                                    MOUSE_BUTTON_BITS_MASK, MOUSE_LEFT_BUTTON_CODE,
                                    MOUSE_MIDDLE_BUTTON_CODE, MOUSE_MODIFIER_ALT,
                                    MOUSE_MODIFIER_CTRL, MOUSE_MODIFIER_SHIFT,
                                    MOUSE_MOTION_FLAG, MOUSE_RIGHT_BUTTON_CODE,
                                    MOUSE_RXVT_MIN_LEN, MOUSE_SCROLL_DOWN_BUTTON,
                                    MOUSE_SCROLL_LEFT_BUTTON,
                                    MOUSE_SCROLL_RIGHT_BUTTON, MOUSE_SCROLL_THRESHOLD,
                                    MOUSE_SCROLL_UP_BUTTON, MOUSE_SGR_MARKER,
                                    MOUSE_SGR_MIN_LEN, MOUSE_SGR_PREFIX_LEN,
                                    MOUSE_SGR_PRESS, MOUSE_SGR_RELEASE,
                                    MOUSE_X10_COORD_OFFSET, MOUSE_X10_MARKER,
                                    MOUSE_X10_MIN_LEN}};

/// Parse terminal mouse sequence from input buffer.
///
/// Dispatches across the three supported terminal mouse tracking protocols:
/// 1. [`SGR`] (preferred & most reliable): `CSI < Cb ; Cx ; Cy M/m`
/// 2. [`X10`]/Legacy: `CSI M Cb Cx Cy`
/// 3. [`RXVT`]: `CSI Cb ; Cx ; Cy M`
///
/// # Returns
///
/// - `Some(ParsedInputEventIR)` on success.
/// - `None` if the sequence is incomplete, unrecognized, or not a mouse sequence.
///
/// [`RXVT`]: https://en.wikipedia.org/wiki/Rxvt
/// [`SGR`]: crate::SgrCode
/// [`X10`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
#[must_use]
pub fn parse_mouse_sequence(buffer: &[u8]) -> Option<ParsedInputEventIR> {
    match buffer {
        // 1. Check for SGR mouse protocol (most reliable).
        // SGR sequence: `ESC [ < Cb ; Cx ; Cy M/m`.
        [ANSI_ESC, ANSI_CSI_BRACKET, MOUSE_SGR_MARKER, ..] => sgr::parse(buffer),

        // 2. Check for X10/Legacy protocol (legacy).
        // X10 sequence: `ESC [ M Cb Cx Cy`.
        [ANSI_ESC, ANSI_CSI_BRACKET, MOUSE_X10_MARKER, ..] => legacy::parse_x10(buffer),

        // 3. Check for RXVT protocol (legacy alternative).
        // RXVT format: `ESC [ Cb ; Cx ; Cy M`.
        [ANSI_ESC, ANSI_CSI_BRACKET, ..] => legacy::parse_rxvt(buffer),

        _ => None,
    }
}

mod sgr {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Parse [`SGR`] mouse protocol: `CSI < Cb ; Cx ; Cy M/m`
    ///
    /// # Returns
    ///
    /// - `Some(ParsedInputEventIR)` on success.
    /// - `None` if the sequence is incomplete.
    ///
    /// Format breakdown:
    /// - `CSI <` prefix (3 bytes, equivalent to `ESC [ <`)
    /// - `Cb` = button byte (with modifiers encoded)
    /// - `Cx` = column (1-based)
    /// - `Cy` = row (1-based)
    /// - `M` = press, `m` = release
    ///
    /// [`SGR`]: crate::SgrCode
    pub fn parse(chunk: &[u8]) -> Option<ParsedInputEventIR> {
        // Minimum: ESC[<0;1;1M (9 bytes).
        if chunk.len() < MOUSE_SGR_MIN_LEN {
            return None;
        }

        // Find the terminator ('M' for press, 'm' for release).
        let (terminator_idx, term_byte) = chunk
            .iter()
            .copied()
            .enumerate()
            .skip(MOUSE_SGR_PREFIX_LEN)
            .find(|&(_idx, byte)| byte == MOUSE_SGR_PRESS || byte == MOUSE_SGR_RELEASE)?;
        let bytes_consumed = byte_offset(terminator_idx + 1);

        // Parse the payload between `ESC[<` and `M/m`: `Cb;Cx;Cy`.
        let payload = chunk.get(MOUSE_SGR_PREFIX_LEN..terminator_idx)?;
        let (part_button_byte, part_cx, part_cy) =
            helpers::parse_semicolon_triplet(payload)?;

        let modifiers = helpers::extract_modifiers(part_button_byte);

        // 1. Check for scroll events first (buttons 64-67).
        if let Some(scroll_dir) = helpers::detect_scroll_event(part_button_byte) {
            return Some(ParsedInputEventIR::new(
                VT100InputEventIR::Mouse {
                    button: VT100MouseButtonIR::Unknown,
                    pos: TermPos::from_one_based(part_cx, part_cy),
                    action: VT100MouseActionIR::Scroll(scroll_dir),
                    modifiers,
                },
                bytes_consumed,
            ));
        }

        // 2. Detect button and action for clicks, drags, and motion.
        let button = helpers::detect_mouse_button(part_button_byte);
        let is_motion = helpers::is_motion_event(part_button_byte);
        let is_press = term_byte == MOUSE_SGR_PRESS;

        // The SGR protocol encodes Press vs Release via the terminator character ('M' vs
        // 'm'), while the button byte encodes motion state and button identity. We
        // combine all three of these dimensions to accurately resolve the final
        // mouse action.
        let action = match (is_motion, is_press, button) {
            // Moving without a button held is a hover (Motion).
            (true, _, VT100MouseButtonIR::Unknown) => VT100MouseActionIR::Motion,
            // Moving with a button held is a Drag.
            (true, _, _) => VT100MouseActionIR::Drag,
            // Not moving, uppercase 'M' is Press.
            (false, true, _) => VT100MouseActionIR::Press,
            // Not moving, lowercase 'm' is Release.
            (false, false, _) => VT100MouseActionIR::Release,
        };

        Some(ParsedInputEventIR::new(
            VT100InputEventIR::Mouse {
                button,
                pos: TermPos::from_one_based(part_cx, part_cy),
                action,
                modifiers,
            },
            bytes_consumed,
        ))
    }
}

mod legacy {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Parse [`X10`]/Legacy mouse protocol: `CSI M Cb Cx Cy`.
    ///
    /// # Returns
    ///
    /// - `Some(ParsedInputEventIR)` on success.
    /// - `None` if the sequence is incomplete.
    ///
    /// Format breakdown:
    /// - `CSI M` prefix (3 bytes, equivalent to `ESC [ M`)
    /// - `Cb` = button byte (bits 0-1: button, bits 2-4: modifiers, bit 5: motion)
    /// - `Cx` = column byte (raw value - 32 = 1-based column position)
    /// - `Cy` = row byte (raw value - 32 = 1-based row position)
    /// - Positions 33-255 represent columns/rows 1-223
    ///
    /// Button encoding (bits 0-1):
    /// - 0 = left button
    /// - 1 = middle button
    /// - 2 = right button
    /// - 3 = release (no button held)
    ///
    /// Modifier encoding (bits 2-4):
    /// - Bit 2 (value 4): Shift
    /// - Bit 3 (value 8): Alt
    /// - Bit 4 (value 16): Ctrl
    ///
    /// Motion flag (bit 5, value 32): Set when mouse moved without button press
    ///
    /// [`X10`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
    pub fn parse_x10(sequence: &[u8]) -> Option<ParsedInputEventIR> {
        // X10 format: `ESC [ M Cb Cx Cy` (6 bytes minimum).
        let [
            ANSI_ESC,
            ANSI_CSI_BRACKET,
            MOUSE_X10_MARKER,
            button_byte,
            part_cx,
            part_cy,
            ..,
        ] = sequence
        else {
            return None;
        };

        let part_button_byte = button_byte.as_u16_widening(); // Widen to u16 for consistent constant usage.

        // Convert raw bytes to 1-based coordinates.
        // X10 encoding: byte value - 32 = position (with offset for positions > 95).
        // Positions are 1-based in the terminal.
        let col = part_cx
            .as_u16_widening()
            .saturating_sub(MOUSE_X10_COORD_OFFSET);
        let row = part_cy
            .as_u16_widening()
            .saturating_sub(MOUSE_X10_COORD_OFFSET);

        // Handle invalid coordinates.
        if col == 0 || row == 0 {
            return None;
        }

        Some(parse_legacy_mouse_event(
            part_button_byte,
            col,
            row,
            byte_offset(MOUSE_X10_MIN_LEN),
        ))
    }

    /// Parse [`RXVT`] mouse protocol: `CSI Cb ; Cx ; Cy M`.
    ///
    /// # Returns
    ///
    /// - `Some(ParsedInputEventIR)` on success.
    /// - `None` if the sequence is incomplete.
    ///
    /// Format breakdown:
    /// - [`CSI`] prefix (2 bytes, equivalent to `ESC [`)
    /// - `Cb` = button code ([`ASCII`] digits, semicolon-separated)
    /// - `Cx` = column ([`ASCII`] digits, semicolon-separated)
    /// - `Cy` = row ([`ASCII`] digits, semicolon-separated)
    /// - `M` = terminator (always uppercase, no lowercase 'm')
    ///
    /// Button encoding (similar to [`X10`]):
    /// - 0 = left button
    /// - 1 = middle button
    /// - 2 = right button
    /// - 3 = release (no button held)
    /// - Add 4 for shift, 8 for alt, 16 for ctrl (like [`X10`])
    /// - Add 32 for motion (mouse moved)
    ///
    /// Similar to [`SGR`] but simpler - no `<` prefix, only M terminator (no m),
    /// and always includes coordinates as decimal numbers.
    ///
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
    /// [`CSI`]: crate::CsiSequence
    /// [`RXVT`]: https://en.wikipedia.org/wiki/Rxvt
    /// [`SGR`]: crate::SgrCode
    /// [`X10`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
    pub fn parse_rxvt(chunk: &[u8]) -> Option<ParsedInputEventIR> {
        // RXVT format: ESC [ Cb ; Cx ; Cy M (minimum 8 bytes: ESC[0;1;1M).
        if chunk.len() < MOUSE_RXVT_MIN_LEN {
            return None;
        }

        let payload = strip_csi_numeric_prefix(chunk)?;

        // Find the terminator 'M'.
        let m_pos = payload.iter().position(|&byte| byte == MOUSE_X10_MARKER)?;
        let bytes_consumed = byte_offset(CSI_PREFIX_LEN + m_pos + 1);

        // Parse the payload between ESC[ and M: "Cb;Cx;Cy".
        let payload_bytes = payload.get(..m_pos)?;
        let (part_button_byte, part_cx, part_cy) =
            helpers::parse_semicolon_triplet(payload_bytes)?;

        Some(parse_legacy_mouse_event(
            part_button_byte,
            part_cx,
            part_cy,
            bytes_consumed,
        ))
    }

    /// Helper to construct mouse events for legacy protocols ([`X10`] and [`RXVT`])
    /// which rely purely on the button byte for action detection.
    ///
    /// [`RXVT`]: https://en.wikipedia.org/wiki/Rxvt
    /// [`X10`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
    fn parse_legacy_mouse_event(
        button_byte: u16,
        col: u16,
        row: u16,
        bytes_consumed: ByteOffset,
    ) -> ParsedInputEventIR {
        let modifiers = helpers::extract_modifiers(button_byte);
        let pos = TermPos::from_one_based(col, row);

        // Check for scroll events first (buttons 64-67).
        if let Some(scroll_dir) = helpers::detect_scroll_event(button_byte) {
            return ParsedInputEventIR::new(
                VT100InputEventIR::Mouse {
                    button: VT100MouseButtonIR::Unknown,
                    pos,
                    action: VT100MouseActionIR::Scroll(scroll_dir),
                    modifiers,
                },
                bytes_consumed,
            );
        }

        // Detect button type.
        let button = helpers::detect_mouse_button(button_byte);

        // Detect action.
        // Unlike SGR, legacy formats rely purely on the button byte to indicate both the
        // physical button identity and the action type (Motion, Press, Release).
        let is_motion = helpers::is_motion_event(button_byte);
        let action = match (is_motion, button) {
            // Moving without a button held is a hover (Motion).
            (true, VT100MouseButtonIR::Unknown) => VT100MouseActionIR::Motion,
            // Moving with a button held is a Drag.
            (true, _) => VT100MouseActionIR::Drag,
            // Not moving, Unknown button indicates Release.
            (false, VT100MouseButtonIR::Unknown) => VT100MouseActionIR::Release,
            // Not moving, valid button indicates Press.
            (false, _) => VT100MouseActionIR::Press,
        };

        ParsedInputEventIR::new(
            VT100InputEventIR::Mouse {
                button,
                pos,
                action,
                modifiers,
            },
            bytes_consumed,
        )
    }
}

mod helpers {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Helper to parse decimal coordinate triplet `"Cb;Cx;Cy"` from [`SGR`] or [`RXVT`]
    /// payloads into `(button_code, col, row)` using [`parse_decimal_digits()`][digits].
    ///
    /// # Returns
    ///
    /// - `Some((button_code, col, row))` if all three fields are valid [`u16`] numbers.
    /// - `None` if fewer than 3 fields exist or any field fails parsing.
    ///
    /// [`RXVT`]: https://en.wikipedia.org/wiki/Rxvt
    /// [`SGR`]: crate::SgrCode
    /// [digits]: crate::vt_100_terminal_input_parser::csi_scanner::parse_decimal_digits
    pub fn parse_semicolon_triplet(payload: &[u8]) -> Option<(u16, u16, u16)> {
        let mut parts = payload.split(|&byte| byte == ANSI_PARAM_SEPARATOR);
        let cb = parse_decimal_digits(parts.next()?)?.as_u16_narrowing();
        let cx = parse_decimal_digits(parts.next()?)?.as_u16_narrowing();
        let cy = parse_decimal_digits(parts.next()?)?.as_u16_narrowing();
        Some((cb, cx, cy))
    }

    /// Detects mouse button from [`SGR`] button byte.
    ///
    /// Button encoding (bits 0-1):
    /// - [`MOUSE_LEFT_BUTTON_CODE`] (`0`) = left button
    /// - [`MOUSE_MIDDLE_BUTTON_CODE`] (`1`) = middle button
    /// - [`MOUSE_RIGHT_BUTTON_CODE`] (`2`) = right button
    /// - [`MOUSE_RELEASE_BUTTON_CODE`] (`3`) = release (for legacy modes, [`SGR`] uses
    ///   'M'/'m' instead)
    ///
    /// # Arguments
    ///
    /// - `button_byte`: Button byte (with modifiers encoded).
    ///
    /// [`MOUSE_LEFT_BUTTON_CODE`]: crate::MOUSE_LEFT_BUTTON_CODE
    /// [`MOUSE_MIDDLE_BUTTON_CODE`]: crate::MOUSE_MIDDLE_BUTTON_CODE
    /// [`MOUSE_RELEASE_BUTTON_CODE`]: crate::MOUSE_RELEASE_BUTTON_CODE
    /// [`MOUSE_RIGHT_BUTTON_CODE`]: crate::MOUSE_RIGHT_BUTTON_CODE
    /// [`SGR`]: crate::SgrCode
    pub fn detect_mouse_button(button_byte: u16) -> VT100MouseButtonIR {
        match button_byte & MOUSE_BUTTON_BITS_MASK {
            MOUSE_LEFT_BUTTON_CODE => VT100MouseButtonIR::Left,
            MOUSE_MIDDLE_BUTTON_CODE => VT100MouseButtonIR::Middle,
            MOUSE_RIGHT_BUTTON_CODE => VT100MouseButtonIR::Right,
            _ => VT100MouseButtonIR::Unknown,
        }
    }

    /// Detects if mouse event is a motion event (moving).
    ///
    /// Motion flag is bit 5 (value 32, [`MOUSE_MOTION_FLAG`]) in the button byte.
    ///
    /// # Arguments
    ///
    /// - `button_byte`: Button byte (with modifiers encoded).
    ///
    /// [`MOUSE_MOTION_FLAG`]: crate::MOUSE_MOTION_FLAG
    pub fn is_motion_event(button_byte: u16) -> bool {
        (button_byte & MOUSE_MOTION_FLAG) != 0
    }

    /// Detects scroll events (up/down/left/right).
    ///
    /// Scroll button codes:
    /// - [`MOUSE_SCROLL_UP_BUTTON`] (`64`) = scroll up
    /// - [`MOUSE_SCROLL_DOWN_BUTTON`] (`65`) = scroll down
    /// - [`MOUSE_SCROLL_LEFT_BUTTON`] (`66`) = scroll left (rare) - but often used for
    ///   scroll up with modifiers!
    /// - [`MOUSE_SCROLL_RIGHT_BUTTON`] (`67`) = scroll right (rare)
    ///
    /// # Arguments
    ///
    /// - `button_byte`: Button byte (with modifiers encoded).
    ///
    /// [`MOUSE_SCROLL_DOWN_BUTTON`]: crate::MOUSE_SCROLL_DOWN_BUTTON
    /// [`MOUSE_SCROLL_LEFT_BUTTON`]: crate::MOUSE_SCROLL_LEFT_BUTTON
    /// [`MOUSE_SCROLL_RIGHT_BUTTON`]: crate::MOUSE_SCROLL_RIGHT_BUTTON
    /// [`MOUSE_SCROLL_UP_BUTTON`]: crate::MOUSE_SCROLL_UP_BUTTON
    pub fn detect_scroll_event(button_byte: u16) -> Option<VT100ScrollDirectionIR> {
        // Check raw button code first (before masking modifiers).
        // Buttons 64+ indicate scroll events.
        if button_byte < MOUSE_SCROLL_THRESHOLD {
            return None;
        }

        // Mask to get base button (without modifiers but keeping scroll bit).
        let base_button = button_byte & MOUSE_BASE_BUTTON_MASK; // Keep bit 6 (value 64).

        #[allow(clippy::match_same_arms)]
        match base_button {
            MOUSE_SCROLL_UP_BUTTON => Some(VT100ScrollDirectionIR::Up),
            MOUSE_SCROLL_DOWN_BUTTON => Some(VT100ScrollDirectionIR::Down),
            MOUSE_SCROLL_LEFT_BUTTON => Some(VT100ScrollDirectionIR::Left),
            MOUSE_SCROLL_RIGHT_BUTTON => Some(VT100ScrollDirectionIR::Right),
            _ => Some(VT100ScrollDirectionIR::Up), // Default fallback.
        }
    }

    /// Extracts modifier keys (Shift, Ctrl, Alt) from [`SGR`] sequence.
    ///
    /// Modifier encoding (bits 2-4):
    /// - Bit 2 (value 4, [`MOUSE_MODIFIER_SHIFT`]): Shift
    /// - Bit 3 (value 8, [`MOUSE_MODIFIER_ALT`]): Alt
    /// - Bit 4 (value 16, [`MOUSE_MODIFIER_CTRL`]): Ctrl
    ///
    /// # Arguments
    ///
    /// - `button_byte`: Button byte (with modifiers encoded).
    ///
    /// [`MOUSE_MODIFIER_ALT`]: crate::MOUSE_MODIFIER_ALT
    /// [`MOUSE_MODIFIER_CTRL`]: crate::MOUSE_MODIFIER_CTRL
    /// [`MOUSE_MODIFIER_SHIFT`]: crate::MOUSE_MODIFIER_SHIFT
    /// [`SGR`]: crate::SgrCode
    pub fn extract_modifiers(button_byte: u16) -> VT100KeyModifiersIR {
        VT100KeyModifiersIR {
            shift: if (button_byte & MOUSE_MODIFIER_SHIFT) != 0 {
                KeyState::Pressed
            } else {
                KeyState::NotPressed
            },
            alt: if (button_byte & MOUSE_MODIFIER_ALT) != 0 {
                KeyState::Pressed
            } else {
                KeyState::NotPressed
            },
            ctrl: if (button_byte & MOUSE_MODIFIER_CTRL) != 0 {
                KeyState::Pressed
            } else {
                KeyState::NotPressed
            },
        }
    }
}

/// Unit tests for mouse input parsing.
///
/// These tests use generator functions instead of hardcoded magic strings to ensure
/// consistency between sequence generation and parsing. For testing strategy details,
/// see the [testing strategy] documentation.
///
/// [testing strategy]: mod@crate::vt_100_terminal_input_parser#testing-strategy
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ansi::{constants::{ANSI_CSI_BRACKET, ANSI_ESC, CONTROL_NUL},
                            generator::generate_keyboard_sequence};

    // ==================== Test Helpers ====================

    /// Builds an [`X10`] mouse sequence using the generator.
    ///
    /// [`X10`] format: `ESC [ M Cb Cx Cy` (6 bytes with null terminator)
    ///
    /// [`X10`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
    fn x10_mouse_sequence(
        button: VT100MouseButtonIR,
        col: u16,
        row: u16,
        action: VT100MouseActionIR,
        modifiers: VT100KeyModifiersIR,
    ) -> Vec<u8> {
        use crate::core::ansi::generator::generate_x10_mouse_sequence;
        generate_x10_mouse_sequence(button, col, row, action, modifiers)
    }

    /// Builds an [`RXVT`] mouse sequence using the generator.
    ///
    /// [`RXVT`] format: `ESC [ Cb ; Cx ; Cy M` (decimal with semicolons)
    ///
    /// [`RXVT`]: https://en.wikipedia.org/wiki/Rxvt
    fn rxvt_mouse_sequence(
        button: VT100MouseButtonIR,
        col: u16,
        row: u16,
        action: VT100MouseActionIR,
        modifiers: VT100KeyModifiersIR,
    ) -> Vec<u8> {
        use crate::core::ansi::generator::generate_rxvt_mouse_sequence;
        generate_rxvt_mouse_sequence(button, col, row, action, modifiers)
    }

    /// Builds an [`SGR`] mouse sequence using the generator.
    ///
    /// [`SGR`] format: `ESC [ < Cb ; Cx ; Cy M/m` (modern standard)
    ///
    /// [`SGR`]: crate::SgrCode
    fn sgr_mouse_sequence(
        button: VT100MouseButtonIR,
        col: u16,
        row: u16,
        action: VT100MouseActionIR,
        modifiers: VT100KeyModifiersIR,
    ) -> Vec<u8> {
        let event = VT100InputEventIR::Mouse {
            button,
            pos: TermPos::from_one_based(col, row),
            action,
            modifiers,
        };
        generate_keyboard_sequence(&event).expect("Failed to generate SGR mouse sequence")
    }

    // X10/Legacy Mouse Protocol Tests
    // Format: ESC [ M Cb Cx Cy (5-6 bytes)
    // Where: Cb = button code, Cx = col (byte - 32), Cy = row (byte - 32)

    #[test]
    fn test_x10_left_click() {
        // X10: Left click at col 1, row 1
        let seq = x10_mouse_sequence(
            VT100MouseButtonIR::Left,
            1,
            1,
            VT100MouseActionIR::Press,
            VT100KeyModifiersIR::default(),
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_mouse_sequence(&seq).expect("Should parse X10");

        assert_eq!(bytes_consumed, byte_offset(6));
        match event {
            VT100InputEventIR::Mouse {
                button,
                pos,
                action,
                modifiers,
            } => {
                assert_eq!(button, VT100MouseButtonIR::Left);
                assert_eq!(pos.col.as_u16(), 1);
                assert_eq!(pos.row.as_u16(), 1);
                assert_eq!(action, VT100MouseActionIR::Press);
                assert!(
                    modifiers.shift == KeyState::NotPressed
                        && modifiers.ctrl == KeyState::NotPressed
                        && modifiers.alt == KeyState::NotPressed
                );
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_x10_middle_click() {
        // X10: Middle click at col 18, row 8
        let seq = x10_mouse_sequence(
            VT100MouseButtonIR::Middle,
            18,
            8,
            VT100MouseActionIR::Press,
            VT100KeyModifiersIR::default(),
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_mouse_sequence(&seq).expect("Should parse X10");

        assert_eq!(bytes_consumed, byte_offset(6));
        match event {
            VT100InputEventIR::Mouse { button, action, .. } => {
                assert_eq!(button, VT100MouseButtonIR::Middle);
                assert_eq!(action, VT100MouseActionIR::Press);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_x10_right_click() {
        // X10: Right click at col 13, row 3
        let seq = x10_mouse_sequence(
            VT100MouseButtonIR::Right,
            13,
            3,
            VT100MouseActionIR::Press,
            VT100KeyModifiersIR::default(),
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_mouse_sequence(&seq).expect("Should parse X10");

        assert_eq!(bytes_consumed, byte_offset(6));
        match event {
            VT100InputEventIR::Mouse { button, action, .. } => {
                assert_eq!(button, VT100MouseButtonIR::Right);
                assert_eq!(action, VT100MouseActionIR::Press);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_x10_release() {
        // X10: Release at col 1, row 1
        let seq = x10_mouse_sequence(
            VT100MouseButtonIR::Left,
            1,
            1,
            VT100MouseActionIR::Release,
            VT100KeyModifiersIR::default(),
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_mouse_sequence(&seq).expect("Should parse X10");

        assert_eq!(bytes_consumed, byte_offset(6));
        match event {
            VT100InputEventIR::Mouse { action, .. } => {
                assert_eq!(action, VT100MouseActionIR::Release);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_x10_motion() {
        // X10: Motion at col 18, row 18
        let seq = x10_mouse_sequence(
            VT100MouseButtonIR::Unknown,
            18,
            18,
            VT100MouseActionIR::Motion,
            VT100KeyModifiersIR::default(),
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_mouse_sequence(&seq).expect("Should parse X10");

        assert_eq!(bytes_consumed, byte_offset(6));
        match event {
            VT100InputEventIR::Mouse { button, action, .. } => {
                assert_eq!(button, VT100MouseButtonIR::Unknown);
                assert_eq!(action, VT100MouseActionIR::Motion);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_x10_with_shift() {
        // X10: Left click with shift at col 1, row 1
        let seq = x10_mouse_sequence(
            VT100MouseButtonIR::Left,
            1,
            1,
            VT100MouseActionIR::Press,
            VT100KeyModifiersIR::SHIFT,
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_mouse_sequence(&seq).expect("Should parse X10");

        assert_eq!(bytes_consumed, byte_offset(6));
        match event {
            VT100InputEventIR::Mouse { modifiers, .. } => {
                assert_eq!(modifiers, VT100KeyModifiersIR::SHIFT);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_x10_with_ctrl() {
        // X10: Left click with ctrl at col 1, row 1
        let seq = x10_mouse_sequence(
            VT100MouseButtonIR::Left,
            1,
            1,
            VT100MouseActionIR::Press,
            VT100KeyModifiersIR::CTRL,
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_mouse_sequence(&seq).expect("Should parse X10");

        assert_eq!(bytes_consumed, byte_offset(6));
        match event {
            VT100InputEventIR::Mouse { modifiers, .. } => {
                assert_eq!(modifiers, VT100KeyModifiersIR::CTRL);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_x10_with_alt() {
        // X10: Left click with alt at col 1, row 1
        let seq = x10_mouse_sequence(
            VT100MouseButtonIR::Left,
            1,
            1,
            VT100MouseActionIR::Press,
            VT100KeyModifiersIR::ALT,
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_mouse_sequence(&seq).expect("Should parse X10");

        assert_eq!(bytes_consumed, byte_offset(6));
        match event {
            VT100InputEventIR::Mouse { modifiers, .. } => {
                assert_eq!(modifiers, VT100KeyModifiersIR::ALT);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_x10_coordinates_1_based() {
        // Verify 1-based coordinates in X10 format
        let seq = x10_mouse_sequence(
            VT100MouseButtonIR::Left,
            1,
            1,
            VT100MouseActionIR::Press,
            VT100KeyModifiersIR::default(),
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_mouse_sequence(&seq).expect("Should parse X10");

        assert_eq!(bytes_consumed, byte_offset(6));
        match event {
            VT100InputEventIR::Mouse { pos, .. } => {
                assert_eq!(pos.col.as_u16(), 1, "Column should be 1-based");
                assert_eq!(pos.row.as_u16(), 1, "Row should be 1-based");
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_x10_large_coordinates() {
        // Test with larger coordinates: col 100, row 50
        let seq = x10_mouse_sequence(
            VT100MouseButtonIR::Left,
            100,
            50,
            VT100MouseActionIR::Press,
            VT100KeyModifiersIR::default(),
        );
        let ParsedInputEventIR { event, .. } =
            parse_mouse_sequence(&seq).expect("Should parse X10");

        match event {
            VT100InputEventIR::Mouse { pos, .. } => {
                assert_eq!(pos.col.as_u16(), 100);
                assert_eq!(pos.row.as_u16(), 50);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_x10_scroll_up() {
        let seq = x10_mouse_sequence(
            VT100MouseButtonIR::Unknown, // Base button for scroll
            10,
            20,
            VT100MouseActionIR::Scroll(VT100ScrollDirectionIR::Up),
            VT100KeyModifiersIR::default(),
        );

        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_mouse_sequence(&seq).expect("Should parse X10");
        assert_eq!(bytes_consumed, byte_offset(MOUSE_X10_MIN_LEN));

        match event {
            VT100InputEventIR::Mouse {
                button,
                pos,
                action,
                modifiers,
            } => {
                assert_eq!(button, VT100MouseButtonIR::Unknown);
                assert_eq!(pos.col.as_u16(), 10);
                assert_eq!(pos.row.as_u16(), 20);
                assert_eq!(
                    action,
                    VT100MouseActionIR::Scroll(VT100ScrollDirectionIR::Up)
                );
                assert_eq!(modifiers, VT100KeyModifiersIR::default());
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_x10_scroll_down() {
        let seq = x10_mouse_sequence(
            VT100MouseButtonIR::Unknown,
            10,
            20,
            VT100MouseActionIR::Scroll(VT100ScrollDirectionIR::Down),
            VT100KeyModifiersIR::default(),
        );

        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_mouse_sequence(&seq).expect("Should parse X10");
        assert_eq!(bytes_consumed, byte_offset(MOUSE_X10_MIN_LEN));

        match event {
            VT100InputEventIR::Mouse { action, .. } => {
                assert_eq!(
                    action,
                    VT100MouseActionIR::Scroll(VT100ScrollDirectionIR::Down)
                );
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_x10_scroll_with_modifiers() {
        let modifiers = VT100KeyModifiersIR::ALT.with_shift();

        let seq = x10_mouse_sequence(
            VT100MouseButtonIR::Unknown,
            1,
            1,
            VT100MouseActionIR::Scroll(VT100ScrollDirectionIR::Up),
            modifiers,
        );

        let ParsedInputEventIR { event, .. } =
            parse_mouse_sequence(&seq).expect("Should parse X10");
        match event {
            VT100InputEventIR::Mouse {
                action,
                modifiers: parsed_mods,
                ..
            } => {
                assert_eq!(
                    action,
                    VT100MouseActionIR::Scroll(VT100ScrollDirectionIR::Up)
                );
                assert_eq!(parsed_mods, modifiers);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_x10_incomplete_sequence() {
        // Incomplete: ESC [ M Cb Cx (missing Cy) - only 5 bytes
        // Note: using raw bytes for intentionally invalid sequence
        let seq = &[
            ANSI_ESC,
            ANSI_CSI_BRACKET,
            MOUSE_X10_MARKER,
            CONTROL_NUL,
            b'!',
        ];
        let result = parse_mouse_sequence(seq);
        assert!(result.is_none(), "Should not parse incomplete X10 sequence");
    }

    #[test]
    fn test_x10_too_short() {
        // Too short: ESC [ M (missing everything else)
        let seq = &[ANSI_ESC, ANSI_CSI_BRACKET, MOUSE_X10_MARKER];
        let result = parse_mouse_sequence(seq);
        assert!(result.is_none(), "Should not parse too-short X10 sequence");
    }

    // RXVT Mouse Protocol Tests
    // Format: ESC [ Cb ; Cx ; Cy M (semicolon-separated decimal, not `<` prefixed)

    #[test]
    fn test_rxvt_left_click() {
        // RXVT: Left click at col 1, row 1
        let seq = rxvt_mouse_sequence(
            VT100MouseButtonIR::Left,
            1,
            1,
            VT100MouseActionIR::Press,
            VT100KeyModifiersIR::default(),
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_mouse_sequence(&seq).expect("Should parse RXVT");

        assert_eq!(bytes_consumed.as_usize(), seq.len());
        match event {
            VT100InputEventIR::Mouse {
                button,
                pos,
                action,
                modifiers,
            } => {
                assert_eq!(button, VT100MouseButtonIR::Left);
                assert_eq!(pos.col.as_u16(), 1);
                assert_eq!(pos.row.as_u16(), 1);
                assert_eq!(action, VT100MouseActionIR::Press);
                assert!(
                    modifiers.shift == KeyState::NotPressed
                        && modifiers.ctrl == KeyState::NotPressed
                        && modifiers.alt == KeyState::NotPressed
                );
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_rxvt_middle_click() {
        // RXVT: Middle click at col 18, row 8
        let seq = rxvt_mouse_sequence(
            VT100MouseButtonIR::Middle,
            18,
            8,
            VT100MouseActionIR::Press,
            VT100KeyModifiersIR::default(),
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_mouse_sequence(&seq).expect("Should parse RXVT");

        assert_eq!(bytes_consumed.as_usize(), seq.len());
        match event {
            VT100InputEventIR::Mouse { button, action, .. } => {
                assert_eq!(button, VT100MouseButtonIR::Middle);
                assert_eq!(action, VT100MouseActionIR::Press);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_rxvt_right_click() {
        // RXVT: Right click at col 13, row 3
        let seq = rxvt_mouse_sequence(
            VT100MouseButtonIR::Right,
            13,
            3,
            VT100MouseActionIR::Press,
            VT100KeyModifiersIR::default(),
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_mouse_sequence(&seq).expect("Should parse RXVT");

        assert_eq!(bytes_consumed.as_usize(), seq.len());
        match event {
            VT100InputEventIR::Mouse { button, action, .. } => {
                assert_eq!(button, VT100MouseButtonIR::Right);
                assert_eq!(action, VT100MouseActionIR::Press);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_rxvt_release() {
        // RXVT: Release at col 1, row 1
        let seq = rxvt_mouse_sequence(
            VT100MouseButtonIR::Left,
            1,
            1,
            VT100MouseActionIR::Release,
            VT100KeyModifiersIR::default(),
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_mouse_sequence(&seq).expect("Should parse RXVT");

        assert_eq!(bytes_consumed.as_usize(), seq.len());
        match event {
            VT100InputEventIR::Mouse { action, .. } => {
                assert_eq!(action, VT100MouseActionIR::Release);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_rxvt_motion() {
        // RXVT: Motion at col 18, row 18
        let seq = rxvt_mouse_sequence(
            VT100MouseButtonIR::Unknown,
            18,
            18,
            VT100MouseActionIR::Motion,
            VT100KeyModifiersIR::default(),
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_mouse_sequence(&seq).expect("Should parse RXVT");

        assert_eq!(bytes_consumed.as_usize(), seq.len());
        match event {
            VT100InputEventIR::Mouse { button, action, .. } => {
                assert_eq!(button, VT100MouseButtonIR::Unknown);
                assert_eq!(action, VT100MouseActionIR::Motion);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_rxvt_with_shift() {
        // RXVT: Left click with shift at col 1, row 1
        let seq = rxvt_mouse_sequence(
            VT100MouseButtonIR::Left,
            1,
            1,
            VT100MouseActionIR::Press,
            VT100KeyModifiersIR::SHIFT,
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_mouse_sequence(&seq).expect("Should parse RXVT");

        assert_eq!(bytes_consumed.as_usize(), seq.len());
        match event {
            VT100InputEventIR::Mouse { modifiers, .. } => {
                assert_eq!(modifiers, VT100KeyModifiersIR::SHIFT);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_rxvt_with_ctrl() {
        // RXVT: Left click with ctrl at col 1, row 1
        let seq = rxvt_mouse_sequence(
            VT100MouseButtonIR::Left,
            1,
            1,
            VT100MouseActionIR::Press,
            VT100KeyModifiersIR::CTRL,
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_mouse_sequence(&seq).expect("Should parse RXVT");

        assert_eq!(bytes_consumed.as_usize(), seq.len());
        match event {
            VT100InputEventIR::Mouse { modifiers, .. } => {
                assert_eq!(modifiers, VT100KeyModifiersIR::CTRL);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_rxvt_with_alt() {
        // RXVT: Left click with alt at col 1, row 1
        let seq = rxvt_mouse_sequence(
            VT100MouseButtonIR::Left,
            1,
            1,
            VT100MouseActionIR::Press,
            VT100KeyModifiersIR::ALT,
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_mouse_sequence(&seq).expect("Should parse RXVT");

        assert_eq!(bytes_consumed.as_usize(), seq.len());
        match event {
            VT100InputEventIR::Mouse { modifiers, .. } => {
                assert_eq!(modifiers, VT100KeyModifiersIR::ALT);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_rxvt_coordinates_1_based() {
        // Verify 1-based coordinates in RXVT format
        let seq = rxvt_mouse_sequence(
            VT100MouseButtonIR::Left,
            1,
            1,
            VT100MouseActionIR::Press,
            VT100KeyModifiersIR::default(),
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_mouse_sequence(&seq).expect("Should parse RXVT");

        assert_eq!(bytes_consumed.as_usize(), seq.len());
        match event {
            VT100InputEventIR::Mouse { pos, .. } => {
                assert_eq!(pos.col.as_u16(), 1, "Column should be 1-based");
                assert_eq!(pos.row.as_u16(), 1, "Row should be 1-based");
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_rxvt_large_coordinates() {
        // Test with larger coordinates: col 100, row 50
        let seq = rxvt_mouse_sequence(
            VT100MouseButtonIR::Left,
            100,
            50,
            VT100MouseActionIR::Press,
            VT100KeyModifiersIR::default(),
        );
        let ParsedInputEventIR { event, .. } =
            parse_mouse_sequence(&seq).expect("Should parse RXVT");

        match event {
            VT100InputEventIR::Mouse { pos, .. } => {
                assert_eq!(pos.col.as_u16(), 100);
                assert_eq!(pos.row.as_u16(), 50);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    /// Test incomplete [`RXVT`] sequence parsing (negative test).
    ///
    /// Uses raw bytes instead of a generator because this tests the parser's
    /// rejection of invalid input. Generators should only produce valid sequences;
    /// this ensures our type system cannot express invalid mouse protocols.
    ///
    /// Sequence: `ESC [ 0 ; 1` (missing `;`, `Cy`, and `M`)
    ///
    /// [`RXVT`]: https://en.wikipedia.org/wiki/Rxvt
    #[test]
    fn test_rxvt_scroll_up() {
        let seq = rxvt_mouse_sequence(
            VT100MouseButtonIR::Unknown, // Base button for scroll
            10,
            20,
            VT100MouseActionIR::Scroll(VT100ScrollDirectionIR::Up),
            VT100KeyModifiersIR::default(),
        );

        let ParsedInputEventIR { event, .. } =
            parse_mouse_sequence(&seq).expect("Should parse RXVT");

        match event {
            VT100InputEventIR::Mouse {
                button,
                pos,
                action,
                modifiers,
            } => {
                assert_eq!(button, VT100MouseButtonIR::Unknown);
                assert_eq!(pos.col.as_u16(), 10);
                assert_eq!(pos.row.as_u16(), 20);
                assert_eq!(
                    action,
                    VT100MouseActionIR::Scroll(VT100ScrollDirectionIR::Up)
                );
                assert_eq!(modifiers, VT100KeyModifiersIR::default());
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_rxvt_scroll_down() {
        let seq = rxvt_mouse_sequence(
            VT100MouseButtonIR::Unknown,
            10,
            20,
            VT100MouseActionIR::Scroll(VT100ScrollDirectionIR::Down),
            VT100KeyModifiersIR::default(),
        );

        let ParsedInputEventIR { event, .. } =
            parse_mouse_sequence(&seq).expect("Should parse RXVT");

        match event {
            VT100InputEventIR::Mouse { action, .. } => {
                assert_eq!(
                    action,
                    VT100MouseActionIR::Scroll(VT100ScrollDirectionIR::Down)
                );
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_rxvt_scroll_with_modifiers() {
        let modifiers = VT100KeyModifiersIR::ALT.with_shift();

        let seq = rxvt_mouse_sequence(
            VT100MouseButtonIR::Unknown,
            1,
            1,
            VT100MouseActionIR::Scroll(VT100ScrollDirectionIR::Up),
            modifiers,
        );

        let ParsedInputEventIR { event, .. } =
            parse_mouse_sequence(&seq).expect("Should parse RXVT");
        match event {
            VT100InputEventIR::Mouse {
                action,
                modifiers: parsed_mods,
                ..
            } => {
                assert_eq!(
                    action,
                    VT100MouseActionIR::Scroll(VT100ScrollDirectionIR::Up)
                );
                assert_eq!(parsed_mods, modifiers);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_rxvt_incomplete_sequence() {
        let seq = &[ANSI_ESC, ANSI_CSI_BRACKET, b'0', b';', b'1'];
        let result = parse_mouse_sequence(seq);
        assert!(
            result.is_none(),
            "Should not parse incomplete RXVT sequence"
        );
    }

    /// Test [`RXVT`] sequence without terminator (negative test).
    ///
    /// Uses raw bytes instead of a generator because this tests the parser's
    /// rejection of invalid input. Generators should only produce valid sequences;
    /// this ensures our type system cannot express invalid mouse protocols.
    ///
    /// Sequence: `ESC [ 0 ; 1 ; 1` (missing `M` terminator)
    ///
    /// [`RXVT`]: https://en.wikipedia.org/wiki/Rxvt
    #[test]
    fn test_rxvt_missing_terminator() {
        let seq = &[ANSI_ESC, ANSI_CSI_BRACKET, b'0', b';', b'1', b';', b'1'];
        let result = parse_mouse_sequence(seq);
        assert!(result.is_none(), "Should not parse RXVT without terminator");
    }

    /// Test [`RXVT`] sequence that is too short (negative test).
    ///
    /// Uses raw bytes instead of a generator because this tests the parser's
    /// rejection of invalid input. Generators should only produce valid sequences;
    /// this ensures our type system cannot express invalid mouse protocols.
    ///
    /// Sequence: `ESC [` (missing all parameters and terminator)
    ///
    /// [`RXVT`]: https://en.wikipedia.org/wiki/Rxvt
    #[test]
    fn test_rxvt_too_short() {
        let seq = &[ANSI_ESC, ANSI_CSI_BRACKET];
        let result = parse_mouse_sequence(seq);
        assert!(result.is_none(), "Should not parse too-short RXVT sequence");
    }

    #[test]
    fn test_sgr_left_click_press() {
        // SGR: Left click press at col 1, row 1
        // Generated sequence: ESC[<0;1;1M
        let seq = sgr_mouse_sequence(
            VT100MouseButtonIR::Left,
            1,
            1,
            VT100MouseActionIR::Press,
            VT100KeyModifiersIR::default(),
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_mouse_sequence(&seq).expect("Should parse");

        assert_eq!(bytes_consumed.as_usize(), seq.len());
        match event {
            VT100InputEventIR::Mouse {
                button,
                pos,
                action,
                modifiers,
            } => {
                assert_eq!(button, VT100MouseButtonIR::Left);
                assert_eq!(pos.col.as_u16(), 1);
                assert_eq!(pos.row.as_u16(), 1);
                assert_eq!(action, VT100MouseActionIR::Press);
                assert!(
                    modifiers.shift == KeyState::NotPressed
                        && modifiers.ctrl == KeyState::NotPressed
                        && modifiers.alt == KeyState::NotPressed
                );
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_sgr_left_click_release() {
        // SGR: Left click release at col 1, row 1
        // Generated sequence: ESC[<0;1;1m (lowercase 'm' = release)
        let seq = sgr_mouse_sequence(
            VT100MouseButtonIR::Left,
            1,
            1,
            VT100MouseActionIR::Release,
            VT100KeyModifiersIR::default(),
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_mouse_sequence(&seq).expect("Should parse");

        assert_eq!(bytes_consumed.as_usize(), seq.len());
        match event {
            VT100InputEventIR::Mouse { action, .. } => {
                assert_eq!(action, VT100MouseActionIR::Release);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_sgr_scroll_up() {
        // SGR: Scroll up at col 37, row 14
        // Generated sequence: ESC[<64;37;14M (button 64 = scroll up)
        let seq = sgr_mouse_sequence(
            VT100MouseButtonIR::Left, // Base button for scroll
            37,
            14,
            VT100MouseActionIR::Scroll(VT100ScrollDirectionIR::Up),
            VT100KeyModifiersIR::default(),
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_mouse_sequence(&seq).expect("Should parse");

        assert_eq!(bytes_consumed.as_usize(), seq.len());
        match event {
            VT100InputEventIR::Mouse { action, pos, .. } => {
                assert_eq!(
                    action,
                    VT100MouseActionIR::Scroll(VT100ScrollDirectionIR::Up)
                );
                assert_eq!(pos.col.as_u16(), 37);
                assert_eq!(pos.row.as_u16(), 14);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_sgr_scroll_down() {
        let seq = sgr_mouse_sequence(
            VT100MouseButtonIR::Unknown,
            37,
            14,
            VT100MouseActionIR::Scroll(VT100ScrollDirectionIR::Down),
            VT100KeyModifiersIR::default(),
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_mouse_sequence(&seq).expect("Should parse");

        assert_eq!(bytes_consumed.as_usize(), seq.len());
        match event {
            VT100InputEventIR::Mouse { action, .. } => {
                assert_eq!(
                    action,
                    VT100MouseActionIR::Scroll(VT100ScrollDirectionIR::Down)
                );
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_sgr_scroll_with_modifiers() {
        let modifiers = VT100KeyModifiersIR::ALT.with_shift();

        let seq = sgr_mouse_sequence(
            VT100MouseButtonIR::Unknown,
            1,
            1,
            VT100MouseActionIR::Scroll(VT100ScrollDirectionIR::Up),
            modifiers,
        );

        let ParsedInputEventIR { event, .. } =
            parse_mouse_sequence(&seq).expect("Should parse");
        match event {
            VT100InputEventIR::Mouse {
                action,
                modifiers: parsed_mods,
                ..
            } => {
                assert_eq!(
                    action,
                    VT100MouseActionIR::Scroll(VT100ScrollDirectionIR::Up)
                );
                assert_eq!(parsed_mods, modifiers);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_sgr_drag() {
        // SGR: Left button drag at col 10, row 5
        // Generated sequence: ESC[<32;10;5M (button 32 = drag with bit 5 set)
        let seq = sgr_mouse_sequence(
            VT100MouseButtonIR::Left,
            10,
            5,
            VT100MouseActionIR::Drag,
            VT100KeyModifiersIR::default(),
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_mouse_sequence(&seq).expect("Should parse");

        assert_eq!(bytes_consumed.as_usize(), seq.len());
        match event {
            VT100InputEventIR::Mouse { button, action, .. } => {
                assert_eq!(button, VT100MouseButtonIR::Left);
                assert_eq!(action, VT100MouseActionIR::Drag);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_modifier_extraction() {
        // SGR: Ctrl+Left click at col 1, row 1
        // Generated sequence: ESC[<16;1;1M (button 16 = Ctrl modifier)
        let seq = sgr_mouse_sequence(
            VT100MouseButtonIR::Left,
            1,
            1,
            VT100MouseActionIR::Press,
            VT100KeyModifiersIR::CTRL,
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_mouse_sequence(&seq).expect("Should parse");

        assert_eq!(bytes_consumed.as_usize(), seq.len());
        match event {
            VT100InputEventIR::Mouse { modifiers, .. } => {
                assert_eq!(modifiers, VT100KeyModifiersIR::CTRL);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_coordinates_are_1_based() {
        // SGR: Verify 1-based coordinates at col 1, row 1
        // Generated sequence: ESC[<0;1;1M
        let seq = sgr_mouse_sequence(
            VT100MouseButtonIR::Left,
            1,
            1,
            VT100MouseActionIR::Press,
            VT100KeyModifiersIR::default(),
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_mouse_sequence(&seq).expect("Should parse");

        assert_eq!(bytes_consumed.as_usize(), seq.len());
        match event {
            VT100InputEventIR::Mouse { pos, .. } => {
                assert_eq!(pos.col.as_u16(), 1, "Column should be 1-based");
                assert_eq!(pos.row.as_u16(), 1, "Row should be 1-based");
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_sgr_motion() {
        // SGR: Motion (hover) at col 12, row 24.
        // Button byte 35 = 32 (Motion) | 3 (Unknown/Release button).
        let seq = b"\x1b[<35;12;24M";
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_mouse_sequence(seq).expect("Should parse SGR motion");

        assert_eq!(bytes_consumed.as_usize(), seq.len());
        match event {
            VT100InputEventIR::Mouse {
                button,
                pos,
                action,
                modifiers,
            } => {
                assert_eq!(button, VT100MouseButtonIR::Unknown);
                assert_eq!(pos.col.as_u16(), 12);
                assert_eq!(pos.row.as_u16(), 24);
                assert_eq!(action, VT100MouseActionIR::Motion);
                assert_eq!(modifiers, VT100KeyModifiersIR::default());
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_sgr_scroll_right() {
        // SGR: Scroll right at col 15, row 25 (button 67 = scroll right).
        let seq = b"\x1b[<67;15;25M";
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_mouse_sequence(seq).expect("Should parse SGR scroll right");

        assert_eq!(bytes_consumed.as_usize(), seq.len());
        match event {
            VT100InputEventIR::Mouse {
                button,
                pos,
                action,
                ..
            } => {
                assert_eq!(button, VT100MouseButtonIR::Unknown);
                assert_eq!(pos.col.as_u16(), 15);
                assert_eq!(pos.row.as_u16(), 25);
                assert_eq!(
                    action,
                    VT100MouseActionIR::Scroll(VT100ScrollDirectionIR::Right)
                );
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_scroll_fallback_direction() {
        // SGR: Unknown scroll button code (e.g. 68) defaults to ScrollDirection::Up.
        let seq = b"\x1b[<68;10;10M";
        let ParsedInputEventIR { event, .. } =
            parse_mouse_sequence(seq).expect("Should parse unknown scroll button");
        match event {
            VT100InputEventIR::Mouse { action, .. } => {
                assert_eq!(
                    action,
                    VT100MouseActionIR::Scroll(VT100ScrollDirectionIR::Up)
                );
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_x10_drag() {
        // X10: Left button drag at col 10, row 5.
        let seq = x10_mouse_sequence(
            VT100MouseButtonIR::Left,
            10,
            5,
            VT100MouseActionIR::Drag,
            VT100KeyModifiersIR::default(),
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_mouse_sequence(&seq).expect("Should parse X10 drag");

        assert_eq!(bytes_consumed, byte_offset(6));
        match event {
            VT100InputEventIR::Mouse { button, action, .. } => {
                assert_eq!(button, VT100MouseButtonIR::Left);
                assert_eq!(action, VT100MouseActionIR::Drag);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_rxvt_drag() {
        // RXVT: Left button drag at col 10, row 5.
        let seq = rxvt_mouse_sequence(
            VT100MouseButtonIR::Left,
            10,
            5,
            VT100MouseActionIR::Drag,
            VT100KeyModifiersIR::default(),
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_mouse_sequence(&seq).expect("Should parse RXVT drag");

        assert_eq!(bytes_consumed.as_usize(), seq.len());
        match event {
            VT100InputEventIR::Mouse { button, action, .. } => {
                assert_eq!(button, VT100MouseButtonIR::Left);
                assert_eq!(action, VT100MouseActionIR::Drag);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_x10_invalid_zero_coordinates() {
        // X10 encoding requires byte > 32 (ASCII space).
        // Byte 32 (space) minus 32 = 0, which is invalid (terminal coordinates are
        // 1-based).
        let seq = &[
            ANSI_ESC,
            ANSI_CSI_BRACKET,
            MOUSE_X10_MARKER,
            b' ',
            b' ',
            b'!',
        ];
        assert!(
            parse_mouse_sequence(seq).is_none(),
            "X10 coordinate resolving to 0 should be rejected"
        );
    }

    #[test]
    fn test_non_mouse_sequences_return_none() {
        assert!(parse_mouse_sequence(&[]).is_none());
        assert!(parse_mouse_sequence(b"plain text").is_none());
        assert!(parse_mouse_sequence(b"\x1b").is_none());
    }
}
