// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Mouse input event [1-based coordinates] parsing from ANSI/`CSI` sequences.
//!
//! This module handles conversion of mouse-related ANSI escape sequences into mouse
//! events, including support for multiple mouse protocols.
//!
//! ## Where You Are in the Pipeline
//!
//! For the full data flow, see the [parent module documentation]. This diagram shows
//! where `mouse.rs` fits:
//!
//! ```text
//! DirectToAnsiInputDevice (async I/O layer)
//!    │
//!    ▼
//! router.rs (routing & `ESC` detection)
//!    │ (routes mouse sequences here)
//! ┌──▼───────────────────────────────────────┐  ┌──────────────────┐
//! │  mouse.rs                                ◀──┤ **YOU ARE HERE** │
//! │  • Parse `SGR` protocol (modern)         │  └──────────────────┘
//! │  • Parse `X10`/Normal (legacy)           │
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
//! - ⬆️ **Up**: [`router`] - Main routing entry point
//! - ➡️ **Peer**: [`keyboard`], [`terminal_events`], [`utf8`] - Other specialized parsers
//! - 📚 **Types**: [`VT100MouseButtonIR`], [`VT100MouseActionIR`], [`TermPos`]
//! - 📤 **Converted by**: [`convert_input_event()`] in `protocol_conversion.rs` (not this
//!   module)
//!
//! ## Supported Mouse Protocols
//! - **`SGR` (Selective Graphic Rendition) Protocol**: Modern standard format
//! - Format: `CSI < Cb ; Cx ; Cy M/m`
//! - Button detection (left=0, middle=1, right=2)
//! - Drag detection (button with flag 32)
//! - Scroll events (buttons 64/65 for vertical, 66/67 for horizontal)
//! - **`X10`/Normal Protocol**: Legacy formats
//! - **`RXVT` Protocol**: Alternative legacy format
//! - **Click Events**: Press (M) and Release (m)
//! - **Drag Events**: Motion while button held
//! - **Motion Events**: Movement without buttons
//! - **Modifier Keys**: Shift, Ctrl, Alt detection
//!
//! # Verifying Coordinate Systems
//!
//! **VT-100 mouse coordinates are 1-based**, where (1, 1) represents the top-left corner.
//! This was confirmed through ground truth discovery via the validation tests, which
//! capture raw bytes from actual terminal interactions. For details on how this was
//! verified, see the [parent module's testing strategy documentation].
//!
//! # Terminal Limitations
//!
//! ## Shift+Click Not Reported
//!
//! Most terminal emulators intercept **Shift+Click** combinations for their own use
//! (text selection, block selection, etc.) and never report these events to the
//! application. This is a terminal-level limitation, not an issue with this parser.
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
//! This limitation is consistent across most terminal emulators (xterm, gnome-terminal,
//! iTerm2, etc.) because Shift+Click is reserved for text selection by the terminal.
//! See the test fixtures for mouse event generation details and validation tests.
//!
//! [1-based coordinates]: mod@super#one-based-mouse-input-events
//! [`TermPos`]: crate::core::coordinates::vt_100_ansi_coords::TermPos
//! [`VT100MouseActionIR`]: super::VT100MouseActionIR
//! [`VT100MouseButtonIR`]: super::VT100MouseButtonIR
//! [`keyboard`]: mod@super::keyboard
//! [`router`]: mod@super::router
//! [`terminal_events`]: mod@super::terminal_events
//! [`utf8`]: mod@super::utf8
//! [parent module documentation]: mod@super#primary-consumer
//! [parent module's testing strategy documentation]: mod@super#testing-strategy
//! [`convert_input_event()`]: crate::tui::terminal_lib_backends::direct_to_ansi::input::protocol_conversion::convert_input_event

use super::ir_event_types::{VT100InputEventIR, VT100KeyModifiersIR, VT100MouseActionIR,
                            VT100MouseButtonIR, VT100ScrollDirectionIR};
