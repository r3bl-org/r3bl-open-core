// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Mouse input event [1-based coordinates] parsing from ANSI/CSI sequences.
//!
//! This module handles conversion of mouse-related ANSI escape sequences into mouse
//! events, including support for:
//!
//! - **SGR (Selective Graphic Rendition) Protocol**: Modern standard format
//!   - Format: `CSI < Cb ; Cx ; Cy M/m`
//!   - Button detection (left=0, middle=1, right=2)
//!   - Drag detection (button with flag 32)
//!   - Scroll events (buttons 64/65 for vertical, 66/67 for horizontal)
//!
//! - **X10/Normal Protocol**: Legacy formats
//! - **RXVT Protocol**: Alternative legacy format
//!
//! - **Click Events**: Press (M) and Release (m)
//! - **Drag Events**: Motion while button held
//! - **Motion Events**: Movement without buttons
//! - **Modifier Keys**: Shift, Ctrl, Alt detection
//!
//! [1-based coordinates]: mod@super#one-based-mouse-input-events

use crate::TermPos;
use super::types::{VT100InputEvent, VT100KeyModifiers, VT100MouseAction, VT100MouseButton,
                   VT100ScrollDirection};

pub fn parse_mouse_sequence(buffer: &[u8]) -> Option<(VT100InputEvent, usize)> {
    // Check for SGR mouse protocol (most reliable)
    if buffer.len() >= 6 && buffer.starts_with(b"\x1b[<") {
        return parse_sgr_mouse(buffer);
    }

    // Check for X10/Normal protocol (legacy)
    if buffer.len() >= 6 && buffer.starts_with(b"\x1b[M") {
        return parse_x10_mouse(buffer);
    }

    // Check for RXVT protocol (legacy alternative)
    if buffer.len() >= 8 && buffer.starts_with(b"\x1b[") && !buffer.starts_with(b"\x1b[<")
        && !buffer.starts_with(b"\x1b[M") {
        // Could be RXVT format: ESC [ Cb ; Cx ; Cy M
        // Try to parse as RXVT - if it fails, we'll return None
        if let Some(result) = parse_rxvt_mouse(buffer) {
            return Some(result);
        }
    }

    None
}

/// Parse SGR mouse protocol: `CSI < Cb ; Cx ; Cy M/m`
///
/// Returns `Some((event, bytes_consumed))` for complete sequences, `None` for incomplete.
///
/// Format breakdown:
/// - `ESC[<` prefix (3 bytes)
/// - `Cb` = button byte (with modifiers encoded)
/// - `Cx` = column (1-based)
/// - `Cy` = row (1-based)
/// - `M` = press, `m` = release
fn parse_sgr_mouse(sequence: &[u8]) -> Option<(VT100InputEvent, usize)> {
    // Minimum: ESC[<0;1;1M (9 bytes)
    if sequence.len() < 9 {
        return None;
    }

    // Find the terminator (M or m)
    // We need to scan from position 3 onwards to find the terminator
    let mut bytes_consumed = 0;
    let mut found_terminator = false;

    for (idx, &byte) in sequence.iter().enumerate().skip(3) {
        if byte == b'M' || byte == b'm' {
            bytes_consumed = idx + 1;
            found_terminator = true;
            break;
        }
    }

    if !found_terminator {
        return None; // Incomplete sequence
    }

    // Extract the action character (terminator)
    let action_char = sequence[bytes_consumed - 1] as char;

    // Parse the content between ESC[< and M/m
    // Skip prefix (3 bytes) and suffix (1 byte)
    let content = std::str::from_utf8(&sequence[3..bytes_consumed - 1]).ok()?;

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
            VT100InputEvent::Mouse {
                button: VT100MouseButton::Unknown,
                pos: TermPos::from_one_based(cx, cy),
                action: VT100MouseAction::Scroll(scroll_dir),
                modifiers,
            },
            bytes_consumed,
        ));
    }

    // Detect button type
    let button = detect_mouse_button(cb)?;

    // Detect action
    let action = if is_drag_event(cb) {
        VT100MouseAction::Drag
    } else if action_char == 'M' {
        VT100MouseAction::Press
    } else {
        VT100MouseAction::Release
    };

    Some((
        VT100InputEvent::Mouse {
            button,
            pos: TermPos::from_one_based(cx, cy),
            action,
            modifiers,
        },
        bytes_consumed,
    ))
}

