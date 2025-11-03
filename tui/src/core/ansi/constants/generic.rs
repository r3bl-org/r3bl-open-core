// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! General-purpose ANSI escape sequence constants and terminal mode parameters.
//!
//! This module contains constants for terminal features and modes that apply across
//! multiple sequence types (both CSI and ESC sequences). These are "application-level"
//! or "feature-level" constants that represent terminal capabilities and configuration
//! options, rather than low-level protocol sequencing details.
//!
//! ## Distinction from CSI-Specific Constants
//!
//! **This module** (`protocols/generic_ansi_constants.rs`):
//! - Terminal modes and features (raw mode, alternate screen, mouse, paste)
//! - DEC private mode numbers that apply at the terminal level
//! - Constants used by multiple protocol layers
//! - Feature configuration flags
//!
//! **CSI module** (`csi_codes/csi_constants.rs`):
//! - CSI-specific sequencing (command characters, CSI components)
//! - SGR parameter values for text formatting
//! - Cursor movement and erase command definitions
//! - Sequences that are exclusively CSI-based
//!
//! ## Example Usage
//!
//! Setting up terminal modes:
//! ```rust
//! use r3bl_tui::{CsiSequence, PrivateModeType, SGR_MOUSE_MODE, BRACKETED_PASTE_MODE};
//!
//! // Enable mouse tracking
//! let seq1 = CsiSequence::EnablePrivateMode(PrivateModeType::Other(SGR_MOUSE_MODE));
//! // Enable bracketed paste mode
//! let seq2 = CsiSequence::EnablePrivateMode(PrivateModeType::Other(BRACKETED_PASTE_MODE));
//! ```

// Terminal Behavior Modes (DEC modes 1-7).

/// Cursor Keys Mode (DECCKM) - DEC mode 1
///
/// Controls how cursor keys are interpreted by the terminal.
///
/// - When set: Cursor keys send ESC sequences (application mode)
/// - When reset: Cursor keys send normal sequences (cursor mode)
pub const DECCKM_CURSOR_KEYS: u16 = 1;

/// VT52 Mode (DECANM) - DEC mode 2
///
/// Controls VT52 terminal compatibility mode.
///
/// - When set: Terminal operates in VT52 mode
/// - When reset: Terminal operates in VT100+ mode (default)
pub const DECANM_VT52_MODE: u16 = 2;

/// 132 Column Mode (DECCOLM) - DEC mode 3
///
/// Controls terminal width (column count).
///
/// - When set: Terminal width is 132 columns
/// - When reset: Terminal width is 80 columns (default)
pub const DECCOLM_132_COLUMN: u16 = 3;

/// Smooth Scrolling Mode (DECSCLM) - DEC mode 4
///
/// Controls scrolling animation behavior.
///
/// - When set: Scrolling is smooth/animated
/// - When reset: Scrolling is instant (default)
pub const DECSCLM_SMOOTH_SCROLL: u16 = 4;

/// Reverse Video Mode (DECSCNM) - DEC mode 5
///
/// Controls whether the terminal displays in reverse video (inverted colors).
///
/// - When set: Reverse video (light text on dark background)
/// - When reset: Normal video (dark text on light background, default)
pub const DECSCNM_REVERSE_VIDEO: u16 = 5;

/// Origin Mode (DECOM) - DEC mode 6
///
/// Controls how cursor positioning is interpreted relative to margins.
///
/// - When set: Cursor movement is relative to scroll margins
/// - When reset: Cursor movement is absolute (default)
pub const DECOM_ORIGIN_MODE: u16 = 6;

/// Auto Wrap Mode (DECAWM) - DEC mode 7
///
/// Controls text wrapping at the right margin.
///
/// - When set: Text wraps to next line at right margin (default)
/// - When reset: Cursor stays at right margin, text overwrites
pub const DECAWM_AUTO_WRAP: u16 = 7;

// Alternative Cursor Operations

/// Save Cursor Position (DEC private) - DEC mode 1048
///
/// Alternative method to save cursor position using CSI sequences.
/// Maps to `ESC [ ? 1048 h` (enable/set).
///
/// Also available via ESC 7 (DECSC) escape sequence.
pub const SAVE_CURSOR_DEC: u16 = 1048;

// Screen Buffer and Display Modes

/// Alternate Screen Buffer Mode - DEC mode 1049
///
/// Controls whether terminal uses the main or alternate screen buffer.
///
/// - When set: Use alternate screen buffer (preserves main screen content)
/// - When reset: Use main screen buffer (default)
///
/// Used by full-screen applications (vim, less, tmux, etc.) to avoid cluttering
/// shell history. When enabled, the application's output is rendered to a separate
/// buffer, and the original screen is restored when the application exits.
pub const ALT_SCREEN_BUFFER: u16 = 1049;

// Input Modes (Mouse and Paste)

/// Application Mouse Tracking Mode - DEC mode 1003
///
/// Enables mouse event reporting to the application.
///
/// - When set: Terminal reports mouse clicks, movement, and scroll events
/// - When reset: Mouse events are not reported (default)
///
/// When enabled, mouse interactions send special escape sequences to the
/// application, allowing interactive mouse support in TUI applications.
pub const APPLICATION_MOUSE_TRACKING: u16 = 1003;

/// URXVT Mouse Extension Mode - DEC mode 1015
///
/// Extended mouse reporting format compatible with URXVT terminal.
///
/// - When set: Use URXVT format for mouse position reporting
/// - When reset: Use standard format (default)
///
/// This mode provides an alternative mouse coordinate encoding that extends
/// the standard X11 mouse protocol to handle larger terminal sizes and
/// additional button information.
pub const URXVT_MOUSE_EXTENSION: u16 = 1015;

/// SGR Mouse Mode (Extended Mouse Protocol) - DEC mode 1006
///
/// Modern extended mouse reporting format using SGR (Select Graphic Rendition) encoding.
///
/// - When set: Use SGR format for mouse reporting
/// - When reset: Use standard format (default)
///
/// This is the most modern and widely-supported mouse protocol extension,
/// providing support for mouse wheel events and proper handling of large terminal
/// coordinates (> 95 columns or rows).
pub const SGR_MOUSE_MODE: u16 = 1006;

/// Bracketed Paste Mode - DEC mode 2004
///
/// Enables distinction between pasted text and keyboard input.
///
/// - When set: Pasted text is wrapped with escape sequences (OSC 52)
/// - When reset: No special paste handling (default)
///
/// When enabled, text pasted from the clipboard is prefixed with `ESC[200~` and
/// suffixed with `ESC[201~`, allowing applications to identify and handle pasted
/// content differently from keyboard input. This prevents misinterpretation of
/// special characters in pasted content.
pub const BRACKETED_PASTE_MODE: u16 = 2004;