use crate::{ByteOffset, KeyState, TermPos, byte_offset,
            core::ansi::constants::{CSI_PREFIX, CSI_PREFIX_LEN, MOUSE_BASE_BUTTON_MASK,
                                    MOUSE_BUTTON_BITS_MASK, MOUSE_BUTTON_CODE_MASK,
                                    MOUSE_MODIFIER_ALT, MOUSE_MODIFIER_CTRL,
                                    MOUSE_MODIFIER_SHIFT, MOUSE_MOTION_FLAG,
                                    MOUSE_SCROLL_THRESHOLD, MOUSE_SGR_PREFIX,
                                    MOUSE_SGR_PREFIX_LEN, MOUSE_SGR_PRESS,
                                    MOUSE_SGR_RELEASE, MOUSE_X10_MARKER,
                                    MOUSE_X10_PREFIX}};

#[must_use]
pub fn parse_mouse_sequence(buffer: &[u8]) -> Option<(VT100InputEventIR, ByteOffset)> {
    // Check for SGR mouse protocol (most reliable)
    if buffer.len() >= 6 && buffer.starts_with(MOUSE_SGR_PREFIX) {
        return parse_sgr_mouse(buffer);
    }

    // Check for X10/Normal protocol (legacy)
    if buffer.len() >= 6 && buffer.starts_with(MOUSE_X10_PREFIX) {
        return parse_x10_mouse(buffer);
    }

    // Check for RXVT protocol (legacy alternative)
    if buffer.len() >= 8
        && buffer.starts_with(CSI_PREFIX)
        && !buffer.starts_with(MOUSE_SGR_PREFIX)
        && !buffer.starts_with(MOUSE_X10_PREFIX)
    {
        // Could be RXVT format: ESC [ Cb ; Cx ; Cy M
        // Try to parse as RXVT - if it fails, we'll return None
        if let Some(result) = parse_rxvt_mouse(buffer) {
            return Some(result);
        }
    }

    None
}

/// Parse `SGR` mouse protocol: `CSI < Cb ; Cx ; Cy M/m`
///
/// Returns `Some((event, bytes_consumed))` for complete sequences, `None` for incomplete.
///
/// Format breakdown:
/// - `ESC [<` prefix (3 bytes)
/// - `Cb` = button byte (with modifiers encoded)
/// - `Cx` = column (1-based)
/// - `Cy` = row (1-based)
/// - `M` = press, `m` = release
fn parse_sgr_mouse(sequence: &[u8]) -> Option<(VT100InputEventIR, ByteOffset)> {
    // Minimum: ESC[<0;1;1M (9 bytes)
    if sequence.len() < 9 {
        return None;
    }

    // Find the terminator (M or m)
    // We need to scan from MOUSE_SGR_PREFIX_LEN onwards to find the terminator
    let mut bytes_consumed = byte_offset(0);
    let mut found_terminator = false;

    for (idx, &byte) in sequence.iter().enumerate().skip(MOUSE_SGR_PREFIX_LEN) {
        if byte == MOUSE_SGR_PRESS || byte == MOUSE_SGR_RELEASE {
            bytes_consumed = byte_offset(idx + 1);
            found_terminator = true;
            break;
        }
    }

    if !found_terminator {
        return None; // Incomplete sequence
    }

    // Extract the action character (terminator)
    let action_char = sequence[bytes_consumed.as_last_byte_index()] as char;

    // Parse the content between ESC[< and M/m
    // Skip prefix (MOUSE_SGR_PREFIX_LEN bytes) and suffix (1 byte)
    let content = std::str::from_utf8(
        &sequence[MOUSE_SGR_PREFIX_LEN..bytes_consumed.as_last_byte_index()],
    )
    .ok()?;

    // Split by semicolons: Cb;Cx;Cy
    let parts: Vec<&str> = content.split(';').collect();
    if parts.len() < 3 {
        return None;
    }

    let cb = parts[0].parse::<u16>().ok()?;
    let cx = parts[1].parse::<u16>().ok()?;
    let cy = parts[2].parse::<u16>().ok()?;

    // Extract modifiers from button byte (bits 2-4)
    let modifiers = extract_modifiers(cb);

    // Check for scroll events first (buttons 64-67)
    if let Some(scroll_dir) = detect_scroll_event(cb) {
        return Some((
            VT100InputEventIR::Mouse {
                button: VT100MouseButtonIR::Unknown,
                pos: TermPos::from_one_based(cx, cy),
                action: VT100MouseActionIR::Scroll(scroll_dir),
                modifiers,
            },
            bytes_consumed,
        ));
    }

    // Detect button type
    let button = detect_mouse_button(cb)?;

    // Detect action
    let action = if is_drag_event(cb) {
        VT100MouseActionIR::Drag
    } else if action_char == 'M' {
        VT100MouseActionIR::Press
    } else {
        VT100MouseActionIR::Release
    };

    Some((
        VT100InputEventIR::Mouse {
            button,
            pos: TermPos::from_one_based(cx, cy),
            action,
            modifiers,
        },
        bytes_consumed,
    ))
}

