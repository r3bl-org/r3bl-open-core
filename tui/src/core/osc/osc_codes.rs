// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Operating System Command ([`OSC`]) codes for terminal control.
//!
//! [`OSC`] sequences allow child processes to send commands to the terminal emulator
//! for features that affect the terminal's operating system integration, such as
//! window titles, notifications, and hyperlinks.
//!
//! # Data Flow
//!
//! **Child process → [`PTY`] → Terminal emulator**: Child process sends [`OSC`] commands
//! to control terminal features (titles, hyperlinks, notifications, etc.)
//!
//! Unlike [`DSR`] sequences, [`OSC`] sequences are typically unidirectional - the child
//! process sends commands to the terminal but doesn't expect responses back.
//!
//! # Structure
//! [`OSC`] sequences follow the pattern: `ESC ] code ; parameters ST`
//! - Start with [`ESC`] (0x1B) followed by `]`
//! - Numeric code identifying the command type
//! - Parameters separated by `;`
//! - End with String Terminator (ST): `ESC \` or BEL (0x07)
//!
//! # Common Uses
//! - **Window Management**: Set window title and tab names
//! - **Hyperlinks**: Create clickable links in terminal output
//! - **Notifications**: Send desktop notifications (terminal-dependent)
//! - **Clipboard**: Access system clipboard (security-restricted)
//!
//! # Examples
//! - `ESC ] 0 ; My Title ESC \` - Set both window title and tab name
//! - `ESC ] 2 ; Window Title ESC \` - Set window title only
//! - `ESC ] 8 ; ; https://example.com ESC \ Link Text ESC ] 8 ; ; ESC \` - Create
//!   hyperlink
//!
//! [`DSR`]: crate::DsrSequence
//! [`ESC`]: crate::EscSequence
//! [`OSC`]: crate::osc_codes::OscSequence
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`ST`]: OSC_TERMINATOR_ST

use crate::{core::common::fast_stringify::{BufTextStorage, FastStringify},
            define_ansi_const};
use std::fmt;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// OSC sequence components
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

define_ansi_const!(@osc_str : OSC_START = [""] => "OSC Start" : "Generic start: `ESC ]`." );

define_ansi_const!(@osc_str : OSC_PROGRESS_START = ["9;4;"] =>
    "Progress Start (OSC 9;4)" : "Progress start: `ESC ] 9 ; 4 ;`."
);

define_ansi_const!(@osc_str : OSC_HYPERLINK_START = ["8;;"] =>
    "Hyperlink Start (OSC 8)" : "Hyperlink start: `ESC ] 8 ; ;`."
);

define_ansi_const!(@osc_str : OSC_TITLE_AND_ICON_START = ["0;"] =>
    "Title and Icon Start (OSC 0)" : "Title and icon start: `ESC ] 0 ;`."
);

define_ansi_const!(@osc_str : OSC_ICON_START = ["1;"] =>
    "Icon Start (OSC 1)" : "Icon start: `ESC ] 1 ;`."
);

define_ansi_const!(@osc_str : OSC_TITLE_START = ["2;"] =>
    "Title Start (OSC 2)" : "Title start: `ESC ] 2 ;`."
);

define_ansi_const!(@esc_str : OSC_TERMINATOR_ST = ["\\"] =>
    "String Terminator (ST)" : "Standard ANSI String Terminator: `ESC \\`."
);

/// BEL Terminator: De-facto standard [`OSC`] terminator.
///
/// Value: `\x07` (`07` hex).
///
/// [`OSC`]: crate::osc_codes::OscSequence
pub const OSC_TERMINATOR_BEL: &str = "\x07";

/// Progress End: Semantic alias for [`OSC_TERMINATOR_ST`] (`ESC \`).
pub const OSC_PROGRESS_END: &str = OSC_TERMINATOR_ST;

/// Title End: Semantic alias for [`OSC_TERMINATOR_BEL`] (`\x07`).
pub const OSC_TITLE_END: &str = OSC_TERMINATOR_BEL;

/// Hyperlink End: Semantic alias for [`OSC_TERMINATOR_BEL`] (`\x07`).
pub const OSC_HYPERLINK_END: &str = OSC_TERMINATOR_BEL;

/// Parameter Delimiter: Semicolon `;` separating [`OSC`] parameters.
///
/// Value: `';'` (`3B` hex).
///
/// [`OSC`]: crate::osc_codes::OscSequence
pub const OSC_DELIMITER: char = ';';

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// OSC code numbers (for parsing)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// `0` - Code for Title and Icon
pub const OSC_CODE_TITLE_AND_ICON: &str = "0";

/// `1` - Code for Icon
pub const OSC_CODE_ICON: &str = "1";

/// `2` - Code for Title
pub const OSC_CODE_TITLE: &str = "2";

/// `8` - Code for Hyperlink
pub const OSC_CODE_HYPERLINK: &str = "8";

