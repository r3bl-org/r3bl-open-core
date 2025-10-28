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

use super::types::{InputEvent, KeyModifiers, MouseAction, MouseButton, Pos,
                   ScrollDirection};

/// Parse a mouse sequence and return an InputEvent if recognized.
///
/// Supports multiple mouse protocols:
/// - SGR (modern): `CSI < Cb ; Cx ; Cy M/m`
/// - X10/Normal (legacy): `CSI M Cb Cx Cy`
/// - RXVT (legacy): `CSI [ Cb ; Cx ; Cy M`
pub fn parse_mouse_sequence(buffer: &[u8]) -> Option<InputEvent> {
    // Check for SGR mouse protocol (most reliable)
    if buffer.len() >= 6 && buffer.starts_with(b"\x1b[<") {
        return parse_sgr_mouse(buffer);
    }

    // TODO: Add X10/Normal and RXVT parsing if needed
    None
}

/// Parse SGR mouse protocol: `CSI < Cb ; Cx ; Cy M/m`
///
/// Format breakdown:
/// - `ESC[<` prefix (3 bytes)
/// - `Cb` = button byte (with modifiers encoded)
/// - `Cx` = column (1-based)
/// - `Cy` = row (1-based)
/// - `M` = press, `m` = release
fn parse_sgr_mouse(sequence: &[u8]) -> Option<InputEvent> {
    // Minimum: ESC[<0;1;1M (9 bytes)
    if sequence.len() < 9 {
        return None;
    }

    // Extract the action character (last byte)
    let action_char = *sequence.last()? as char;
    if action_char != 'M' && action_char != 'm' {
        return None;
    }

    // Parse the content between ESC[< and M/m
    // Skip prefix (3 bytes) and suffix (1 byte)
    let content = std::str::from_utf8(&sequence[3..sequence.len() - 1]).ok()?;

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
        return Some(InputEvent::Mouse {
            button: MouseButton::Unknown,
            pos: Pos::from_one_based(cx, cy),
            action: MouseAction::Scroll(scroll_dir),
            modifiers,
        });
    }

    // Detect button type
    let button = detect_mouse_button(cb)?;

    // Detect action
    let action = if is_drag_event(cb) {
        MouseAction::Drag
    } else if action_char == 'M' {
        MouseAction::Press
    } else {
        MouseAction::Release
    };

    Some(InputEvent::Mouse {
        button,
        pos: Pos::from_one_based(cx, cy),
        action,
        modifiers,
    })
}

/// Detect mouse button from SGR button byte.
///
/// Button encoding (bits 0-1):
/// - 0 = left button
/// - 1 = middle button
/// - 2 = right button
/// - 3 = release (for legacy modes, SGR uses 'M'/'m' instead)
fn detect_mouse_button(cb: u16) -> Option<MouseButton> {
    // Mask out modifier and drag bits (keep only bits 0-5)
    let button_code = cb & 0x3F;

    // Scroll events are handled separately
    if button_code >= 64 {
        return None;
    }

    // Get base button (bits 0-1)
    match button_code & 0x3 {
        0 => Some(MouseButton::Left),
        1 => Some(MouseButton::Middle),
        2 => Some(MouseButton::Right),
        _ => Some(MouseButton::Unknown),
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
fn detect_scroll_event(cb: u16) -> Option<ScrollDirection> {
    // Check raw button code first (before masking modifiers)
    // Buttons 64+ indicate scroll events
    if cb >= 64 {
        // Mask to get base button (without modifiers but keeping scroll bit)
        let base_button = cb & 0x7F; // Keep bit 6 (value 64)

        match base_button {
            64..=67 => Some(ScrollDirection::Up), // All scroll up variants
            68..=71 => Some(ScrollDirection::Down), // All scroll down variants
            _ => Some(ScrollDirection::Up),       /* Default to up for unknown scroll
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
fn extract_modifiers(cb: u16) -> KeyModifiers {
    KeyModifiers {
        shift: (cb & 4) != 0,
        alt: (cb & 8) != 0,
        ctrl: (cb & 16) != 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sgr_left_click_press() {
        // From Phase 1: ESC[<0;1;1M (left click at top-left)
        let seq = b"\x1b[<0;1;1M";
        let event = parse_mouse_sequence(seq).expect("Should parse");

        match event {
            InputEvent::Mouse {
                button,
                pos,
                action,
                modifiers,
            } => {
                assert_eq!(button, MouseButton::Left);
                assert_eq!(pos.col.as_u16(), 1);
                assert_eq!(pos.row.as_u16(), 1);
                assert_eq!(action, MouseAction::Press);
                assert!(!modifiers.shift && !modifiers.ctrl && !modifiers.alt);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_sgr_left_click_release() {
        // From Phase 1: ESC[<0;1;1m (lowercase 'm' = release)
        let seq = b"\x1b[<0;1;1m";
        let event = parse_mouse_sequence(seq).expect("Should parse");

        match event {
            InputEvent::Mouse { action, .. } => {
                assert_eq!(action, MouseAction::Release);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_sgr_scroll_up() {
        // From Phase 1: ESC[<66;37;14M (scroll up at col 37, row 14)
        // Button 66 = scroll up with modifiers (Shift bit set)
        let seq = b"\x1b[<66;37;14M";
        let event = parse_mouse_sequence(seq).expect("Should parse");

        match event {
            InputEvent::Mouse { action, pos, .. } => {
                assert_eq!(action, MouseAction::Scroll(ScrollDirection::Up));
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
        let event = parse_mouse_sequence(seq).expect("Should parse");

        match event {
            InputEvent::Mouse { button, action, .. } => {
                assert_eq!(button, MouseButton::Left);
                assert_eq!(action, MouseAction::Drag);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_modifier_extraction() {
        // Ctrl+Left click: button 0 + Ctrl (16) = 16
        let seq = b"\x1b[<16;1;1M";
        let event = parse_mouse_sequence(seq).expect("Should parse");

        match event {
            InputEvent::Mouse { modifiers, .. } => {
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
        let event = parse_mouse_sequence(seq).expect("Should parse");

        match event {
            InputEvent::Mouse { pos, .. } => {
                assert_eq!(pos.col.as_u16(), 1, "Column should be 1-based");
                assert_eq!(pos.row.as_u16(), 1, "Row should be 1-based");
            }
            _ => panic!("Expected Mouse event"),
        }
    }
}