/// Parse `X10`/Normal mouse protocol: `CSI M Cb Cx Cy`
///
/// Returns `Some((event, bytes_consumed))` for complete sequences, `None` for incomplete.
///
/// Format breakdown:
/// - `ESC [M` prefix (3 bytes)
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
fn parse_x10_mouse(sequence: &[u8]) -> Option<(VT100InputEventIR, ByteOffset)> {
    // X10 format: ESC [ M Cb Cx Cy (5 bytes minimum)
    if sequence.len() < 5 {
        return None;
    }

    // Check prefix: ESC [ M
    if !sequence.starts_with(MOUSE_X10_PREFIX) {
        return None;
    }

    // Extract button, column, and row bytes
    let cb = u16::from(sequence[3]); // Widen to u16 for consistent constant usage
    let cx = sequence[4];
    let cy = if sequence.len() > 5 {
        sequence[5]
    } else {
        return None;
    };

    // Convert raw bytes to 1-based coordinates
    // X10 encoding: byte value - 32 = position (with offset for positions > 95)
    // Positions are 1-based in the terminal
    let col = u16::from(cx).saturating_sub(32);
    let row = u16::from(cy).saturating_sub(32);

    // Handle invalid coordinates
    if col == 0 || row == 0 {
        return None;
    }

    // Extract modifiers from button byte (bits 2-4)
    let modifiers = VT100KeyModifiersIR {
        shift: if (cb & MOUSE_MODIFIER_SHIFT) != 0 {
            KeyState::Pressed
        } else {
            KeyState::NotPressed
        },
        alt: if (cb & MOUSE_MODIFIER_ALT) != 0 {
            KeyState::Pressed
        } else {
            KeyState::NotPressed
        },
        ctrl: if (cb & MOUSE_MODIFIER_CTRL) != 0 {
            KeyState::Pressed
        } else {
            KeyState::NotPressed
        },
    };

    // Check motion flag (bit 5, value 32)
    let is_motion = (cb & MOUSE_MOTION_FLAG) != 0;

    // Get button code (bits 0-1)
    let button_bits = cb & MOUSE_BUTTON_BITS_MASK;

    // Determine action and button
    if is_motion {
        // Motion without button
        return Some((
            VT100InputEventIR::Mouse {
                button: VT100MouseButtonIR::Unknown,
                pos: TermPos::from_one_based(col, row),
                action: VT100MouseActionIR::Motion,
                modifiers,
            },
            byte_offset(6), // ESC [ M Cb Cx Cy = 6 bytes
        ));
    }

    match button_bits {
        0 => {
            // Left button
            Some((
                VT100InputEventIR::Mouse {
                    button: VT100MouseButtonIR::Left,
                    pos: TermPos::from_one_based(col, row),
                    action: VT100MouseActionIR::Press,
                    modifiers,
                },
                byte_offset(6),
            ))
        }
        1 => {
            // Middle button
            Some((
                VT100InputEventIR::Mouse {
                    button: VT100MouseButtonIR::Middle,
                    pos: TermPos::from_one_based(col, row),
                    action: VT100MouseActionIR::Press,
                    modifiers,
                },
                byte_offset(6),
            ))
        }
        2 => {
            // Right button
            Some((
                VT100InputEventIR::Mouse {
                    button: VT100MouseButtonIR::Right,
                    pos: TermPos::from_one_based(col, row),
                    action: VT100MouseActionIR::Press,
                    modifiers,
                },
                byte_offset(6),
            ))
        }
        3 => {
            // Release (button 3)
            Some((
                VT100InputEventIR::Mouse {
                    button: VT100MouseButtonIR::Unknown,
                    pos: TermPos::from_one_based(col, row),
                    action: VT100MouseActionIR::Release,
                    modifiers,
                },
                byte_offset(6),
            ))
        }
        _ => None,
    }
}

