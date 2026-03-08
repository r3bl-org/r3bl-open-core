// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words URXVT

//! General-purpose [`ANSI`] constants for terminal modes and features.
//!
//! This module contains application-level constants that apply across both [`CSI`] and
//! [`ESC`] protocol layers. Unlike low-level sequencing details, these represent
//! terminal-wide capabilities and configuration options.
//!
//! See [constants module design] for the three-tier architecture.
//!
//! # Organization and Scope
//!
//! ### This Module ([`generic`]):
//! - **Classic [`DEC`] Modes (1-7)**: Original [`VT-100`]/[`VT-220`] terminal modes.
//! - **Modern Extensions (1000+)**: Additions by [`xterm`], [`rxvt-unicode`], etc.,
//!   leveraging the [`DEC`] private mode set/reset mechanism.
//! - **Terminal-Wide Features**: Raw mode, alternate screen, mouse tracking, and
//!   bracketed paste.
//! - **Cross-Protocol Constants**: Values and flags used by multiple protocol layers.
//!
//! ### Specialized Modules:
//! - **[`csi`]**: [`CSI`]-specific sequencing, [`SGR`] parameters, and cursor movement.
//! - **[`esc`]**: Simple, non-parameterized [`ESC`] sequences.
//! - **[`mouse`]**: Dedicated mouse protocol markers and bitmasks.
//!
//! # Examples
//!
//! You can toggle terminal features using either pre-built static strings (Tier 2) or the
//! dynamic [`CsiSequence`] builder (Tier 3).
//!
//! ### Option 1: Static Strings (Tier 2 - Zero Overhead)
//!
//! Use these for hardcoded, performance-critical feature toggles.
//! ```rust
//! use r3bl_tui::SGR_MOUSE_MODE_ENABLE_STR;
//!
//! let enable_mouse = SGR_MOUSE_MODE_ENABLE_STR; // "\x1b[?1006h"
//! ```
//!
//! ### Option 2: Dynamic Builder (Tier 3 - Type Safe)
//!
//! Use these when composing sequences or handling modes dynamically.
//! ```rust
//! use r3bl_tui::{CsiSequence, PrivateModeType, SGR_MOUSE_MODE};
//!
//! let seq = CsiSequence::EnablePrivateMode(PrivateModeType::Other(SGR_MOUSE_MODE));
//! ```
//!
//! ## References
//!
//! - [VT510 Programmer Reference] - original [`DEC`] terminal modes (1-7)
//! - [XTerm Control Sequences] - [`xterm`] and community extensions (1003+)
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`csi`]: crate::constants::csi
//! [`CSI`]: crate::CsiSequence
//! [`CsiSequence`]: crate::CsiSequence
//! [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
//! [`esc`]: crate::constants::esc
//! [`ESC`]: crate::EscSequence
//! [`EscSequence`]: crate::EscSequence
//! [`generic`]: crate::constants::generic
//! [`mouse`]: crate::constants::mouse
//! [`PrivateModeType`]: crate::PrivateModeType
//! [`rxvt-unicode`]: https://en.wikipedia.org/wiki/Rxvt-unicode
//! [`SGR`]: crate::SgrCode
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [`VT-220`]: https://en.wikipedia.org/wiki/VT220
//! [`xterm`]: https://en.wikipedia.org/wiki/Xterm
//! [constants module design]: mod@crate::constants#design
//! [VT510 Programmer Reference]: https://vt100.net/docs/vt510-rm/contents.html
//! [XTerm Control Sequences]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html

use crate::define_ansi_const;

// Terminal Behavior Modes (DEC modes 1-7).

/// Cursor Keys Mode ([`DECCKM - DEC mode 1`]): Controls how cursor keys are interpreted.
///
/// Value: `1`.
///
/// - When set: Cursor keys send [`ESC`] sequences (application mode).
/// - When reset: Cursor keys send normal sequences (cursor mode).
///
/// [`DECCKM - DEC mode 1`]: https://vt100.net/docs/vt510-rm/contents.html
/// [`ESC`]: crate::EscSequence
pub const DECCKM_CURSOR_KEYS: u16 = 1;

