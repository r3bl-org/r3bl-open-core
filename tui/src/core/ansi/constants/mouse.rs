// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words buttonless

//! Mouse input protocol constants for [`ANSI`]/[`CSI`] sequences.
//!
//! Defines byte values, bit masks, and protocol prefixes used by mouse input parsers
//! to handle [`SGR`] (modern), [`X10`] (legacy), and [`RXVT`] (legacy) mouse protocols.
//!
//! See [constants module design] for the three-tier architecture.
//!
//! # Bit Notation
//!
//! Throughout this file, bit indices are **0-based** and refer to the bit positions
//! counting from the right (least significant bit). For example, Bit 0 is the rightmost
//! bit (`0b0000_0001`) and Bit 7 is the leftmost bit (`0b1000_0000`).
//!
//! Here is a practical example of how these constants are bitwise OR'd together to form
//! a payload byte (e.g., decimal `35`, representing a buttonless hover motion event):
//!
//! ```text
//! `76543210` - Bit positions
//! `00100000` (Decimal `32`) : Motion Flag (Bit 5 is set)
//! `00000011` (Decimal `3`)  : Unknown/Release Button (Bits 0 and 1 are set)
//! `--------`
//! `00100011` (Decimal `35`) : Final payload byte (`32 | 3`)
//! ```
//!
//! ### Bitmask arithmetic operations
//!
//! *(See also: [`keyboard` module docs] for a contrasting example of where arithmetic
//! addition is required by the VT100 spec).*
//!
//! When combining bitmasks (like applying a modifier to a button):
//! - **always** use bitwise OR (`|` or `|=`)
//! - **do not** use of arithmetic addition (`+` or `+=`).
//!
//! While `32 + 3 = 35` and `32 | 3 = 35` produce the same result when bits do not
//! overlap, arithmetic addition is unsafe if a flag is accidentally applied twice. For
//! example, applying the [`MOUSE_MOTION_FLAG`] (`32`) twice:
//! - **Unsafe (`+`)**: `32 + 32 = 64`. This causes a binary carry-over into Bit 6,
//!   corrupting the value and incorrectly triggering the [`MOUSE_SCROLL_THRESHOLD`].
//! - **Safe (`|`)**: `32 | 32 = 32`. Bitwise OR guarantees the bit is simply turned on,
//!   preventing state corruption and unintended side effects.
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`CSI`]: crate::CsiSequence
//! [`keyboard` module docs]: mod@crate::core::ansi::vt_100_terminal_input_parser::keyboard#how-bitmask-encoding-for-modifiers-works
//! [`RXVT`]: https://en.wikipedia.org/wiki/Rxvt
//! [`SGR`]: crate::SgrCode
//! [`X10`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
//! [constants module design]: mod@crate::constants#design

// ==================== Protocol Prefixes ====================

/// [`SGR`] Mouse Protocol Prefix: Modern mouse tracking prefix `ESC [ <`.
///
/// Format: `CSI < Cb ; Cx ; Cy M/m`
///
/// [`SGR`]: crate::SgrCode
pub const MOUSE_SGR_PREFIX: &[u8] = b"\x1b[<";

/// [`SGR`] Mouse Prefix Length: Number of bytes in the [`SGR`] prefix.
///
/// Prefix `ESC [ <` is 3 bytes: [`ESC`] (`1B` hex), `[`, `<`.
///
/// [`ESC`]: crate::ANSI_ESC
/// [`SGR`]: crate::SgrCode
pub const MOUSE_SGR_PREFIX_LEN: usize = 3;

/// [`SGR`] Minimum Mouse Sequence Length: `ESC [ < 0 ; 1 ; 1 M` (9 bytes).
///
/// [`SGR`]: crate::SgrCode
pub const MOUSE_SGR_MIN_LEN: usize = 9;

/// [`X10`]/Legacy Mouse Protocol Prefix: Legacy mouse tracking prefix `ESC [ M`.
///
/// Format: `CSI M Cb Cx Cy`
///
/// [`X10`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
pub const MOUSE_X10_PREFIX: &[u8] = b"\x1b[M";

/// [`X10`] Minimum Mouse Sequence Length: `ESC [ M Cb Cx Cy` (6 bytes).
///
/// [`X10`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
pub const MOUSE_X10_MIN_LEN: usize = 6;