/// Parse `RXVT` mouse protocol: `CSI Cb ; Cx ; Cy M`
///
/// Returns `Some((event, bytes_consumed))` for complete sequences, `None` for incomplete.
///
/// Format breakdown:
/// - `ESC [` prefix (2 bytes)
/// - `Cb` = button code (ASCII digits, semicolon-separated)
/// - `Cx` = column (ASCII digits, semicolon-separated)
/// - `Cy` = row (ASCII digits, semicolon-separated)
/// - `M` = terminator (always uppercase, no lowercase 'm')
///
/// Button encoding (similar to X10):
/// - 0 = left button
/// - 1 = middle button
/// - 2 = right button
/// - 3 = release (no button held)
/// - Add 4 for shift, 8 for alt, 16 for ctrl (like X10)
/// - Add 32 for motion (mouse moved)
///
/// Similar to SGR but simpler - no `<` prefix, only M terminator (no m),
/// and always includes coordinates as decimal numbers.
#[allow(clippy::too_many_lines)]
fn parse_rxvt_mouse(sequence: &[u8]) -> Option<(VT100InputEventIR, ByteOffset)> {
    // RXVT format: ESC [ Cb ; Cx ; Cy M (minimum 8 bytes: ESC[0;1;1M)
    if sequence.len() < 8 {
        return None;
    }

    // Check prefix: ESC [
    if !sequence.starts_with(CSI_PREFIX) {
        return None;
    }

    // Find the terminator 'M'
    let mut bytes_consumed = byte_offset(0);
    let mut found_terminator = false;

    for (idx, &byte) in sequence.iter().enumerate().skip(CSI_PREFIX_LEN) {
        if byte == MOUSE_X10_MARKER {
            bytes_consumed = byte_offset(idx + 1);
            found_terminator = true;
            break;
        }
    }

    if !found_terminator {
        return None; // Incomplete sequence
    }

    // Parse the content between ESC[ and M
    // Skip prefix (CSI_PREFIX_LEN bytes) and suffix (1 byte)
    let content = std::str::from_utf8(
        &sequence[CSI_PREFIX_LEN..bytes_consumed.as_last_byte_index()],
    )
    .ok()?;

    // Split by semicolons: Cb;Cx;Cy
    let parts: Vec<&str> = content.split(';').collect();
    if parts.len() < 3 {
        return None;
    }

    let cb = parts[0].parse::<u16>().ok()?;
    let cx = parts[1].parse::<u16>().ok()?;
    let cy = parts[2].parse::<u16>().ok()?;

    // Extract modifiers from button byte (similar to X10)
    let modifiers = VT100KeyModifiersIR {
        shift: if (cb & MOUSE_MODIFIER_SHIFT) != 0 {
            KeyState::Pressed
        } else {
            KeyState::NotPressed
        },
        alt: if (cb & MOUSE_MODIFIER_ALT) != 0 {
            KeyState::Pressed
        } else {
            KeyState::NotPressed
        },
        ctrl: if (cb & MOUSE_MODIFIER_CTRL) != 0 {
            KeyState::Pressed
        } else {
            KeyState::NotPressed
        },
    };

    // Check motion flag (bit 5, value 32)
    let is_motion = (cb & MOUSE_MOTION_FLAG) != 0;

    // Get button code (bits 0-1)
    let button_bits = cb & MOUSE_BUTTON_BITS_MASK;

    // Determine action and button
    if is_motion {
        // Motion without button
        return Some((
            VT100InputEventIR::Mouse {
                button: VT100MouseButtonIR::Unknown,
                pos: TermPos::from_one_based(cx, cy),
                action: VT100MouseActionIR::Motion,
                modifiers,
            },
            bytes_consumed,
        ));
    }

    match button_bits {
        0 => {
            // Left button
            Some((
                VT100InputEventIR::Mouse {
                    button: VT100MouseButtonIR::Left,
                    pos: TermPos::from_one_based(cx, cy),
                    action: VT100MouseActionIR::Press,
                    modifiers,
                },
                bytes_consumed,
            ))
        }
        1 => {
            // Middle button
            Some((
                VT100InputEventIR::Mouse {
                    button: VT100MouseButtonIR::Middle,
                    pos: TermPos::from_one_based(cx, cy),
                    action: VT100MouseActionIR::Press,
                    modifiers,
                },
                bytes_consumed,
            ))
        }
        2 => {
            // Right button
            Some((
                VT100InputEventIR::Mouse {
                    button: VT100MouseButtonIR::Right,
                    pos: TermPos::from_one_based(cx, cy),
                    action: VT100MouseActionIR::Press,
                    modifiers,
                },
                bytes_consumed,
            ))
        }
        3 => {
            // Release (button 3)
            Some((
                VT100InputEventIR::Mouse {
                    button: VT100MouseButtonIR::Unknown,
                    pos: TermPos::from_one_based(cx, cy),
                    action: VT100MouseActionIR::Release,
                    modifiers,
                },
                bytes_consumed,
            ))
        }
        _ => None,
    }
}

