// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Mouse input protocol constants for ANSI/CSI sequences.
//!
//! Defines byte values, bit masks, and protocol prefixes used by mouse input parsers
//! to handle SGR (modern), X10 (legacy), and RXVT (legacy) mouse protocols.

// ==================== Protocol Prefixes ====================

/// SGR mouse protocol prefix: `ESC [ <`
///
/// Modern standard mouse protocol with better support for modifiers and coordinates.
/// Format: `CSI < Cb ; Cx ; Cy M/m`
pub const MOUSE_SGR_PREFIX: &[u8] = b"\x1b[<";

/// SGR mouse protocol prefix length in bytes.
///
/// The prefix `ESC [ <` is 3 bytes: `\x1b` (ESC), `[`, `<`.
pub const MOUSE_SGR_PREFIX_LEN: usize = 3;

/// X10/Normal mouse protocol prefix: `ESC [ M`
///
/// Legacy protocol with fixed 6-byte sequences.
/// Format: `CSI M Cb Cx Cy`
pub const MOUSE_X10_PREFIX: &[u8] = b"\x1b[M";

// ==================== Action Codes ====================

/// SGR mouse protocol marker: `<` (60 dec, 3C hex).
///
/// Used in SGR extended mouse tracking sequences: `ESC [ < Cb ; Cx ; Cy M/m`
pub const MOUSE_SGR_MARKER: u8 = b'<';

/// SGR mouse press action: `M` (uppercase)
///
/// Indicates a mouse button press in SGR protocol.
pub const MOUSE_SGR_PRESS: u8 = b'M';

/// SGR mouse release action: `m` (lowercase)
///
/// Indicates a mouse button release in SGR protocol.
/// Only used in SGR; X10 and RXVT use button code 3 for release.
pub const MOUSE_SGR_RELEASE: u8 = b'm';

/// X10 mouse protocol marker: `M`
///
/// Used in X10 format to identify the protocol.
pub const MOUSE_X10_MARKER: u8 = b'M';

// ==================== Button Codes and Bit Masks ====================

/// Mask for button code bits (bits 0-1).
///
/// Extracts the base button code (0=left, 1=middle, 2=right, 3=release)
/// from the button byte.
pub const MOUSE_BUTTON_BITS_MASK: u16 = 0b0000_0011;

/// Left mouse button code (0)
///
/// Used in X10, RXVT, and SGR mouse protocols to represent button code 0.
pub const MOUSE_LEFT_BUTTON_CODE: u16 = 0;

/// Middle mouse button code (1)
///
/// Used in X10, RXVT, and SGR mouse protocols to represent button code 1.
pub const MOUSE_MIDDLE_BUTTON_CODE: u16 = 1;

/// Right mouse button code (2)
///
/// Used in X10, RXVT, and SGR mouse protocols to represent button code 2.
pub const MOUSE_RIGHT_BUTTON_CODE: u16 = 2;

/// Mouse button release code (3)
///
/// Used in X10, RXVT, and SGR mouse protocols to indicate button release.
/// When the button byte equals 3, it means no button is pressed.
pub const MOUSE_RELEASE_BUTTON_CODE: u16 = 3;

/// Mask for complete button code (bits 0-5).
///
/// Includes button bits (0-1), modifier bits (2-4), and drag bit (5).
/// Used to distinguish scroll events (button >= 64) from regular clicks.
pub const MOUSE_BUTTON_CODE_MASK: u16 = 0b0011_1111;

/// Mask for button code with scroll bit (bits 0-6).
///
/// Preserves all button information: button bits (0-1), modifier bits (2-4),
/// motion flag (5), and scroll bit (6). When scroll bit is set (value 64+),
/// indicates a scroll event. Used before distinguishing scroll from regular clicks.
pub const MOUSE_BASE_BUTTON_MASK: u16 = 0b0111_1111;

// ==================== Modifier Bit Flags ====================

/// Shift modifier flag (bit 2, value 4).
///
/// Set in button byte when Shift key is held during mouse event.
/// Note: Shift+Click is often intercepted by terminals for text selection.
pub const MOUSE_MODIFIER_SHIFT: u16 = 0b0000_0100;

/// Alt/Meta modifier flag (bit 3, value 8).
///
/// Set in button byte when Alt/Meta key is held during mouse event.
pub const MOUSE_MODIFIER_ALT: u16 = 0b0000_1000;

/// Ctrl modifier flag (bit 4, value 16).
///
/// Set in button byte when Ctrl key is held during mouse event.
pub const MOUSE_MODIFIER_CTRL: u16 = 0b0001_0000;

// ==================== Action Flags ====================

/// Drag/Motion flag (bit 5, value 32).
///
/// In SGR protocol: Set when mouse button is held and moved (drag).
/// In X10 protocol: Set when mouse moves without button held (motion).
pub const MOUSE_MOTION_FLAG: u16 = 0b0010_0000;

// ==================== Scroll Detection ====================

/// Scroll event threshold (value 64).
///
/// Scroll events have button codes >= 64.
/// Regular button codes are 0-3 (with modifiers 0-63).
/// Scroll codes: 64-67 (up/down/left/right) with optional modifiers.
pub const MOUSE_SCROLL_THRESHOLD: u16 = 0b0100_0000;

// ==================== Scroll Button Codes ====================

/// SGR scroll up button code (64)
///
/// Used in SGR mouse protocol for scroll up events.
pub const MOUSE_SCROLL_UP_BUTTON: u16 = 64;

/// SGR scroll down button code (68)
///
/// Used in SGR mouse protocol for scroll down events.
pub const MOUSE_SCROLL_DOWN_BUTTON: u16 = 68;

/// SGR scroll left button code (66)
///
/// Used in SGR mouse protocol for horizontal scroll left events.
pub const MOUSE_SCROLL_LEFT_BUTTON: u16 = 66;

/// SGR scroll right button code (67)
///
/// Used in SGR mouse protocol for horizontal scroll right events.
pub const MOUSE_SCROLL_RIGHT_BUTTON: u16 = 67;
