// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Operating System Command (OSC) codes for terminal control.
//!
//! OSC sequences allow child processes to send commands to the terminal emulator
//! for features that affect the terminal's operating system integration, such as
//! window titles, notifications, and hyperlinks.
//!
//! ## Data Flow
//!
//! **Child process → PTY → Terminal emulator**: Child process sends OSC commands
//! to control terminal features (titles, hyperlinks, notifications, etc.)
//!
//! Unlike DSR sequences, OSC sequences are typically unidirectional - the child
//! process sends commands to the terminal but doesn't expect responses back.
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

// Common OSC sequence components for sending outgoing sequences.

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

// OSC code numbers for parsing incoming sequences by the ANSI parser.

/// OSC code 0: Set both window title and icon name
pub const OSC_CODE_TITLE_AND_ICON: &str = "0";

/// OSC code 1: Set icon name
pub const OSC_CODE_ICON: &str = "1";

/// OSC code 2: Set window title
pub const OSC_CODE_TITLE: &str = "2";

/// OSC code 8: Hyperlink
pub const OSC_CODE_HYPERLINK: &str = "8";

use crate::{core::common::fast_stringify::{BufTextStorage, FastStringify},
            generate_impl_display_for_fast_stringify};
use std::fmt;

/// OSC sequence builder enum that provides type-safe construction of Operating System
/// Command sequences.
///
/// This enum follows the same pattern as `CsiSequence` and `EscSequence`, providing a
/// structured way to build OSC sequences instead of manual string formatting.
///
/// OSC sequences follow the format: `ESC ] code ; parameters ST`
/// where ST is the String Terminator (ESC \ or BEL).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OscSequence {
    /// OSC 0: Set both window title and icon name
    /// Format: `ESC ] 0 ; title ST`
    SetTitleAndIcon(String),

    /// OSC 1: Set icon name only
    /// Format: `ESC ] 1 ; icon_name ST`
    SetIcon(String),

    /// OSC 2: Set window title only\
    /// Format: `ESC ] 2 ; title ST`
    SetTitle(String),

    /// OSC 8: Hyperlink start sequence
    /// Format: `ESC ] 8 ; id ; uri ST`
    /// The `id` parameter is optional and used for link identification
    HyperlinkStart { uri: String, id: Option<String> },

    /// OSC 8: Hyperlink end sequence\
    /// Format: `ESC ] 8 ; ; ST`
    /// This closes the hyperlink started by `HyperlinkStart`
    HyperlinkEnd,
}

impl FastStringify for OscSequence {
    fn write_to_buf(&self, acc: &mut BufTextStorage) -> fmt::Result {
        acc.push_str(OSC_START);
        match self {
            OscSequence::SetTitleAndIcon(title) => {
                acc.push_str(OSC_CODE_TITLE_AND_ICON);
                acc.push(DELIMITER);
                acc.push_str(title);
            }
            OscSequence::SetIcon(icon) => {
                acc.push_str(OSC_CODE_ICON);
                acc.push(DELIMITER);
                acc.push_str(icon);
            }
            OscSequence::SetTitle(title) => {
                acc.push_str(OSC_CODE_TITLE);
                acc.push(DELIMITER);
                acc.push_str(title);
            }
            OscSequence::HyperlinkStart { uri, id } => {
                acc.push_str(OSC_CODE_HYPERLINK);
                acc.push(DELIMITER);
                if let Some(link_id) = id {
                    acc.push_str(link_id);
                }
                acc.push(DELIMITER);
                acc.push_str(uri);
            }
            OscSequence::HyperlinkEnd => {
                acc.push_str(OSC_CODE_HYPERLINK);
                acc.push(DELIMITER);
                acc.push(DELIMITER);
            }
        }
        // Use BELL_TERMINATOR as the default terminator (more compatible)
        acc.push_str(BELL_TERMINATOR);
        Ok(())
    }

    fn write_buf_to_fmt(
        &self,
        acc: &BufTextStorage,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.write_str(&acc.clone())
    }
}

generate_impl_display_for_fast_stringify!(OscSequence);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_osc_sequence_set_title_and_icon() {
        let sequence = OscSequence::SetTitleAndIcon("My Title".to_string());
        let result = sequence.to_string();
        let expected = "\x1b]0;My Title\x07";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_osc_sequence_set_icon() {
        let sequence = OscSequence::SetIcon("Icon Name".to_string());
        let result = sequence.to_string();
        let expected = "\x1b]1;Icon Name\x07";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_osc_sequence_set_title() {
        let sequence = OscSequence::SetTitle("Window Title".to_string());
        let result = sequence.to_string();
        let expected = "\x1b]2;Window Title\x07";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_osc_sequence_hyperlink_start_with_id() {
        let sequence = OscSequence::HyperlinkStart {
            uri: "https://example.com".to_string(),
            id: Some("link1".to_string()),
        };
        let result = sequence.to_string();
        let expected = "\x1b]8;link1;https://example.com\x07";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_osc_sequence_hyperlink_start_without_id() {
        let sequence = OscSequence::HyperlinkStart {
            uri: "https://example.com".to_string(),
            id: None,
        };
        let result = sequence.to_string();
        let expected = "\x1b]8;;https://example.com\x07";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_osc_sequence_hyperlink_end() {
        let sequence = OscSequence::HyperlinkEnd;
        let result = sequence.to_string();
        let expected = "\x1b]8;;\x07";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_osc_sequence_empty_strings() {
        let sequence = OscSequence::SetTitle(String::new());
        let result = sequence.to_string();
        let expected = "\x1b]2;\x07";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_osc_sequence_special_characters() {
        let sequence = OscSequence::SetTitle("Title with spaces & symbols!".to_string());
        let result = sequence.to_string();
        let expected = "\x1b]2;Title with spaces & symbols!\x07";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_hyperlink_complete_sequence() {
        let start = OscSequence::HyperlinkStart {
            uri: "https://r3bl.com".to_string(),
            id: Some("r3bl".to_string()),
        };
        let end = OscSequence::HyperlinkEnd;

        let complete_link = format!("{start}Link Text{end}");
        let expected = "\x1b]8;r3bl;https://r3bl.com\x07Link Text\x1b]8;;\x07";
        assert_eq!(complete_link, expected);
    }
}