/// VT52 Mode ([`DECANM - DEC mode 2`]): Controls VT52 terminal compatibility mode.
///
/// Value: `2`.
///
/// - When set: Terminal operates in VT52 mode.
/// - When reset: Terminal operates in [`VT-100`]+ mode (default).
///
/// [`DECANM - DEC mode 2`]: https://vt100.net/docs/vt510-rm/contents.html
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
pub const DECANM_VT52_MODE: u16 = 2;

/// 132 Column Mode ([`DECCOLM - DEC mode 3`]): Controls terminal width.
///
/// Value: `3`.
///
/// - When set: Terminal width is 132 columns.
/// - When reset: Terminal width is 80 columns (default).
///
/// [`DECCOLM - DEC mode 3`]: https://vt100.net/docs/vt510-rm/contents.html
pub const DECCOLM_132_COLUMN: u16 = 3;

/// Smooth Scrolling Mode ([`DECSCLM - DEC mode 4`]): Controls scrolling behavior.
///
/// Value: `4`.
///
/// - When set: Scrolling is smooth/animated.
/// - When reset: Scrolling is instant (default).
///
/// [`DECSCLM - DEC mode 4`]: https://vt100.net/docs/vt510-rm/contents.html
pub const DECSCLM_SMOOTH_SCROLL: u16 = 4;

/// Reverse Video Mode ([`DECSCNM - DEC mode 5`]): Controls background inversion.
///
/// Value: `5`.
///
/// - When set: Reverse video (light text on dark background).
/// - When reset: Normal video (dark text on light background, default).
///
/// [`DECSCNM - DEC mode 5`]: https://vt100.net/docs/vt510-rm/contents.html
pub const DECSCNM_REVERSE_VIDEO: u16 = 5;

/// Origin Mode ([`DECOM - DEC mode 6`]): Controls margin relative positioning.
///
/// Value: `6`.
///
/// - When set: Cursor movement is relative to scroll margins.
/// - When reset: Cursor movement is absolute (default).
///
/// [`DECOM - DEC mode 6`]: https://vt100.net/docs/vt510-rm/contents.html
pub const DECOM_ORIGIN_MODE: u16 = 6;

/// Auto Wrap Mode ([`DECAWM - DEC mode 7`]): Controls margin wrapping.
///
/// Value: `7`.
///
/// - When set: Text wraps to next line at right margin (default).
/// - When reset: Cursor stays at right margin, text overwrites.
///
/// [`DECAWM - DEC mode 7`]: https://vt100.net/docs/vt510-rm/contents.html
pub const DECAWM_AUTO_WRAP: u16 = 7;

// Alternative Cursor Operations (xterm extensions).

/// Save Cursor Position - [`xterm private mode 1048`]
///
/// This is an alternative method to save cursor position:
/// 1. Using [`CsiSequence`] sequences - maps to `ESC [ ? 1048 h` (enable/set).
/// 2. Via [`ESC 7`] ([`DECSC`]) escape sequence.
///
/// [`CsiSequence`]: crate::CsiSequence
/// [`DECSC`]: https://vt100.net/docs/vt510-rm/contents.html
/// [`ESC 7`]: variant@crate::EscSequence::SaveCursor
/// [`xterm private mode 1048`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html
pub const SAVE_CURSOR_DEC: u16 = 1048;

// Screen Buffer and Display Modes (xterm extensions).

define_ansi_const!(@csi_str : ALT_SCREEN_ENABLE_STR = ["?1049h"] =>
    "Enable Alternate Screen Buffer" : "Enable alternate screen buffer sequence string."
);

define_ansi_const!(@csi_str : ALT_SCREEN_DISABLE_STR = ["?1049l"] =>
    "Disable Alternate Screen Buffer" : "Disable alternate screen buffer sequence string."
);

