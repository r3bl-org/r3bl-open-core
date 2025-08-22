// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Operating System Command (OSC) codes for terminal control.
//!
//! OSC sequences provide communication between applications and the terminal emulator
//! for features that affect the terminal's operating system integration, such as
//! window titles, notifications, and hyperlinks.
//!
//! ## Structure
//! OSC sequences follow the pattern: `ESC ] code ; parameters ST`
//! - Start with ESC (0x1B) followed by `]`
//! - Numeric code identifying the command type
//! - Parameters separated by `;`
//! - End with String Terminator (ESC \\) or BEL (0x07)
//!
//! ## Common Uses
//! - **Window Management**: Set window title and tab names
//! - **Hyperlinks**: Create clickable links in terminal output
//! - **Notifications**: Send desktop notifications (terminal-dependent)
//! - **Clipboard**: Access system clipboard (security-restricted)
//!
//! ## Examples
//! - `ESC]0;My Title ESC\\` - Set both window title and tab name
//! - `ESC]2;Window Title ESC\\` - Set window title only
//! - `ESC]8;;https://example.com ESC\\Link Text ESC]8;; ESC\\` - Create hyperlink

// Common OSC sequence components for sending outgoing sequences

/// Generic OSC sequence start: ESC ] 
pub const OSC_START: &str = "\x1b]";
/// OSC 9;4 sequence prefix: ESC ] 9 ; 4 ;
pub const START: &str = "\x1b]9;4;";
/// OSC 8 hyperlink sequence prefix: ESC ] 8 ; ;
pub const OSC8_START: &str = "\x1b]8;;";
/// Sequence terminator: ESC \\ (String Terminator)
pub const END: &str = "\x1b\\";
/// Parameter delimiter within OSC sequences
pub const DELIMITER: char = ';';

/// Terminal title and tab control sequences for sending outgoing sequences
///
/// OSC 0 sequence: Set both window title and tab name (ESC ] 0 ;)
/// We only implement OSC 0 (title + tab). OSC 1 (icon only) and OSC 2
/// (title only) are not needed for modern terminal multiplexing where
/// consistent branding is preferred.
pub const OSC0_SET_TITLE_AND_TAB: &str = "\x1b]0;";

/// OSC 1 sequence: Set icon name (ESC ] 1 ;)
/// For testing compatibility - rarely used in modern terminals
pub const OSC1_SET_ICON: &str = "\x1b]1;";

/// OSC 2 sequence: Set window title only (ESC ] 2 ;)
/// For testing compatibility - OSC 0 is preferred
pub const OSC2_SET_TITLE: &str = "\x1b]2;";

/// Alternative terminator: BEL character (0x07)
/// Some terminals prefer this over the `STRING_TERMINATOR`
pub const BELL_TERMINATOR: &str = "\x07";

// OSC code numbers for parsing incoming sequences by the ANSI parser

/// OSC code 0: Set both window title and icon name
pub const OSC_CODE_TITLE_AND_ICON: &str = "0";

/// OSC code 1: Set icon name
pub const OSC_CODE_ICON: &str = "1";

/// OSC code 2: Set window title
pub const OSC_CODE_TITLE: &str = "2";

/// OSC code 8: Hyperlink
pub const OSC_CODE_HYPERLINK: &str = "8";