/// Detect mouse button from SGR button byte.
///
/// Button encoding (bits 0-1):
/// - 0 = left button
/// - 1 = middle button
/// - 2 = right button
/// - 3 = release (for legacy modes, SGR uses 'M'/'m' instead)
fn detect_mouse_button(cb: u16) -> Option<VT100MouseButtonIR> {
    // Mask out modifier and drag bits (keep only bits 0-5)
    let button_code = cb & MOUSE_BUTTON_CODE_MASK;

    // Scroll events are handled separately
    if button_code >= MOUSE_SCROLL_THRESHOLD {
        return None;
    }

    // Get base button (bits 0-1)
    match button_code & MOUSE_BUTTON_BITS_MASK {
        0 => Some(VT100MouseButtonIR::Left),
        1 => Some(VT100MouseButtonIR::Middle),
        2 => Some(VT100MouseButtonIR::Right),
        _ => Some(VT100MouseButtonIR::Unknown),
    }
}

/// Detect if mouse event is a drag (button held while moving).
///
/// Drag flag is bit 5 (value 32) in the button byte.
fn is_drag_event(cb: u16) -> bool { (cb & MOUSE_MOTION_FLAG) != 0 }

/// Detect scroll events (up/down/left/right).
///
/// Scroll button codes:
/// - 64 = scroll up
/// - 65 = scroll down
/// - 66 = scroll left (rare) - but often used for scroll up with modifiers!
/// - 67 = scroll right (rare)
fn detect_scroll_event(cb: u16) -> Option<VT100ScrollDirectionIR> {
    // Check raw button code first (before masking modifiers)
    // Buttons 64+ indicate scroll events
    if cb >= MOUSE_SCROLL_THRESHOLD {
        // Mask to get base button (without modifiers but keeping scroll bit)
        let base_button = cb & MOUSE_BASE_BUTTON_MASK; // Keep bit 6 (value 64)

        match base_button {
            68..=71 => Some(VT100ScrollDirectionIR::Down), // All scroll down variants
            _ /* 64..=67 */ => Some(VT100ScrollDirectionIR::Up), /* All scroll up variants + default to up for unknown scroll events */
        }
    } else {
        None
    }
}