/// Alternate Screen Buffer Mode ([`xterm private mode 1049`]): Controls buffer choice.
///
/// Value: `1049`.
///
/// Controls whether terminal uses the main or alternate screen buffer.
///
/// - When set: Use alternate screen buffer (preserves main screen content).
/// - When reset: Use main screen buffer (default).
///
/// Used by full-screen applications ([`vim`], [`less`], [`tmux`], etc.) to avoid
/// cluttering shell history. When enabled, the application's output is rendered to a
/// separate buffer, and the original screen is restored when the application exits.
///
/// [`less`]: https://en.wikipedia.org/wiki/Less_(Unix)
/// [`tmux`]: https://en.wikipedia.org/wiki/Tmux
/// [`vim`]: https://en.wikipedia.org/wiki/Vim_(text_editor)
/// [`xterm private mode 1049`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html
pub const ALT_SCREEN_BUFFER: u16 = 1049;

// Input Modes - Mouse and Paste (xterm and community extensions).

/// Application Mouse Tracking ([`xterm private mode 1003`]): Enables event reporting.
///
/// Value: `1003`.
///
/// - When set: Terminal reports mouse clicks, movement, and scroll events.
/// - When reset: Mouse events are not reported (default).
///
/// When enabled, mouse interactions send special escape sequences to the application,
/// allowing interactive mouse support in [`TUI`] applications.
///
/// [`TUI`]: crate::tui::TerminalWindow::main_event_loop
/// [`xterm private mode 1003`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html
pub const APPLICATION_MOUSE_TRACKING: u16 = 1003;

/// URXVT Mouse Extension Mode ([`rxvt-unicode private mode 1015`]): legacy mouse format.
///
/// Value: `1015`.
///
/// Extended mouse reporting format introduced by the [`rxvt-unicode`] terminal emulator.
/// Other terminals adopted this encoding, and the mode number `1015` became the standard
/// identifier for "URXVT-style mouse encoding."
///
/// - When set: Use [`URXVT`] format for mouse position reporting.
/// - When reset: Use standard format (default).
///
/// This mode provides an alternative mouse coordinate encoding that extends the standard
/// [`X11`] mouse protocol to handle larger terminal sizes and additional button
/// information. Largely superseded by [`SGR_MOUSE_MODE`] (mode 1006).
///
/// [`rxvt-unicode private mode 1015`]:
///     https://invisible-island.net/xterm/ctlseqs/ctlseqs.html
/// [`rxvt-unicode`]: https://en.wikipedia.org/wiki/Rxvt-unicode
/// [`URXVT`]: https://en.wikipedia.org/wiki/Rxvt-unicode
/// [`X11`]: https://en.wikipedia.org/wiki/X11
pub const URXVT_MOUSE_EXTENSION: u16 = 1015;

/// [`SGR`] Mouse Mode ([`xterm private mode 1006`]): Modern extended mouse protocol.
///
/// Value: `1006`.
///
/// - When set: Use [`SGR`] format for mouse reporting.
/// - When reset: Use standard format (default).
///
/// This is the most modern and widely-supported mouse protocol extension, providing
/// support for mouse wheel events and proper handling of large terminal coordinates (>
/// `95` columns or rows). Supersedes [`URXVT_MOUSE_EXTENSION`] (mode `1015`).
///
/// [`SGR`]: crate::SgrCode
/// [`xterm private mode 1006`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html
pub const SGR_MOUSE_MODE: u16 = 1006;

define_ansi_const!(@csi_str : SGR_MOUSE_MODE_ENABLE_STR = ["?1006h"] =>
    "Enable SGR Mouse Mode" : "Enable SGR mouse mode sequence string."
);

define_ansi_const!(@csi_str : SGR_MOUSE_MODE_DISABLE_STR = ["?1006l"] =>
    "Disable SGR Mouse Mode" : "Disable SGR mouse mode sequence string."
);

/// Bracketed Paste Mode ([`xterm private mode 2004`]): Distinguish paste from input.
///
/// Value: `2004`.
///
/// - When set: Pasted text is wrapped with [`CSI`] bracket sequences.
/// - When reset: No special paste handling (default).
///
/// When enabled, text pasted from the clipboard is prefixed with `ESC [ 200 ~` and
/// suffixed with `ESC [ 201 ~`, allowing applications to identify and handle pasted
/// content differently from keyboard input. This prevents misinterpretation of special
/// characters in pasted content.
///
/// [`CSI`]: crate::CsiSequence
/// [`xterm private mode 2004`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html
pub const BRACKETED_PASTE_MODE: u16 = 2004;