/// Parse X10/Normal mouse protocol: `CSI M Cb Cx Cy`
///
/// Returns `Some((event, bytes_consumed))` for complete sequences, `None` for incomplete.
///
/// Format breakdown:
/// - `ESC[M` prefix (3 bytes)
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
fn parse_x10_mouse(sequence: &[u8]) -> Option<(VT100InputEvent, usize)> {
    // X10 format: ESC [ M Cb Cx Cy (5 bytes minimum)
    if sequence.len() < 5 {
        return None;
    }

    // Check prefix: ESC [ M
    if !sequence.starts_with(b"\x1b[M") {
        return None;
    }

    // Extract button, column, and row bytes
    let cb = sequence[3];
    let cx = sequence[4];
    let cy = if sequence.len() > 5 { sequence[5] } else { return None };

    // Convert raw bytes to 1-based coordinates
    // X10 encoding: byte value - 32 = position (with offset for positions > 95)
    // Positions are 1-based in the terminal
    let col = (cx as u16).saturating_sub(32);
    let row = (cy as u16).saturating_sub(32);

    // Handle invalid coordinates
    if col == 0 || row == 0 {
        return None;
    }

    // Extract modifiers from button byte (bits 2-4)
    let modifiers = VT100KeyModifiers {
        shift: (cb & 4) != 0,
        alt: (cb & 8) != 0,
        ctrl: (cb & 16) != 0,
    };

    // Check motion flag (bit 5, value 32)
    let is_motion = (cb & 32) != 0;

    // Get button code (bits 0-1)
    let button_bits = cb & 0x3;

    // Determine action and button
    if is_motion {
        // Motion without button
        return Some((
            VT100InputEvent::Mouse {
                button: VT100MouseButton::Unknown,
                pos: TermPos::from_one_based(col, row),
                action: VT100MouseAction::Motion,
                modifiers,
            },
            6, // ESC [ M Cb Cx Cy = 6 bytes
        ));
    }

    match button_bits {
        0 => {
            // Left button
            Some((
                VT100InputEvent::Mouse {
                    button: VT100MouseButton::Left,
                    pos: TermPos::from_one_based(col, row),
                    action: VT100MouseAction::Press,
                    modifiers,
                },
                6,
            ))
        }
        1 => {
            // Middle button
            Some((
                VT100InputEvent::Mouse {
                    button: VT100MouseButton::Middle,
                    pos: TermPos::from_one_based(col, row),
                    action: VT100MouseAction::Press,
                    modifiers,
                },
                6,
            ))
        }
        2 => {
            // Right button
            Some((
                VT100InputEvent::Mouse {
                    button: VT100MouseButton::Right,
                    pos: TermPos::from_one_based(col, row),
                    action: VT100MouseAction::Press,
                    modifiers,
                },
                6,
            ))
        }
        3 => {
            // Release (button 3)
            Some((
                VT100InputEvent::Mouse {
                    button: VT100MouseButton::Unknown,
                    pos: TermPos::from_one_based(col, row),
                    action: VT100MouseAction::Release,
                    modifiers,
                },
                6,
            ))
        }
        _ => None,
    }
}