/// Extract modifier keys (Shift, Ctrl, Alt) from SGR sequence.
///
/// Modifier encoding (bits 2-4):
/// - Bit 2 (value 4): Shift
/// - Bit 3 (value 8): Alt
/// - Bit 4 (value 16): Ctrl
fn extract_modifiers(cb: u16) -> VT100KeyModifiersIR {
    VT100KeyModifiersIR {
        shift: if (cb & MOUSE_MODIFIER_SHIFT) != 0 {
            KeyState::Pressed
        } else {
            KeyState::NotPressed
        },
        alt: if (cb & MOUSE_MODIFIER_ALT) != 0 {
            KeyState::Pressed
        } else {
            KeyState::NotPressed
        },
        ctrl: if (cb & MOUSE_MODIFIER_CTRL) != 0 {
            KeyState::Pressed
        } else {
            KeyState::NotPressed
        },
    }
}

/// Unit tests for mouse input parsing.
///
/// These tests use generator functions instead of hardcoded magic strings to ensure
/// consistency between sequence generation and parsing. For testing strategy details,
/// see the [testing strategy] documentation.
///
/// [testing strategy]: mod@super#testing-strategy
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ansi::constants::{ANSI_CSI_BRACKET, ANSI_ESC, CONTROL_NUL};

    // ==================== Test Helpers ====================

    /// Build an X10 mouse sequence using the generator.
    ///
    /// X10 format: `ESC [ M Cb Cx Cy` (6 bytes with null terminator)
    fn x10_mouse_sequence(
        button: VT100MouseButtonIR,
        col: u16,
        row: u16,
        action: VT100MouseActionIR,
        modifiers: VT100KeyModifiersIR,
    ) -> Vec<u8> {
        use crate::core::ansi::vt_100_terminal_input_parser::test_fixtures::generate_x10_mouse_sequence;
        generate_x10_mouse_sequence(button, col, row, action, modifiers)
    }

    /// Build an RXVT mouse sequence using the generator.
    ///
    /// RXVT format: `ESC [ Cb ; Cx ; Cy M` (decimal with semicolons)
    fn rxvt_mouse_sequence(
        button: VT100MouseButtonIR,
        col: u16,
        row: u16,
        action: VT100MouseActionIR,
        modifiers: VT100KeyModifiersIR,
    ) -> Vec<u8> {
        use crate::core::ansi::vt_100_terminal_input_parser::test_fixtures::generate_rxvt_mouse_sequence;
        generate_rxvt_mouse_sequence(button, col, row, action, modifiers)
    }

    /// Build an SGR mouse sequence using the generator.
    ///
    /// SGR format: `ESC [ < Cb ; Cx ; Cy M/m` (modern standard)
    fn sgr_mouse_sequence(
        button: VT100MouseButtonIR,
        col: u16,
        row: u16,
        action: VT100MouseActionIR,
        modifiers: VT100KeyModifiersIR,
    ) -> Vec<u8> {
        use crate::core::ansi::vt_100_terminal_input_parser::test_fixtures::generate_keyboard_sequence;
        let event = VT100InputEventIR::Mouse {
            button,
            pos: TermPos::from_one_based(col, row),
            action,
            modifiers,
        };
        generate_keyboard_sequence(&event).expect("Failed to generate SGR mouse sequence")
    }

    // X10/Normal Mouse Protocol Tests
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
        let (event, bytes_consumed) =
            parse_mouse_sequence(&seq).expect("Should parse X10");

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
        let (event, bytes_consumed) =
            parse_mouse_sequence(&seq).expect("Should parse X10");

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
        let (event, bytes_consumed) =
            parse_mouse_sequence(&seq).expect("Should parse X10");

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
        let (event, bytes_consumed) =
            parse_mouse_sequence(&seq).expect("Should parse X10");

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
        let (event, bytes_consumed) =
            parse_mouse_sequence(&seq).expect("Should parse X10");

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
            VT100KeyModifiersIR {
                shift: KeyState::Pressed,
                ctrl: KeyState::NotPressed,
                alt: KeyState::NotPressed,
            },
        );
        let (event, bytes_consumed) =
            parse_mouse_sequence(&seq).expect("Should parse X10");

        assert_eq!(bytes_consumed, byte_offset(6));
        match event {
            VT100InputEventIR::Mouse { modifiers, .. } => {
                assert_eq!(modifiers.shift, KeyState::Pressed);
                assert_eq!(modifiers.ctrl, KeyState::NotPressed);
                assert_eq!(modifiers.alt, KeyState::NotPressed);
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
            VT100KeyModifiersIR {
                shift: KeyState::NotPressed,
                ctrl: KeyState::Pressed,
                alt: KeyState::NotPressed,
            },
        );
        let (event, bytes_consumed) =
            parse_mouse_sequence(&seq).expect("Should parse X10");

        assert_eq!(bytes_consumed, byte_offset(6));
        match event {
            VT100InputEventIR::Mouse { modifiers, .. } => {
                assert_eq!(modifiers.shift, KeyState::NotPressed);
                assert_eq!(modifiers.ctrl, KeyState::Pressed);
                assert_eq!(modifiers.alt, KeyState::NotPressed);
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
            VT100KeyModifiersIR {
                shift: KeyState::NotPressed,
                ctrl: KeyState::NotPressed,
                alt: KeyState::Pressed,
            },
        );
        let (event, bytes_consumed) =
            parse_mouse_sequence(&seq).expect("Should parse X10");

        assert_eq!(bytes_consumed, byte_offset(6));
        match event {
            VT100InputEventIR::Mouse { modifiers, .. } => {
                assert_eq!(modifiers.shift, KeyState::NotPressed);
                assert_eq!(modifiers.ctrl, KeyState::NotPressed);
                assert_eq!(modifiers.alt, KeyState::Pressed);
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
        let (event, bytes_consumed) =
            parse_mouse_sequence(&seq).expect("Should parse X10");

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
        let (event, _) = parse_mouse_sequence(&seq).expect("Should parse X10");

        match event {
            VT100InputEventIR::Mouse { pos, .. } => {
                assert_eq!(pos.col.as_u16(), 100);
                assert_eq!(pos.row.as_u16(), 50);
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
        let (event, bytes_consumed) =
            parse_mouse_sequence(&seq).expect("Should parse RXVT");

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
        let (event, bytes_consumed) =
            parse_mouse_sequence(&seq).expect("Should parse RXVT");

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
        let (event, bytes_consumed) =
            parse_mouse_sequence(&seq).expect("Should parse RXVT");

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
        let (event, bytes_consumed) =
            parse_mouse_sequence(&seq).expect("Should parse RXVT");

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
        let (event, bytes_consumed) =
            parse_mouse_sequence(&seq).expect("Should parse RXVT");

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
            VT100KeyModifiersIR {
                shift: KeyState::Pressed,
                ctrl: KeyState::NotPressed,
                alt: KeyState::NotPressed,
            },
        );
        let (event, bytes_consumed) =
            parse_mouse_sequence(&seq).expect("Should parse RXVT");

        assert_eq!(bytes_consumed.as_usize(), seq.len());
        match event {
            VT100InputEventIR::Mouse { modifiers, .. } => {
                assert_eq!(modifiers.shift, KeyState::Pressed);
                assert_eq!(modifiers.ctrl, KeyState::NotPressed);
                assert_eq!(modifiers.alt, KeyState::NotPressed);
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
            VT100KeyModifiersIR {
                shift: KeyState::NotPressed,
                ctrl: KeyState::Pressed,
                alt: KeyState::NotPressed,
            },
        );
        let (event, bytes_consumed) =
            parse_mouse_sequence(&seq).expect("Should parse RXVT");

        assert_eq!(bytes_consumed.as_usize(), seq.len());
        match event {
            VT100InputEventIR::Mouse { modifiers, .. } => {
                assert_eq!(modifiers.shift, KeyState::NotPressed);
                assert_eq!(modifiers.ctrl, KeyState::Pressed);
                assert_eq!(modifiers.alt, KeyState::NotPressed);
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
            VT100KeyModifiersIR {
                shift: KeyState::NotPressed,
                ctrl: KeyState::NotPressed,
                alt: KeyState::Pressed,
            },
        );
        let (event, bytes_consumed) =
            parse_mouse_sequence(&seq).expect("Should parse RXVT");

        assert_eq!(bytes_consumed.as_usize(), seq.len());
        match event {
            VT100InputEventIR::Mouse { modifiers, .. } => {
                assert_eq!(modifiers.shift, KeyState::NotPressed);
                assert_eq!(modifiers.ctrl, KeyState::NotPressed);
                assert_eq!(modifiers.alt, KeyState::Pressed);
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
        let (event, bytes_consumed) =
            parse_mouse_sequence(&seq).expect("Should parse RXVT");

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
        let (event, _) = parse_mouse_sequence(&seq).expect("Should parse RXVT");

        match event {
            VT100InputEventIR::Mouse { pos, .. } => {
                assert_eq!(pos.col.as_u16(), 100);
                assert_eq!(pos.row.as_u16(), 50);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    /// Test incomplete RXVT sequence parsing (negative test).
    ///
    /// Uses raw bytes instead of a generator because this tests the parser's
    /// rejection of invalid input. Generators should only produce valid sequences;
    /// this ensures our type system cannot express invalid mouse protocols.
    ///
    /// Sequence: `ESC [ 0 ; 1` (missing `;`, `Cy`, and `M`)
    #[test]
    fn test_rxvt_incomplete_sequence() {
        let seq = &[ANSI_ESC, ANSI_CSI_BRACKET, b'0', b';', b'1'];
        let result = parse_mouse_sequence(seq);
        assert!(
            result.is_none(),
            "Should not parse incomplete RXVT sequence"
        );
    }

    /// Test RXVT sequence without terminator (negative test).
    ///
    /// Uses raw bytes instead of a generator because this tests the parser's
    /// rejection of invalid input. Generators should only produce valid sequences;
    /// this ensures our type system cannot express invalid mouse protocols.
    ///
    /// Sequence: `ESC [ 0 ; 1 ; 1` (missing `M` terminator)
    #[test]
    fn test_rxvt_missing_terminator() {
        let seq = &[ANSI_ESC, ANSI_CSI_BRACKET, b'0', b';', b'1', b';', b'1'];
        let result = parse_mouse_sequence(seq);
        assert!(result.is_none(), "Should not parse RXVT without terminator");
    }

    /// Test RXVT sequence that is too short (negative test).
    ///
    /// Uses raw bytes instead of a generator because this tests the parser's
    /// rejection of invalid input. Generators should only produce valid sequences;
    /// this ensures our type system cannot express invalid mouse protocols.
    ///
    /// Sequence: `ESC [` (missing all parameters and terminator)
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
        let (event, bytes_consumed) = parse_mouse_sequence(&seq).expect("Should parse");

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
        let (event, bytes_consumed) = parse_mouse_sequence(&seq).expect("Should parse");

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
        let (event, bytes_consumed) = parse_mouse_sequence(&seq).expect("Should parse");

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
        let (event, bytes_consumed) = parse_mouse_sequence(&seq).expect("Should parse");

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
            VT100KeyModifiersIR {
                ctrl: KeyState::Pressed,
                shift: KeyState::NotPressed,
                alt: KeyState::NotPressed,
            },
        );
        let (event, bytes_consumed) = parse_mouse_sequence(&seq).expect("Should parse");

        assert_eq!(bytes_consumed.as_usize(), seq.len());
        match event {
            VT100InputEventIR::Mouse { modifiers, .. } => {
                assert_eq!(modifiers.ctrl, KeyState::Pressed);
                assert_eq!(modifiers.shift, KeyState::NotPressed);
                assert_eq!(modifiers.alt, KeyState::NotPressed);
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
        let (event, bytes_consumed) = parse_mouse_sequence(&seq).expect("Should parse");

        assert_eq!(bytes_consumed.as_usize(), seq.len());
        match event {
            VT100InputEventIR::Mouse { pos, .. } => {
                assert_eq!(pos.col.as_u16(), 1, "Column should be 1-based");
                assert_eq!(pos.row.as_u16(), 1, "Row should be 1-based");
            }
            _ => panic!("Expected Mouse event"),
        }
    }
}