/// [`X10`] Coordinate Offset: Coordinates are transmitted as `(value + 32)` to ensure
/// they remain printable [`ASCII`] characters.
///
/// Value: `32`.
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
/// [`X10`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
pub const MOUSE_X10_COORD_OFFSET: u16 = 32;

/// [`RXVT`] Minimum Mouse Sequence Length: `ESC [ 0 ; 1 ; 1 M` (8 bytes).
///
/// [`RXVT`]: https://en.wikipedia.org/wiki/Rxvt
pub const MOUSE_RXVT_MIN_LEN: usize = 8;

// ==================== Action Codes ====================

/// [`SGR`] Mouse Marker ([`SGR`]): The `<` byte (`60` dec, `3C` hex).
///
/// Value: `'<'` dec, `3C` hex.
///
/// Sequence: `ESC [ < Cb ; Cx ; Cy M/m`.
///
/// [`SGR`]: crate::SgrCode
pub const MOUSE_SGR_MARKER: u8 = b'<';

/// [`SGR`] Mouse Press Action ([`SGR`]): `M` (uppercase) indicates button press.
///
/// Value: `'M'` dec, `4D` hex.
///
/// Sequence: `CSI < Cb ; Cx ; Cy M`.
///
/// [`SGR`]: crate::SgrCode
pub const MOUSE_SGR_PRESS: u8 = b'M';

/// [`SGR`] Mouse Release Action ([`SGR`]): `m` (lowercase) indicates button release.
///
/// Value: `'m'` dec, `6D` hex.
///
/// Sequence: `CSI < Cb ; Cx ; Cy m`.
///
/// Only used in [`SGR`]; [`X10`] and [`RXVT`] use button code `3` for release.
///
/// [`RXVT`]: https://en.wikipedia.org/wiki/Rxvt
/// [`SGR`]: crate::SgrCode
/// [`X10`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
pub const MOUSE_SGR_RELEASE: u8 = b'm';

/// [`X10`] Mouse Protocol Marker ([`X10`]): `M` identifies the [`X10`] format.
///
/// Value: `'M'` dec, `4D` hex.
///
/// Sequence: `CSI M Cb Cx Cy`.
///
/// [`X10`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
pub const MOUSE_X10_MARKER: u8 = b'M';

// ==================== Button Codes and Bit Masks ====================

/// Button Bits Mask ([`ANSI`]): Extracts base button code from bits 0-1.
///
/// Bit pattern: `0b0000_0011` (0=left, 1=middle, 2=right, 3=release).
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const MOUSE_BUTTON_BITS_MASK: u16 = 0b0000_0011;

/// Left Mouse Button Code ([`ANSI`]): Button code `0` for left click.
///
/// Value: `0`.
///
/// Used in [`X10`], [`RXVT`], and [`SGR`] mouse protocols.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`RXVT`]: https://en.wikipedia.org/wiki/Rxvt
/// [`SGR`]: crate::SgrCode
/// [`X10`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
pub const MOUSE_LEFT_BUTTON_CODE: u16 = 0;

/// Middle Mouse Button Code ([`ANSI`]): Button code `1` for middle click.
///
/// Value: `1`.
///
/// Used in [`X10`], [`RXVT`], and [`SGR`] mouse protocols.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`RXVT`]: https://en.wikipedia.org/wiki/Rxvt
/// [`SGR`]: crate::SgrCode
/// [`X10`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
pub const MOUSE_MIDDLE_BUTTON_CODE: u16 = 1;

/// Right Mouse Button Code ([`ANSI`]): Button code `2` for right click.
///
/// Value: `2`.
///
/// Used in [`X10`], [`RXVT`], and [`SGR`] mouse protocols.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`RXVT`]: https://en.wikipedia.org/wiki/Rxvt
/// [`SGR`]: crate::SgrCode
/// [`X10`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
pub const MOUSE_RIGHT_BUTTON_CODE: u16 = 2;

/// Mouse Button Release / Unknown Code ([`ANSI`]): Button code `3` indicates no button
/// pressed.
///
/// Value: `3`.
///
/// Used in [`X10`], [`RXVT`], and [`SGR`] mouse protocols. When combined with the
/// [`MOUSE_MOTION_FLAG`], it represents buttonless hover movement instead of a release.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`RXVT`]: https://en.wikipedia.org/wiki/Rxvt
/// [`SGR`]: crate::SgrCode
/// [`X10`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
pub const MOUSE_RELEASE_BUTTON_CODE: u16 = 3;