/// Parse RXVT mouse protocol: `CSI Cb ; Cx ; Cy M`
///
/// Returns `Some((event, bytes_consumed))` for complete sequences, `None` for incomplete.
///
/// Format breakdown:
/// - `ESC[` prefix (2 bytes)
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
fn parse_rxvt_mouse(sequence: &[u8]) -> Option<(VT100InputEvent, usize)> {
    // RXVT format: ESC [ Cb ; Cx ; Cy M (minimum 8 bytes: ESC[0;1;1M)
    if sequence.len() < 8 {
        return None;
    }

    // Check prefix: ESC [
    if !sequence.starts_with(b"\x1b[") {
        return None;
    }

    // Find the terminator 'M'
    let mut bytes_consumed = 0;
    let mut found_terminator = false;

    for (idx, &byte) in sequence.iter().enumerate().skip(2) {
        if byte == b'M' {
            bytes_consumed = idx + 1;
            found_terminator = true;
            break;
        }
    }

    if !found_terminator {
        return None; // Incomplete sequence
    }

    // Parse the content between ESC[ and M
    // Skip prefix (2 bytes) and suffix (1 byte)
    let content = std::str::from_utf8(&sequence[2..bytes_consumed - 1]).ok()?;

    // Split by semicolons: Cb;Cx;Cy
    let parts: Vec<&str> = content.split(';').collect();
    if parts.len() < 3 {
        return None;
    }

    let cb = parts[0].parse::<u16>().ok()?;
    let cx = parts[1].parse::<u16>().ok()?;
    let cy = parts[2].parse::<u16>().ok()?;

    // Extract modifiers from button byte (similar to X10)
    let modifiers = VT100KeyModifiers {
        shift: (cb & 4) != 0,
        alt: (cb & 8) != 0,
        ctrl: (cb & 16) != 0,
    };

    // Check motion flag (bit 5, value 32)
    let is_motion = (cb & 32) != 0;

    // Get button code (bits 0-1)
    let button_bits = cb & 0x3;

    // Determine action and button
    if is_motion {
        // Motion without button
        return Some((
            VT100InputEvent::Mouse {
                button: VT100MouseButton::Unknown,
                pos: TermPos::from_one_based(cx, cy),
                action: VT100MouseAction::Motion,
                modifiers,
            },
            bytes_consumed,
        ));
    }

    match button_bits {
        0 => {
            // Left button
            Some((
                VT100InputEvent::Mouse {
                    button: VT100MouseButton::Left,
                    pos: TermPos::from_one_based(cx, cy),
                    action: VT100MouseAction::Press,
                    modifiers,
                },
                bytes_consumed,
            ))
        }
        1 => {
            // Middle button
            Some((
                VT100InputEvent::Mouse {
                    button: VT100MouseButton::Middle,
                    pos: TermPos::from_one_based(cx, cy),
                    action: VT100MouseAction::Press,
                    modifiers,
                },
                bytes_consumed,
            ))
        }
        2 => {
            // Right button
            Some((
                VT100InputEvent::Mouse {
                    button: VT100MouseButton::Right,
                    pos: TermPos::from_one_based(cx, cy),
                    action: VT100MouseAction::Press,
                    modifiers,
                },
                bytes_consumed,
            ))
        }
        3 => {
            // Release (button 3)
            Some((
                VT100InputEvent::Mouse {
                    button: VT100MouseButton::Unknown,
                    pos: TermPos::from_one_based(cx, cy),
                    action: VT100MouseAction::Release,
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
fn detect_mouse_button(cb: u16) -> Option<VT100MouseButton> {
    // Mask out modifier and drag bits (keep only bits 0-5)
    let button_code = cb & 0x3F;

    // Scroll events are handled separately
    if button_code >= 64 {
        return None;
    }

    // Get base button (bits 0-1)
    match button_code & 0x3 {
        0 => Some(VT100MouseButton::Left),
        1 => Some(VT100MouseButton::Middle),
        2 => Some(VT100MouseButton::Right),
        _ => Some(VT100MouseButton::Unknown),
    }
}

/// Detect if mouse event is a drag (button held while moving).
///
/// Drag flag is bit 5 (value 32) in the button byte.
fn is_drag_event(cb: u16) -> bool { (cb & 32) != 0 }

/// Detect scroll events (up/down/left/right).
///
/// Scroll button codes:
/// - 64 = scroll up
/// - 65 = scroll down
/// - 66 = scroll left (rare) - but often used for scroll up with modifiers!
/// - 67 = scroll right (rare)
fn detect_scroll_event(cb: u16) -> Option<VT100ScrollDirection> {
    // Check raw button code first (before masking modifiers)
    // Buttons 64+ indicate scroll events
    if cb >= 64 {
        // Mask to get base button (without modifiers but keeping scroll bit)
        let base_button = cb & 0x7F; // Keep bit 6 (value 64)

        match base_button {
            64..=67 => Some(VT100ScrollDirection::Up), // All scroll up variants
            68..=71 => Some(VT100ScrollDirection::Down), // All scroll down variants
            _ => Some(VT100ScrollDirection::Up),       /* Default to up for unknown scroll
                                                    * events */
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
fn extract_modifiers(cb: u16) -> VT100KeyModifiers {
    VT100KeyModifiers {
        shift: (cb & 4) != 0,
        alt: (cb & 8) != 0,
        ctrl: (cb & 16) != 0,
    }
}

    // X10/Normal Mouse Protocol Tests
    // Format: ESC [ M Cb Cx Cy (5-6 bytes)
    // Where: Cb = button code, Cx = col (byte - 32), Cy = row (byte - 32)

    #[test]
    fn test_x10_left_click() {
        // X10: ESC [ M 0 33 33 (left click at col 1, row 1)
        // Button: 0 (left), Col: 33-32=1, Row: 33-32=1
        let seq = b"\x1b[M\x00!!\x00";
        let (event, bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse X10");

        assert_eq!(bytes_consumed, 6);
        match event {
            VT100InputEvent::Mouse {
                button,
                pos,
                action,
                modifiers,
            } => {
                assert_eq!(button, VT100MouseButton::Left);
                assert_eq!(pos.col.as_u16(), 1);
                assert_eq!(pos.row.as_u16(), 1);
                assert_eq!(action, VT100MouseAction::Press);
                assert!(!modifiers.shift && !modifiers.ctrl && !modifiers.alt);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_x10_middle_click() {
        // X10: ESC [ M 1 50 40 (middle click at col 18, row 8)
        // Button: 1 (middle), Col: 50-32=18, Row: 40-32=8
        let seq = b"\x1b[M\x012(\x00";
        let (event, bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse X10");

        assert_eq!(bytes_consumed, 6);
        match event {
            VT100InputEvent::Mouse { button, action, .. } => {
                assert_eq!(button, VT100MouseButton::Middle);
                assert_eq!(action, VT100MouseAction::Press);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_x10_right_click() {
        // X10: ESC [ M 2 45 35 (right click at col 13, row 3)
        // Button: 2 (right), Col: 45-32=13, Row: 35-32=3
        let seq = b"\x1b[M\x02-#\x00";
        let (event, bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse X10");

        assert_eq!(bytes_consumed, 6);
        match event {
            VT100InputEvent::Mouse { button, action, .. } => {
                assert_eq!(button, VT100MouseButton::Right);
                assert_eq!(action, VT100MouseAction::Press);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_x10_release() {
        // X10: ESC [ M 3 33 33 (release at col 1, row 1)
        // Button: 3 (release), Col: 33-32=1, Row: 33-32=1
        let seq = b"\x1b[M\x03!!\x00";
        let (event, bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse X10");

        assert_eq!(bytes_consumed, 6);
        match event {
            VT100InputEvent::Mouse { action, .. } => {
                assert_eq!(action, VT100MouseAction::Release);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_x10_motion() {
        // X10: ESC [ M 35 50 50 (motion flag set, bit 5 = 32, so 3+32=35)
        // Button: 3+32=35 (motion), Col: 50-32=18, Row: 50-32=18
        let seq = b"\x1b[M\x232\\x00";
        let (event, bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse X10");

        assert_eq!(bytes_consumed, 6);
        match event {
            VT100InputEvent::Mouse { button, action, .. } => {
                assert_eq!(button, VT100MouseButton::Unknown);
                assert_eq!(action, VT100MouseAction::Motion);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_x10_with_shift() {
        // X10: ESC [ M 4 33 33 (left click with shift: button 0 + shift 4 = 4)
        // Button: 0 with shift, Col: 33-32=1, Row: 33-32=1
        let seq = b"\x1b[M\x04!!\x00";
        let (event, bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse X10");

        assert_eq!(bytes_consumed, 6);
        match event {
            VT100InputEvent::Mouse { modifiers, .. } => {
                assert!(modifiers.shift);
                assert!(!modifiers.ctrl);
                assert!(!modifiers.alt);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_x10_with_ctrl() {
        // X10: ESC [ M 16 33 33 (left click with ctrl: button 0 + ctrl 16 = 16)
        // Button: 0 with ctrl, Col: 33-32=1, Row: 33-32=1
        let seq = b"\x1b[M\x10!!\x00";
        let (event, bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse X10");

        assert_eq!(bytes_consumed, 6);
        match event {
            VT100InputEvent::Mouse { modifiers, .. } => {
                assert!(!modifiers.shift);
                assert!(modifiers.ctrl);
                assert!(!modifiers.alt);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_x10_with_alt() {
        // X10: ESC [ M 8 33 33 (left click with alt: button 0 + alt 8 = 8)
        // Button: 0 with alt, Col: 33-32=1, Row: 33-32=1
        let seq = b"\x1b[M\x08!!\x00";
        let (event, bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse X10");

        assert_eq!(bytes_consumed, 6);
        match event {
            VT100InputEvent::Mouse { modifiers, .. } => {
                assert!(!modifiers.shift);
                assert!(!modifiers.ctrl);
                assert!(modifiers.alt);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_x10_coordinates_1_based() {
        // Verify 1-based coordinates in X10 format
        // ESC [ M 0 (33 for col 1) (33 for row 1)
        let seq = b"\x1b[M\x00!!\x00";
        let (event, bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse X10");

        assert_eq!(bytes_consumed, 6);
        match event {
            VT100InputEvent::Mouse { pos, .. } => {
                assert_eq!(pos.col.as_u16(), 1, "Column should be 1-based");
                assert_eq!(pos.row.as_u16(), 1, "Row should be 1-based");
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_x10_large_coordinates() {
        // Test with larger coordinates: col 100, row 50
        // Col: 100 + 32 = 132 = byte value
        // Row: 50 + 32 = 82 = byte value
        let seq = b"\x1b[M\x00\x84R\x00";
        let (event, _) = parse_mouse_sequence(seq).expect("Should parse X10");

        match event {
            VT100InputEvent::Mouse { pos, .. } => {
                assert_eq!(pos.col.as_u16(), 100);
                assert_eq!(pos.row.as_u16(), 50);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_x10_incomplete_sequence() {
        // Incomplete: ESC [ M Cb Cx (missing Cy) - only 5 bytes
        let seq = b"\x1b[M\x00!";
        let result = parse_mouse_sequence(seq);
        assert!(result.is_none(), "Should not parse incomplete X10 sequence");
    }

    #[test]
    fn test_x10_too_short() {
        // Too short: ESC [ M (missing everything else)
        let seq = b"\x1b[M";
        let result = parse_mouse_sequence(seq);
        assert!(result.is_none(), "Should not parse too-short X10 sequence");
    }


    // RXVT Mouse Protocol Tests
    // Format: ESC [ Cb ; Cx ; Cy M (semicolon-separated decimal, not `<` prefixed)

    #[test]
    fn test_rxvt_left_click() {
        // RXVT: ESC [ 0 ; 1 ; 1 M (left click at col 1, row 1)
        let seq = b"\x1b[0;1;1M";
        let (event, bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse RXVT");

        assert_eq!(bytes_consumed, seq.len());
        match event {
            VT100InputEvent::Mouse {
                button,
                pos,
                action,
                modifiers,
            } => {
                assert_eq!(button, VT100MouseButton::Left);
                assert_eq!(pos.col.as_u16(), 1);
                assert_eq!(pos.row.as_u16(), 1);
                assert_eq!(action, VT100MouseAction::Press);
                assert!(!modifiers.shift && !modifiers.ctrl && !modifiers.alt);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_rxvt_middle_click() {
        // RXVT: ESC [ 1 ; 18 ; 8 M (middle click at col 18, row 8)
        let seq = b"\x1b[1;18;8M";
        let (event, bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse RXVT");

        assert_eq!(bytes_consumed, seq.len());
        match event {
            VT100InputEvent::Mouse { button, action, .. } => {
                assert_eq!(button, VT100MouseButton::Middle);
                assert_eq!(action, VT100MouseAction::Press);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_rxvt_right_click() {
        // RXVT: ESC [ 2 ; 13 ; 3 M (right click at col 13, row 3)
        let seq = b"\x1b[2;13;3M";
        let (event, bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse RXVT");

        assert_eq!(bytes_consumed, seq.len());
        match event {
            VT100InputEvent::Mouse { button, action, .. } => {
                assert_eq!(button, VT100MouseButton::Right);
                assert_eq!(action, VT100MouseAction::Press);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_rxvt_release() {
        // RXVT: ESC [ 3 ; 1 ; 1 M (release at col 1, row 1)
        let seq = b"\x1b[3;1;1M";
        let (event, bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse RXVT");

        assert_eq!(bytes_consumed, seq.len());
        match event {
            VT100InputEvent::Mouse { action, .. } => {
                assert_eq!(action, VT100MouseAction::Release);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_rxvt_motion() {
        // RXVT: ESC [ 35 ; 18 ; 18 M (motion flag set, button 3 + motion 32 = 35)
        let seq = b"\x1b[35;18;18M";
        let (event, bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse RXVT");

        assert_eq!(bytes_consumed, seq.len());
        match event {
            VT100InputEvent::Mouse { button, action, .. } => {
                assert_eq!(button, VT100MouseButton::Unknown);
                assert_eq!(action, VT100MouseAction::Motion);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_rxvt_with_shift() {
        // RXVT: ESC [ 4 ; 1 ; 1 M (left click with shift: button 0 + shift 4 = 4)
        let seq = b"\x1b[4;1;1M";
        let (event, bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse RXVT");

        assert_eq!(bytes_consumed, seq.len());
        match event {
            VT100InputEvent::Mouse { modifiers, .. } => {
                assert!(modifiers.shift);
                assert!(!modifiers.ctrl);
                assert!(!modifiers.alt);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_rxvt_with_ctrl() {
        // RXVT: ESC [ 16 ; 1 ; 1 M (left click with ctrl: button 0 + ctrl 16 = 16)
        let seq = b"\x1b[16;1;1M";
        let (event, bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse RXVT");

        assert_eq!(bytes_consumed, seq.len());
        match event {
            VT100InputEvent::Mouse { modifiers, .. } => {
                assert!(!modifiers.shift);
                assert!(modifiers.ctrl);
                assert!(!modifiers.alt);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_rxvt_with_alt() {
        // RXVT: ESC [ 8 ; 1 ; 1 M (left click with alt: button 0 + alt 8 = 8)
        let seq = b"\x1b[8;1;1M";
        let (event, bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse RXVT");

        assert_eq!(bytes_consumed, seq.len());
        match event {
            VT100InputEvent::Mouse { modifiers, .. } => {
                assert!(!modifiers.shift);
                assert!(!modifiers.ctrl);
                assert!(modifiers.alt);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_rxvt_coordinates_1_based() {
        // Verify 1-based coordinates in RXVT format
        let seq = b"\x1b[0;1;1M";
        let (event, bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse RXVT");

        assert_eq!(bytes_consumed, seq.len());
        match event {
            VT100InputEvent::Mouse { pos, .. } => {
                assert_eq!(pos.col.as_u16(), 1, "Column should be 1-based");
                assert_eq!(pos.row.as_u16(), 1, "Row should be 1-based");
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_rxvt_large_coordinates() {
        // Test with larger coordinates: col 100, row 50
        let seq = b"\x1b[0;100;50M";
        let (event, _) = parse_mouse_sequence(seq).expect("Should parse RXVT");

        match event {
            VT100InputEvent::Mouse { pos, .. } => {
                assert_eq!(pos.col.as_u16(), 100);
                assert_eq!(pos.row.as_u16(), 50);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_rxvt_incomplete_sequence() {
        // Incomplete: ESC [ 0 ; 1 (missing ; and Cy and M)
        let seq = b"\x1b[0;1";
        let result = parse_mouse_sequence(seq);
        assert!(result.is_none(), "Should not parse incomplete RXVT sequence");
    }

    #[test]
    fn test_rxvt_missing_terminator() {
        // Missing terminator: ESC [ 0 ; 1 ; 1 (no M)
        let seq = b"\x1b[0;1;1";
        let result = parse_mouse_sequence(seq);
        assert!(result.is_none(), "Should not parse RXVT without terminator");
    }

    #[test]
    fn test_rxvt_too_short() {
        // Too short: ESC [ (missing everything else)
        let seq = b"\x1b[";
        let result = parse_mouse_sequence(seq);
        assert!(result.is_none(), "Should not parse too-short RXVT sequence");
    }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sgr_left_click_press() {
        // From Phase 1: ESC[<0;1;1M (left click at top-left)
        let seq = b"\x1b[<0;1;1M";
        let (event, bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse");

        assert_eq!(bytes_consumed, seq.len());
        match event {
            VT100InputEvent::Mouse {
                button,
                pos,
                action,
                modifiers,
            } => {
                assert_eq!(button, VT100MouseButton::Left);
                assert_eq!(pos.col.as_u16(), 1);
                assert_eq!(pos.row.as_u16(), 1);
                assert_eq!(action, VT100MouseAction::Press);
                assert!(!modifiers.shift && !modifiers.ctrl && !modifiers.alt);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_sgr_left_click_release() {
        // From Phase 1: ESC[<0;1;1m (lowercase 'm' = release)
        let seq = b"\x1b[<0;1;1m";
        let (event, bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse");

        assert_eq!(bytes_consumed, seq.len());
        match event {
            VT100InputEvent::Mouse { action, .. } => {
                assert_eq!(action, VT100MouseAction::Release);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_sgr_scroll_up() {
        // From Phase 1: ESC[<66;37;14M (scroll up at col 37, row 14)
        // Button 66 = scroll up with modifiers (Shift bit set)
        let seq = b"\x1b[<66;37;14M";
        let (event, bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse");

        assert_eq!(bytes_consumed, seq.len());
        match event {
            VT100InputEvent::Mouse { action, pos, .. } => {
                assert_eq!(action, VT100MouseAction::Scroll(VT100ScrollDirection::Up));
                assert_eq!(pos.col.as_u16(), 37);
                assert_eq!(pos.row.as_u16(), 14);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_sgr_drag() {
        // Drag has bit 5 set (32): button 0 + drag = 32
        let seq = b"\x1b[<32;10;5M";
        let (event, bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse");

        assert_eq!(bytes_consumed, seq.len());
        match event {
            VT100InputEvent::Mouse { button, action, .. } => {
                assert_eq!(button, VT100MouseButton::Left);
                assert_eq!(action, VT100MouseAction::Drag);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_modifier_extraction() {
        // Ctrl+Left click: button 0 + Ctrl (16) = 16
        let seq = b"\x1b[<16;1;1M";
        let (event, bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse");

        assert_eq!(bytes_consumed, seq.len());
        match event {
            VT100InputEvent::Mouse { modifiers, .. } => {
                assert!(modifiers.ctrl);
                assert!(!modifiers.shift);
                assert!(!modifiers.alt);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_coordinates_are_1_based() {
        // Verify 1-based coordinates from Phase 1 findings
        let seq = b"\x1b[<0;1;1M";
        let (event, bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse");

        assert_eq!(bytes_consumed, seq.len());
        match event {
            VT100InputEvent::Mouse { pos, .. } => {
                assert_eq!(pos.col.as_u16(), 1, "Column should be 1-based");
                assert_eq!(pos.row.as_u16(), 1, "Row should be 1-based");
            }
            _ => panic!("Expected Mouse event"),
        }
    }
}