/// [`OSC`] ([`OSC` spec]) sequence builder enum that provides type-safe construction of
/// Operating System Command sequences.
///
/// This enum follows the same pattern as [`CsiSequence`] and [`EscSequence`], providing
/// a structured way to build [`OSC`] sequences instead of manual string formatting.
///
/// [`OSC`] sequences follow the format: `ESC ] code ; parameters ST`
/// where ST is the String Terminator (ST): `ESC \` or BEL.
///
/// [`CsiSequence`]: crate::CsiSequence
/// [`ESC`]: crate::EscSequence
/// [`EscSequence`]: crate::EscSequence
/// [`OSC` spec]: https://en.wikipedia.org/wiki/ANSI_escape_code#OSC
/// [`OSC`]: crate::osc_codes::OscSequence
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OscSequence {
    /// `ESC ] 0 ; title ST` - Set Title and Icon
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    SetTitleAndIcon(String),

    /// `ESC ] 1 ; icon ST` - Set Icon
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    SetIcon(String),

    /// `ESC ] 2 ; title ST` - Set Title
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    SetTitle(String),

    /// `ESC ] 8 ; id ; uri ST` - Hyperlink Start
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    HyperlinkStart { uri: String, id: Option<String> },

    /// `ESC ] 8 ; ; ST` - Hyperlink End
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    HyperlinkEnd,

    /// `ESC ] 9 ; 4 ; 1 ; percent ST` - Progress Update
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    ProgressUpdate(u8),
}

impl FastStringify for OscSequence {
    fn write_to_buf(&self, acc: &mut BufTextStorage) -> fmt::Result {
        acc.push_str(OSC_START);
        let terminator = match self {
            OscSequence::ProgressUpdate(percent) => {
                acc.push_str("9;4;1;");
                acc.push_str(&percent.to_string());
                OSC_PROGRESS_END
            }
            OscSequence::SetTitleAndIcon(title) => {
                acc.push_str(OSC_CODE_TITLE_AND_ICON);
                acc.push(OSC_DELIMITER);
                acc.push_str(title);
                OSC_TITLE_END
            }
            OscSequence::SetIcon(icon) => {
                acc.push_str(OSC_CODE_ICON);
                acc.push(OSC_DELIMITER);
                acc.push_str(icon);
                OSC_TITLE_END
            }
            OscSequence::SetTitle(title) => {
                acc.push_str(OSC_CODE_TITLE);
                acc.push(OSC_DELIMITER);
                acc.push_str(title);
                OSC_TITLE_END
            }
            OscSequence::HyperlinkStart { uri, id } => {
                acc.push_str(OSC_CODE_HYPERLINK);
                acc.push(OSC_DELIMITER);
                if let Some(link_id) = id {
                    acc.push_str(link_id);
                }
                acc.push(OSC_DELIMITER);
                acc.push_str(uri);
                OSC_HYPERLINK_END
            }
            OscSequence::HyperlinkEnd => {
                acc.push_str(OSC_CODE_HYPERLINK);
                acc.push(OSC_DELIMITER);
                acc.push(OSC_DELIMITER);
                OSC_HYPERLINK_END
            }
        };
        acc.push_str(terminator);
        ok!()
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
        let expected = format!("{OSC_START}0;My Title{OSC_TERMINATOR_BEL}");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_osc_sequence_set_icon() {
        let sequence = OscSequence::SetIcon("Icon Name".to_string());
        let result = sequence.to_string();
        let expected = format!("{OSC_START}1;Icon Name{OSC_TERMINATOR_BEL}");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_osc_sequence_set_title() {
        let sequence = OscSequence::SetTitle("Window Title".to_string());
        let result = sequence.to_string();
        let expected = format!("{OSC_START}2;Window Title{OSC_TERMINATOR_BEL}");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_osc_sequence_hyperlink_start_with_id() {
        let sequence = OscSequence::HyperlinkStart {
            uri: "https://example.com".to_string(),
            id: Some("link1".to_string()),
        };
        let result = sequence.to_string();
        let expected =
            format!("{OSC_START}8;link1;https://example.com{OSC_TERMINATOR_BEL}");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_osc_sequence_hyperlink_start_without_id() {
        let sequence = OscSequence::HyperlinkStart {
            uri: "https://example.com".to_string(),
            id: None,
        };
        let result = sequence.to_string();
        let expected = format!("{OSC_START}8;;https://example.com{OSC_TERMINATOR_BEL}");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_osc_sequence_hyperlink_end() {
        let sequence = OscSequence::HyperlinkEnd;
        let result = sequence.to_string();
        let expected = format!("{OSC_START}8;;{OSC_TERMINATOR_BEL}");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_osc_sequence_empty_strings() {
        let sequence = OscSequence::SetTitle(String::new());
        let result = sequence.to_string();
        let expected = format!("{OSC_START}2;{OSC_TERMINATOR_BEL}");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_osc_sequence_special_characters() {
        let sequence = OscSequence::SetTitle("Title with spaces & symbols!".to_string());
        let result = sequence.to_string();
        let expected =
            format!("{OSC_START}2;Title with spaces & symbols!{OSC_TERMINATOR_BEL}");
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
        let expected = format!(
            "{OSC_START}8;r3bl;https://r3bl.com{OSC_TERMINATOR_BEL}Link Text{OSC_START}8;;{OSC_TERMINATOR_BEL}"
        );
        assert_eq!(complete_link, expected);
    }
}