/// Button Code Mask ([`ANSI`]): Extracts complete button code from bits 0-5.
///
/// Bit pattern: `0b0011_1111` - button (0-1), modifier (2-4), drag (5).
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const MOUSE_BUTTON_CODE_MASK: u16 = 0b0011_1111;

/// Base Button Mask ([`ANSI`]): Extracts button code with scroll bit from bits 0-6.
///
/// Bit pattern: `0b0111_1111` - button (0-1), modifier (2-4), motion (5), scroll (6).
///
/// When scroll bit is set (value `64`+), indicates a scroll event.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const MOUSE_BASE_BUTTON_MASK: u16 = 0b0100_0011;

// ==================== Modifier Bit Flags ====================

/// Shift Modifier Flag ([`ANSI`]): Bit 2, value `4`.
///
/// Bit pattern: `0b0000_0100` - set when Shift is held during mouse event.
///
/// Note: Shift+Click is often intercepted by terminals for text selection.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const MOUSE_MODIFIER_SHIFT: u16 = 0b0000_0100;

/// Alt/Meta Modifier Flag ([`ANSI`]): Bit 3, value `8`.
///
/// Bit pattern: `0b0000_1000` - set when Alt/Meta is held during mouse event.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const MOUSE_MODIFIER_ALT: u16 = 0b0000_1000;

/// Ctrl Modifier Flag ([`ANSI`]): Bit 4, value `16`.
///
/// Bit pattern: `0b0001_0000` - set when Ctrl is held during mouse event.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const MOUSE_MODIFIER_CTRL: u16 = 0b0001_0000;

// ==================== Action Flags ====================

/// Motion / Drag Flag ([`ANSI`]): Bit 5, value `32`.
///
/// Bit pattern: `0b0010_0000` - Indicates mouse movement (hover or drag).
///
/// When set alongside a specific button code (0, 1, 2), it indicates a drag.
/// When set alongside the release/unknown button code (3), it indicates buttonless hover.
/// Applies across [`SGR`], [`X10`], and [`RXVT`] protocols.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`RXVT`]: https://en.wikipedia.org/wiki/Rxvt
/// [`SGR`]: crate::SgrCode
/// [`X10`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
pub const MOUSE_MOTION_FLAG: u16 = 0b0010_0000;

// ==================== Scroll Detection ====================

/// Scroll Event Threshold ([`ANSI`]): Value `64` separates scroll from click events.
///
/// Bit pattern: `0b0100_0000` - button codes >= `64` are scroll events.
///
/// Scroll codes: `64`-`67` (up/down/left/right) with optional modifiers.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const MOUSE_SCROLL_THRESHOLD: u16 = 0b0100_0000;

// ==================== Scroll Button Codes ====================

/// [`SGR`] Scroll Up ([`SGR`]): Button code `64` for scroll up events.
///
/// Value: `64`.
///
/// Used in [`SGR`] mouse protocol.
///
/// [`SGR`]: crate::SgrCode
pub const MOUSE_SCROLL_UP_BUTTON: u16 = 64;

/// [`SGR`] Scroll Down ([`SGR`]): Button code `65` for scroll down events.
///
/// Value: `65`.
///
/// Used in [`SGR`] mouse protocol.
///
/// [`SGR`]: crate::SgrCode
pub const MOUSE_SCROLL_DOWN_BUTTON: u16 = 65;

/// [`SGR`] Scroll Left ([`SGR`]): Button code `66` for horizontal scroll left events.
///
/// Value: `66`.
///
/// Used in [`SGR`] mouse protocol.
///
/// [`SGR`]: crate::SgrCode
pub const MOUSE_SCROLL_LEFT_BUTTON: u16 = 66;

/// [`SGR`] Scroll Right ([`SGR`]): Button code `67` for horizontal scroll right events.
///
/// Value: `67`.
///
/// Used in [`SGR`] mouse protocol.
///
/// [`SGR`]: crate::SgrCode
pub const MOUSE_SCROLL_RIGHT_BUTTON: u16 = 67;
