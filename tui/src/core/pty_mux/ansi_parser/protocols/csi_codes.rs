// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Control Sequence Introducer (CSI) codes for terminal control.
//!
//! CSI sequences are the most common type of ANSI escape sequences used in modern
//! terminals. They provide parameterized control over cursor movement, text formatting,
//! colors, and display manipulation.
//!
//! ## Evolution from ESC Sequences
//!
//! CSI sequences evolved from the simpler direct ESC sequences to provide greater
//! flexibility:
//!
//! - **ESC sequences** (the predecessors): Simple, non-parameterized commands like `ESC
//!   7` (save cursor) or `ESC D` (move down one line). See [`esc_codes`] for details.
//! - **CSI sequences** (modern approach): Parameterized commands like `ESC[s` (save
//!   cursor) or `ESC[5B` (move down 5 lines). The parameters make them much more
//!   flexible.
//!
//! Many operations can be performed using either approach for backward compatibility.
//! Modern applications typically prefer CSI for their flexibility.
//!
//! [`esc_codes`]: crate::ansi_parser::esc_codes
//!
//! ## Structure
//! CSI sequences follow the pattern: `ESC [ parameters final_character`
//! - Start with ESC (0x1B) followed by `[`
//! - Optional numeric parameters separated by `;`
//! - End with a single letter that determines the action
//!
//! ## Common Uses
//! - **Cursor Movement**: Move cursor to specific positions or by relative amounts
//! - **Text Formatting**: Apply colors, bold, italic, underline, and other text
//!   attributes
//! - **Display Control**: Clear screen/lines, scroll content, save/restore cursor
//!   position
//! - **Terminal Modes**: Configure terminal behavior and features
//!
//! ## Examples
//! - `ESC[2J` - Clear entire screen
//! - `ESC[1;5H` - Move cursor to row 1, column 5
//! - `ESC[31m` - Set text color to red
//! - `ESC[1A` - Move cursor up 1 line

use std::{cmp::max,
          fmt::{self, Display}};

use super::{super::{param_utils::ParamsExt,
                    term_units::{TermCol, TermRow, term_row}},
            dsr_codes::DsrRequestType};
use crate::{BufTextStorage, ColIndex, ColWidth, Length, RowHeight, RowIndex, WriteToBuf,
            col, height, len, row, width};

// CSI sequence components.

/// CSI sequence start: ESC [
pub const CSI_START: &str = "\x1b[";

/// Private mode prefix for CSI sequences
pub const CSI_PRIVATE_MODE_PREFIX: char = '?';

/// Parameter separator in CSI sequences
pub const CSI_PARAM_SEPARATOR: char = ';';

// Cursor Movement.

/// CSI A: Cursor Up (CUU)
/// Moves cursor up by n lines (default 1)
pub const CUU_CURSOR_UP: char = 'A';

/// CSI B: Cursor Down (CUD)
/// Moves cursor down by n lines (default 1)
pub const CUD_CURSOR_DOWN: char = 'B';

/// CSI C: Cursor Forward/Right (CUF)
/// Moves cursor forward by n columns (default 1)
pub const CUF_CURSOR_FORWARD: char = 'C';

/// CSI D: Cursor Backward/Left (CUB)
/// Moves cursor backward by n columns (default 1)
pub const CUB_CURSOR_BACKWARD: char = 'D';

/// CSI E: Cursor Next Line (CNL)
/// Moves cursor to beginning of line n lines down (default 1)
pub const CNL_CURSOR_NEXT_LINE: char = 'E';

/// CSI F: Cursor Previous Line (CPL)
/// Moves cursor to beginning of line n lines up (default 1)
pub const CPL_CURSOR_PREV_LINE: char = 'F';

/// CSI G: Cursor Horizontal Absolute (CHA)
/// Moves cursor to column n (default 1)
pub const CHA_CURSOR_COLUMN: char = 'G';

/// CSI H: Cursor Position (CUP)
/// Moves cursor to row n, column m (default 1,1)
pub const CUP_CURSOR_POSITION: char = 'H';

/// CSI f: Horizontal and Vertical Position (HVP)
/// Same as CUP - moves cursor to row n, column m (default 1,1)
pub const HVP_CURSOR_POSITION: char = 'f';

// Erasing.

/// CSI J: Erase in Display (ED)
/// 0 = erase from cursor to end of screen (default)
/// 1 = erase from start of screen to cursor
/// 2 = erase entire screen
/// 3 = erase entire screen and scrollback
pub const ED_ERASE_DISPLAY: char = 'J';

/// CSI K: Erase in Line (EL)
/// 0 = erase from cursor to end of line (default)
/// 1 = erase from start of line to cursor
/// 2 = erase entire line
pub const EL_ERASE_LINE: char = 'K';

// Scrolling.

/// CSI S: Scroll Up (SU)
/// Scrolls text up by n lines (default 1)
pub const SU_SCROLL_UP: char = 'S';

/// CSI T: Scroll Down (SD)
/// Scrolls text down by n lines (default 1)
pub const SD_SCROLL_DOWN: char = 'T';
/// DECSTBM - Set Top and Bottom Margins - ESC [ top ; bottom r
pub const DECSTBM_SET_MARGINS: char = 'r';

// Line Operations.

/// CSI L: Insert Line (IL)
/// Inserts one or more blank lines, starting at the cursor
/// Lines below cursor and in scrolling region move down
pub const IL_INSERT_LINE: char = 'L';

/// CSI M: Delete Line (DL)
/// Deletes one or more lines in the scrolling region, starting with cursor line
/// Lines below cursor move up, blank lines added at bottom
pub const DL_DELETE_LINE: char = 'M';

// Character Operations.

/// CSI P: Delete Character (DCH)
/// Deletes one or more characters on current line
/// Characters to the right shift left, blanks inserted at end
pub const DCH_DELETE_CHAR: char = 'P';

/// CSI @: Insert Character (ICH)
/// Inserts one or more blank characters at cursor position
/// Characters to the right shift right, rightmost characters lost
pub const ICH_INSERT_CHAR: char = '@';

/// CSI X: Erase Character (ECH)
/// Erases one or more characters at cursor position
/// Characters are replaced with blanks, no shifting occurs
pub const ECH_ERASE_CHAR: char = 'X';

// Additional Cursor Positioning.

/// CSI d: Vertical Position Absolute (VPA)
/// Moves cursor to specified row (default 1)
/// Horizontal position unchanged
pub const VPA_VERTICAL_POSITION: char = 'd';

// Text Formatting (SGR).

/// CSI m: Select Graphic Rendition (SGR)
/// Sets colors and text attributes
pub const SGR_SET_GRAPHICS: char = 'm';

// SGR Parameters.

/// Reset all attributes
pub const SGR_RESET: u16 = 0;

/// Bold/Bright
pub const SGR_BOLD: u16 = 1;

/// Dim/Faint
pub const SGR_DIM: u16 = 2;

/// Italic
pub const SGR_ITALIC: u16 = 3;

/// Underline
pub const SGR_UNDERLINE: u16 = 4;

/// Slow Blink
pub const SGR_BLINK: u16 = 5;

/// Rapid Blink
pub const SGR_RAPID_BLINK: u16 = 6;

/// Reverse/Inverse
pub const SGR_REVERSE: u16 = 7;

/// Hidden/Conceal
pub const SGR_HIDDEN: u16 = 8;

/// Strikethrough
pub const SGR_STRIKETHROUGH: u16 = 9;

/// Reset Bold/Dim
pub const SGR_RESET_BOLD_DIM: u16 = 22;

/// Reset Italic
pub const SGR_RESET_ITALIC: u16 = 23;

/// Reset Underline
pub const SGR_RESET_UNDERLINE: u16 = 24;

/// Reset Blink
pub const SGR_RESET_BLINK: u16 = 25;

/// Reset Reverse
pub const SGR_RESET_REVERSE: u16 = 27;

/// Reset Hidden
pub const SGR_RESET_HIDDEN: u16 = 28;

/// Reset Strikethrough
pub const SGR_RESET_STRIKETHROUGH: u16 = 29;

// Foreground Colors (30-37, 90-97).

/// Black foreground
pub const SGR_FG_BLACK: u16 = 30;

/// Red foreground
pub const SGR_FG_RED: u16 = 31;

/// Green foreground
pub const SGR_FG_GREEN: u16 = 32;

/// Yellow foreground
pub const SGR_FG_YELLOW: u16 = 33;

/// Blue foreground
pub const SGR_FG_BLUE: u16 = 34;

/// Magenta foreground
pub const SGR_FG_MAGENTA: u16 = 35;

/// Cyan foreground
pub const SGR_FG_CYAN: u16 = 36;

/// White/Gray foreground
pub const SGR_FG_WHITE: u16 = 37;

/// Default foreground
pub const SGR_FG_DEFAULT: u16 = 39;

/// Bright Black foreground
pub const SGR_FG_BRIGHT_BLACK: u16 = 90;

/// Bright Red foreground
pub const SGR_FG_BRIGHT_RED: u16 = 91;

/// Bright Green foreground
pub const SGR_FG_BRIGHT_GREEN: u16 = 92;

/// Bright Yellow foreground
pub const SGR_FG_BRIGHT_YELLOW: u16 = 93;

/// Bright Blue foreground
pub const SGR_FG_BRIGHT_BLUE: u16 = 94;

/// Bright Magenta foreground
pub const SGR_FG_BRIGHT_MAGENTA: u16 = 95;

/// Bright Cyan foreground
pub const SGR_FG_BRIGHT_CYAN: u16 = 96;

/// Bright White foreground
pub const SGR_FG_BRIGHT_WHITE: u16 = 97;

// Background Colors (40-47, 100-107).

/// Black background
pub const SGR_BG_BLACK: u16 = 40;

/// Red background
pub const SGR_BG_RED: u16 = 41;

/// Green background
pub const SGR_BG_GREEN: u16 = 42;

/// Yellow background
pub const SGR_BG_YELLOW: u16 = 43;

/// Blue background
pub const SGR_BG_BLUE: u16 = 44;

/// Magenta background
pub const SGR_BG_MAGENTA: u16 = 45;

/// Cyan background
pub const SGR_BG_CYAN: u16 = 46;

/// White/Gray background
pub const SGR_BG_WHITE: u16 = 47;

/// Default background
pub const SGR_BG_DEFAULT: u16 = 49;

/// Bright Black background
pub const SGR_BG_BRIGHT_BLACK: u16 = 100;

/// Bright Red background
pub const SGR_BG_BRIGHT_RED: u16 = 101;

/// Bright Green background
pub const SGR_BG_BRIGHT_GREEN: u16 = 102;

/// Bright Yellow background
pub const SGR_BG_BRIGHT_YELLOW: u16 = 103;

/// Bright Blue background
pub const SGR_BG_BRIGHT_BLUE: u16 = 104;

/// Bright Magenta background
pub const SGR_BG_BRIGHT_MAGENTA: u16 = 105;

/// Bright Cyan background
pub const SGR_BG_BRIGHT_CYAN: u16 = 106;

/// Bright White background
pub const SGR_BG_BRIGHT_WHITE: u16 = 107;

// Cursor Save/Restore (CSI versions).

/// CSI s: Save Cursor Position (SCP)
/// Alternative to ESC 7
pub const SCP_SAVE_CURSOR: char = 's';

/// CSI u: Restore Cursor Position (RCP)
/// Alternative to ESC 8
pub const RCP_RESTORE_CURSOR: char = 'u';

// Device Status.

/// CSI n: Device Status Report (DSR)
/// 5 = request status
/// 6 = request cursor position
pub const DSR_DEVICE_STATUS: char = 'n';

// Mode Setting.

/// CSI h: Set Mode (SM)
/// Sets various terminal modes
pub const SM_SET_MODE: char = 'h';

/// CSI l: Reset Mode (RM)
/// Resets various terminal modes
pub const RM_RESET_MODE: char = 'l';

// Private Mode Setting (with ? prefix).

/// CSI ? h: Set Private Mode
/// Sets DEC private modes
pub const SM_SET_PRIVATE_MODE: char = 'h';

/// CSI ? l: Reset Private Mode
/// Resets DEC private modes
pub const RM_RESET_PRIVATE_MODE: char = 'l';

// Common Private Mode Numbers.

/// Cursor visibility (DECTCEM)
pub const DECCKM_CURSOR_KEYS: u16 = 1;

/// Application cursor keys
pub const DECANM_VT52_MODE: u16 = 2;

/// 132 column mode
pub const DECCOLM_132_COLUMN: u16 = 3;

/// Smooth scroll
pub const DECSCLM_SMOOTH_SCROLL: u16 = 4;

/// Reverse video
pub const DECSCNM_REVERSE_VIDEO: u16 = 5;

/// Origin mode
pub const DECOM_ORIGIN_MODE: u16 = 6;

/// Auto wrap
pub const DECAWM_AUTO_WRAP: u16 = 7;

/// Show cursor
pub const DECTCEM_SHOW_CURSOR: u16 = 25;

/// Save cursor
pub const SAVE_CURSOR_DEC: u16 = 1048;

/// Alternate screen buffer
pub const ALT_SCREEN_BUFFER: u16 = 1049;

/// DEC Private Mode types for CSI ? h/l sequences
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PrivateModeType {
    /// DECCKM - Application Cursor Keys (1)
    CursorKeys,
    /// DECANM - VT52 Mode (2)
    Vt52Mode,
    /// DECCOLM - 132 Column Mode (3)
    Column132,
    /// DECSCLM - Smooth Scroll (4)
    SmoothScroll,
    /// DECSCNM - Reverse Video (5)
    ReverseVideo,
    /// DECOM - Origin Mode (6)
    OriginMode,
    /// DECAWM - Auto Wrap Mode (7)
    AutoWrap,
    /// DECTCEM - Show/Hide Cursor (25)
    ShowCursor,
    /// Save Cursor (1048)
    SaveCursorDec,
    /// Use Alternate Screen Buffer (1049)
    AlternateScreenBuffer,
    /// Unknown/unsupported private mode
    Other(u16),
}

impl PrivateModeType {
    #[must_use]
    pub fn as_u16(&self) -> u16 {
        match self {
            Self::CursorKeys => DECCKM_CURSOR_KEYS,
            Self::Vt52Mode => DECANM_VT52_MODE,
            Self::Column132 => DECCOLM_132_COLUMN,
            Self::SmoothScroll => DECSCLM_SMOOTH_SCROLL,
            Self::ReverseVideo => DECSCNM_REVERSE_VIDEO,
            Self::OriginMode => DECOM_ORIGIN_MODE,
            Self::AutoWrap => DECAWM_AUTO_WRAP,
            Self::ShowCursor => DECTCEM_SHOW_CURSOR,
            Self::SaveCursorDec => SAVE_CURSOR_DEC,
            Self::AlternateScreenBuffer => ALT_SCREEN_BUFFER,
            Self::Other(n) => *n,
        }
    }
}

impl From<u16> for PrivateModeType {
    fn from(value: u16) -> Self {
        match value {
            DECCKM_CURSOR_KEYS => Self::CursorKeys,
            DECANM_VT52_MODE => Self::Vt52Mode,
            DECCOLM_132_COLUMN => Self::Column132,
            DECSCLM_SMOOTH_SCROLL => Self::SmoothScroll,
            DECSCNM_REVERSE_VIDEO => Self::ReverseVideo,
            DECOM_ORIGIN_MODE => Self::OriginMode,
            DECAWM_AUTO_WRAP => Self::AutoWrap,
            DECTCEM_SHOW_CURSOR => Self::ShowCursor,
            SAVE_CURSOR_DEC => Self::SaveCursorDec,
            ALT_SCREEN_BUFFER => Self::AlternateScreenBuffer,
            n => Self::Other(n),
        }
    }
}

impl From<&vte::Params> for PrivateModeType {
    fn from(params: &vte::Params) -> Self {
        let mode_num = params.extract_nth_opt(0).unwrap_or(0);
        mode_num.into()
    }
}

/// Margin request types for DECSTBM (Set Top and Bottom Margins) operations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MarginRequest {
    /// Reset margins to full screen (ESC[r, ESC[0r, ESC[0;0r)
    Reset,
    /// Set specific scrolling region margins
    SetRegion { top: TermRow, bottom: TermRow },
}

impl From<(Option<u16>, Option<u16>)> for MarginRequest {
    fn from((maybe_top, maybe_bottom): (Option<u16>, Option<u16>)) -> Self {
        // VT100 spec: missing params or zero params mean reset to full screen
        match (maybe_top, maybe_bottom) {
            (None | Some(0), None) | (Some(0), Some(0)) => Self::Reset,
            _ => {
                // Convert to 1-based terminal coordinates (VT100 spec uses 1-based)
                let top_row = maybe_top.map_or(1, |v| max(v, 1));
                let bottom_row = maybe_bottom.unwrap_or(24); // Default bottom
                Self::SetRegion {
                    top: term_row(top_row),
                    bottom: term_row(bottom_row),
                }
            }
        }
    }
}

impl From<&vte::Params> for MarginRequest {
    fn from(params: &vte::Params) -> Self {
        let maybe_top = params.extract_nth_opt(0);
        let maybe_bottom = params.extract_nth_opt(1);
        (maybe_top, maybe_bottom).into()
    }
}

/// Movement count for cursor and scroll operations.
///
/// VT100 specification: missing parameters or zero parameters default to 1.
/// This type encapsulates that logic for all movement operations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MovementCount(pub u16);

impl MovementCount {
    /// Parse VT100 movement parameters as a generic [`Length`].
    ///
    /// VT100 specification: missing parameters or zero parameters default to 1.
    ///
    /// # Returns
    /// [`Length`] type for type-safe usage with the bounds checking system.
    #[must_use]
    pub fn parse_as_length(params: &vte::Params) -> Length {
        let count = params.extract_nth_non_zero(0);
        len(count)
    }

    /// Parse VT100 movement parameters as a [`RowHeight`] for vertical operations.
    ///
    /// VT100 specification: missing parameters or zero parameters default to 1.
    ///
    /// # Returns
    /// [`RowHeight`] type for type-safe usage with the bounds checking system.
    #[must_use]
    pub fn parse_as_row_height(params: &vte::Params) -> RowHeight {
        let count = params.extract_nth_non_zero(0);
        height(count)
    }

    /// Parse VT100 movement parameters as a `[ColWidth]` for horizontal operations.
    ///
    /// VT100 specification: missing parameters or zero parameters default to 1.
    ///
    /// # Returns
    /// [`ColWidth`] type for type-safe usage with the bounds checking system.
    #[must_use]
    pub fn parse_as_col_width(params: &vte::Params) -> ColWidth {
        let count = params.extract_nth_non_zero(0);
        width(count)
    }
}

impl From<&vte::Params> for MovementCount {
    fn from(params: &vte::Params) -> Self {
        // ParamsExt::extract_nth_non_zero() guarantees count >= 1
        // per VT100 spec: missing or zero parameters default to 1
        let count = params.extract_nth_non_zero(0);
        Self(count)
    }
}

/// Absolute position for cursor positioning operations.
///
/// VT100 specification: position parameters are 1-based, with
/// missing/zero parameters defaulting to 1. This type encapsulates
/// position parameters for absolute cursor positioning commands like
/// CHA (Cursor Horizontal Absolute) and VPA (Vertical Position Absolute).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AbsolutePosition(pub u16);

impl AbsolutePosition {
    /// Parse VT100 position parameter as a 0-based [`RowIndex`].
    ///
    /// VT100 specification: position parameters are 1-based, with missing/zero
    /// parameters defaulting to 1. This method handles the 1-based to 0-based
    /// conversion internally.
    ///
    /// # Returns
    /// [`RowIndex`] with 0-based position ready for use in buffer operations.
    #[must_use]
    pub fn parse_as_row_index(params: &vte::Params) -> RowIndex {
        let position = params.extract_nth_non_zero(0); // Gets 1-based position
        row(position.saturating_sub(1)) // Convert to 0-based
    }

    /// Parse VT100 position parameter as a 0-based [`ColIndex`].
    ///
    /// VT100 specification: position parameters are 1-based, with missing/zero
    /// parameters defaulting to 1. This method handles the 1-based to 0-based
    /// conversion internally.
    ///
    /// # Returns
    /// [`ColIndex`] with 0-based position ready for use in buffer operations.
    #[must_use]
    pub fn parse_as_col_index(params: &vte::Params) -> ColIndex {
        let position = params.extract_nth_non_zero(0); // Gets 1-based position
        col(position.saturating_sub(1)) // Convert to 0-based
    }
}

/// Cursor position request for CUP (Cursor Position) operations.
///
/// VT100 specification: coordinates are 1-based, but internally converted to 0-based.
/// Missing or zero parameters default to 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CursorPositionRequest {
    /// Row position (0-based, converted from 1-based VT100)
    pub row: u16,
    /// Column position (0-based, converted from 1-based VT100)
    pub col: u16,
}

impl From<(u16, u16)> for CursorPositionRequest {
    fn from((row_param, col_param): (u16, u16)) -> Self {
        // Convert from 1-based VT100 coordinates to 0-based internal coordinates
        Self {
            row: row_param.saturating_sub(1),
            col: col_param.saturating_sub(1),
        }
    }
}

impl From<&vte::Params> for CursorPositionRequest {
    fn from(params: &vte::Params) -> Self {
        let row_param = params.extract_nth_non_zero(0);
        let col_param = params.extract_nth_non_zero(1);
        (row_param, col_param).into()
    }
}

/// Test helper functions for CSI sequences.
#[cfg(test)]
pub mod csi_test_helpers {
    use super::CsiSequence;

    /// Helper function to create a `CsiSequence::CursorPosition`.
    ///
    /// # Panics
    /// Panics if the provided position is not a `CsiSequence::CursorPosition`.
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::ansi_parser::csi_codes::csi_test_helpers::csi_seq_cursor_pos;
    /// use r3bl_tui::ansi_parser::term_units::{term_row, term_col};
    ///
    /// // Instead of:
    /// // CsiSequence::CursorPosition { row: term_row(2), col: term_col(3) }
    ///
    /// // You can write:
    /// let seq = csi_seq_cursor_pos(term_row(2) + term_col(3));
    /// ```
    #[must_use]
    pub fn csi_seq_cursor_pos(position: CsiSequence) -> CsiSequence {
        match position {
            CsiSequence::CursorPosition { .. } => position,
            _ => panic!("Expected CsiSequence::CursorPosition"),
        }
    }

    /// Helper function to create a `CsiSequence::CursorPositionAlt`.
    ///
    /// # Panics
    /// Panics if the provided position is not a `CsiSequence::CursorPosition` or
    /// `CursorPositionAlt`.
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::ansi_parser::csi_codes::csi_test_helpers::csi_seq_cursor_pos_alt;
    /// use r3bl_tui::ansi_parser::term_units::{term_row, term_col};
    ///
    /// // Instead of:
    /// // CsiSequence::CursorPositionAlt { row: term_row(3), col: term_col(7) }
    ///
    /// // You can write:
    /// let seq = csi_seq_cursor_pos_alt(term_row(3) + term_col(7));
    /// ```
    #[must_use]
    pub fn csi_seq_cursor_pos_alt(position: CsiSequence) -> CsiSequence {
        match position {
            CsiSequence::CursorPosition { row, col } => {
                CsiSequence::CursorPositionAlt { row, col }
            }
            CsiSequence::CursorPositionAlt { .. } => position,
            _ => panic!("Expected CsiSequence::CursorPosition or CursorPositionAlt"),
        }
    }
}

// CSI sequence builder following the same pattern as SgrCode.

/// Builder for CSI (Control Sequence Introducer) sequences.
/// Similar to `SgrCode` but for cursor movement and other CSI commands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CsiSequence {
    /// Cursor Up (CUU) - ESC [ n A
    CursorUp(u16),
    /// Cursor Down (CUD) - ESC [ n B
    CursorDown(u16),
    /// Cursor Forward (CUF) - ESC [ n C
    CursorForward(u16),
    /// Cursor Backward (CUB) - ESC [ n D
    CursorBackward(u16),
    /// Cursor Position (CUP) - ESC [ row ; col H
    CursorPosition { row: TermRow, col: TermCol },
    /// Cursor Position alternate form (HVP) - ESC [ row ; col f
    CursorPositionAlt { row: TermRow, col: TermCol },
    /// Erase Display (ED) - ESC [ n J
    EraseDisplay(u16),
    /// Erase Line (EL) - ESC [ n K
    EraseLine(u16),
    /// Save Cursor (SCP) - ESC [ s
    SaveCursor,
    /// Restore Cursor (RCP) - ESC [ u
    RestoreCursor,
    /// Cursor Next Line (CNL) - ESC [ n E
    CursorNextLine(u16),
    /// Cursor Previous Line (CPL) - ESC [ n F
    CursorPrevLine(u16),
    /// Cursor Horizontal Absolute (CHA) - ESC [ n G
    CursorHorizontalAbsolute(u16),
    /// Scroll Up (SU) - ESC [ n S
    ScrollUp(u16),
    /// Scroll Down (SD) - ESC [ n T
    ScrollDown(u16),
    /// Set Top and Bottom Margins (DECSTBM) - ESC [ top ; bottom r
    SetScrollingMargins {
        top: Option<TermRow>,
        bottom: Option<TermRow>,
    },
    /// Device Status Report (DSR) - ESC [ n n
    DeviceStatusReport(DsrRequestType),
    /// Enable Private Mode - ESC [ ? n h (n = mode number like `DECAWM_AUTO_WRAP`)
    /// See [`crate::offscreen_buffer::AnsiParserSupport::auto_wrap_mode`]
    EnablePrivateMode(PrivateModeType),
    /// Disable Private Mode - ESC [ ? n l (n = mode number like `DECAWM_AUTO_WRAP`)
    /// See [`crate::offscreen_buffer::AnsiParserSupport::auto_wrap_mode`]
    DisablePrivateMode(PrivateModeType),
    /// Insert Line (IL) - ESC [ n L
    InsertLine(u16),
    /// Delete Line (DL) - ESC [ n M
    DeleteLine(u16),
    /// Delete Character (DCH) - ESC [ n P
    DeleteChar(u16),
    /// Insert Character (ICH) - ESC [ n @
    InsertChar(u16),
    /// Erase Character (ECH) - ESC [ n X
    EraseChar(u16),
    /// Vertical Position Absolute (VPA) - ESC [ n d
    VerticalPositionAbsolute(u16),
}

impl Display for CsiSequence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut acc = BufTextStorage::new();
        self.write_to_buf(&mut acc)?;
        self.write_buf_to_fmt(&acc, f)
    }
}

impl WriteToBuf for CsiSequence {
    #[allow(clippy::too_many_lines)]
    fn write_to_buf(&self, acc: &mut BufTextStorage) -> fmt::Result {
        acc.push_str("\x1b[");
        match self {
            CsiSequence::CursorUp(n) => {
                acc.push_str(&n.to_string());
                acc.push(CUU_CURSOR_UP);
            }
            CsiSequence::CursorDown(n) => {
                acc.push_str(&n.to_string());
                acc.push(CUD_CURSOR_DOWN);
            }
            CsiSequence::CursorForward(n) => {
                acc.push_str(&n.to_string());
                acc.push(CUF_CURSOR_FORWARD);
            }
            CsiSequence::CursorBackward(n) => {
                acc.push_str(&n.to_string());
                acc.push(CUB_CURSOR_BACKWARD);
            }
            CsiSequence::CursorPosition { row, col } => {
                acc.push_str(&row.as_u16().to_string());
                acc.push(CSI_PARAM_SEPARATOR);
                acc.push_str(&col.as_u16().to_string());
                acc.push(CUP_CURSOR_POSITION);
            }
            CsiSequence::CursorPositionAlt { row, col } => {
                acc.push_str(&row.as_u16().to_string());
                acc.push(CSI_PARAM_SEPARATOR);
                acc.push_str(&col.as_u16().to_string());
                acc.push(HVP_CURSOR_POSITION);
            }
            CsiSequence::EraseDisplay(n) => {
                acc.push_str(&n.to_string());
                acc.push(ED_ERASE_DISPLAY);
            }
            CsiSequence::EraseLine(n) => {
                acc.push_str(&n.to_string());
                acc.push(EL_ERASE_LINE);
            }
            CsiSequence::SaveCursor => {
                acc.push(SCP_SAVE_CURSOR);
            }
            CsiSequence::RestoreCursor => {
                acc.push(RCP_RESTORE_CURSOR);
            }
            CsiSequence::CursorNextLine(n) => {
                acc.push_str(&n.to_string());
                acc.push(CNL_CURSOR_NEXT_LINE);
            }
            CsiSequence::CursorPrevLine(n) => {
                acc.push_str(&n.to_string());
                acc.push(CPL_CURSOR_PREV_LINE);
            }
            CsiSequence::CursorHorizontalAbsolute(n) => {
                acc.push_str(&n.to_string());
                acc.push(CHA_CURSOR_COLUMN);
            }
            CsiSequence::ScrollUp(n) => {
                acc.push_str(&n.to_string());
                acc.push(SU_SCROLL_UP);
            }
            CsiSequence::ScrollDown(n) => {
                acc.push_str(&n.to_string());
                acc.push(SD_SCROLL_DOWN);
            }
            CsiSequence::SetScrollingMargins { top, bottom } => {
                if let Some(top_row) = top {
                    acc.push_str(&top_row.as_u16().to_string());
                }
                acc.push(CSI_PARAM_SEPARATOR);
                if let Some(bottom_row) = bottom {
                    acc.push_str(&bottom_row.as_u16().to_string());
                }
                acc.push(DECSTBM_SET_MARGINS);
            }
            CsiSequence::DeviceStatusReport(dsr_type) => {
                acc.push_str(&dsr_type.as_u16().to_string());
                acc.push(DSR_DEVICE_STATUS);
            }
            CsiSequence::EnablePrivateMode(mode) => {
                acc.push(CSI_PRIVATE_MODE_PREFIX);
                acc.push_str(&mode.as_u16().to_string());
                acc.push(SM_SET_PRIVATE_MODE);
            }
            CsiSequence::DisablePrivateMode(mode) => {
                acc.push(CSI_PRIVATE_MODE_PREFIX);
                acc.push_str(&mode.as_u16().to_string());
                acc.push(RM_RESET_PRIVATE_MODE);
            }
            CsiSequence::InsertLine(n) => {
                acc.push_str(&n.to_string());
                acc.push(IL_INSERT_LINE);
            }
            CsiSequence::DeleteLine(n) => {
                acc.push_str(&n.to_string());
                acc.push(DL_DELETE_LINE);
            }
            CsiSequence::DeleteChar(n) => {
                acc.push_str(&n.to_string());
                acc.push(DCH_DELETE_CHAR);
            }
            CsiSequence::InsertChar(n) => {
                acc.push_str(&n.to_string());
                acc.push(ICH_INSERT_CHAR);
            }
            CsiSequence::EraseChar(n) => {
                acc.push_str(&n.to_string());
                acc.push(ECH_ERASE_CHAR);
            }
            CsiSequence::VerticalPositionAbsolute(n) => {
                acc.push_str(&n.to_string());
                acc.push(VPA_VERTICAL_POSITION);
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    // Integration test helper - process CSI sequence and extract params
    fn process_csi_sequence_and_test<F>(sequence: &str, test_fn: F)
    where
        F: Fn(&vte::Params),
    {
        use vte::{Parser, Perform};

        struct TestPerformer<F> {
            test_fn: Option<F>,
        }

        impl<F> TestPerformer<F>
        where
            F: Fn(&vte::Params),
        {
            fn new(test_fn: F) -> Self {
                Self {
                    test_fn: Some(test_fn),
                }
            }
        }

        impl<F> Perform for TestPerformer<F>
        where
            F: Fn(&vte::Params),
        {
            fn csi_dispatch(
                &mut self,
                params: &vte::Params,
                _intermediates: &[u8],
                _ignore: bool,
                _c: char,
            ) {
                if let Some(test_fn) = self.test_fn.take() {
                    test_fn(params);
                }
            }

            // Required by Perform trait but unused
            fn print(&mut self, _c: char) {}
            fn execute(&mut self, _byte: u8) {}
            fn hook(
                &mut self,
                _params: &vte::Params,
                _intermediates: &[u8],
                _ignore: bool,
                _c: char,
            ) {
            }
            fn put(&mut self, _byte: u8) {}
            fn unhook(&mut self) {}
            fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}
            fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}
        }

        let mut parser = Parser::new();
        let mut performer = TestPerformer::new(test_fn);

        for byte in sequence.bytes() {
            parser.advance(&mut performer, byte);
        }
    }

    mod movement_count_tests {
        use super::*;

        #[test]
        fn test_parse_as_length_with_valid_value() {
            process_csi_sequence_and_test("\x1b[5A", |params| {
                let result = MovementCount::parse_as_length(params);
                assert_eq!(result.as_usize(), 5);
            });
        }

        #[test]
        fn test_parse_as_length_with_missing_params() {
            process_csi_sequence_and_test("\x1b[A", |params| {
                let result = MovementCount::parse_as_length(params);
                assert_eq!(result.as_usize(), 1); // Should default to 1
            });
        }

        #[test]
        fn test_parse_as_length_with_zero_param() {
            process_csi_sequence_and_test("\x1b[0A", |params| {
                let result = MovementCount::parse_as_length(params);
                assert_eq!(result.as_usize(), 1); // Zero should become 1
            });
        }

        #[test]
        fn test_parse_as_row_height_with_valid_value() {
            process_csi_sequence_and_test("\x1b[10A", |params| {
                let result = MovementCount::parse_as_row_height(params);
                assert_eq!(result.as_u16(), 10);
            });
        }

        #[test]
        fn test_parse_as_row_height_with_missing_params() {
            process_csi_sequence_and_test("\x1b[A", |params| {
                let result = MovementCount::parse_as_row_height(params);
                assert_eq!(result.as_u16(), 1); // Should default to 1
            });
        }

        #[test]
        fn test_parse_as_col_width_with_valid_value() {
            process_csi_sequence_and_test("\x1b[25C", |params| {
                let result = MovementCount::parse_as_col_width(params);
                assert_eq!(result.as_u16(), 25);
            });
        }

        #[test]
        fn test_parse_as_col_width_with_missing_params() {
            process_csi_sequence_and_test("\x1b[C", |params| {
                let result = MovementCount::parse_as_col_width(params);
                assert_eq!(result.as_u16(), 1); // Should default to 1
            });
        }

        #[test]
        fn test_from_params_trait() {
            process_csi_sequence_and_test("\x1b[42A", |params| {
                let movement_count = MovementCount::from(params);
                assert_eq!(movement_count.0, 42);
            });
        }

        #[test]
        fn test_from_params_trait_with_empty() {
            process_csi_sequence_and_test("\x1b[A", |params| {
                let movement_count = MovementCount::from(params);
                assert_eq!(movement_count.0, 1); // Should default to 1
            });
        }

        #[test]
        fn test_from_params_trait_with_zero() {
            process_csi_sequence_and_test("\x1b[0A", |params| {
                let movement_count = MovementCount::from(params);
                assert_eq!(movement_count.0, 1); // Zero should become 1
            });
        }
    }

    mod absolute_position_tests {
        use super::*;

        #[test]
        fn test_parse_as_row_index_with_valid_value() {
            process_csi_sequence_and_test("\x1b[5d", |params| {
                // VPA command
                let result = AbsolutePosition::parse_as_row_index(params);
                assert_eq!(result.as_u16(), 4); // Should be 0-based (5-1=4)
            });
        }

        #[test]
        fn test_parse_as_row_index_with_missing_params() {
            process_csi_sequence_and_test("\x1b[d", |params| {
                let result = AbsolutePosition::parse_as_row_index(params);
                assert_eq!(result.as_u16(), 0); // Missing param defaults to 1, then 1-1=0
            });
        }

        #[test]
        fn test_parse_as_row_index_with_zero() {
            process_csi_sequence_and_test("\x1b[0d", |params| {
                let result = AbsolutePosition::parse_as_row_index(params);
                assert_eq!(result.as_u16(), 0); // Zero becomes 1, then 1-1=0
            });
        }

        #[test]
        fn test_parse_as_row_index_with_one() {
            process_csi_sequence_and_test("\x1b[1d", |params| {
                let result = AbsolutePosition::parse_as_row_index(params);
                assert_eq!(result.as_u16(), 0); // Should be 0-based (1-1=0)
            });
        }

        #[test]
        fn test_parse_as_col_index_with_valid_value() {
            process_csi_sequence_and_test("\x1b[10G", |params| {
                // CHA command
                let result = AbsolutePosition::parse_as_col_index(params);
                assert_eq!(result.as_u16(), 9); // Should be 0-based (10-1=9)
            });
        }

        #[test]
        fn test_parse_as_col_index_with_missing_params() {
            process_csi_sequence_and_test("\x1b[G", |params| {
                let result = AbsolutePosition::parse_as_col_index(params);
                assert_eq!(result.as_u16(), 0); // Missing param defaults to 1, then 1-1=0
            });
        }

        #[test]
        fn test_parse_as_col_index_with_zero() {
            process_csi_sequence_and_test("\x1b[0G", |params| {
                let result = AbsolutePosition::parse_as_col_index(params);
                assert_eq!(result.as_u16(), 0); // Zero becomes 1, then 1-1=0
            });
        }

        #[test]
        fn test_parse_as_col_index_large_value() {
            process_csi_sequence_and_test("\x1b[100G", |params| {
                let result = AbsolutePosition::parse_as_col_index(params);
                assert_eq!(result.as_u16(), 99); // Should be 0-based (100-1=99)
            });
        }
    }

    mod cursor_position_request_tests {
        use super::*;

        #[test]
        fn test_from_params_with_both_values() {
            process_csi_sequence_and_test("\x1b[5;10H", |params| {
                // CUP command
                let result = CursorPositionRequest::from(params);
                assert_eq!(result.row, 4); // Should be 0-based (5-1=4)
                assert_eq!(result.col, 9); // Should be 0-based (10-1=9)
            });
        }

        #[test]
        fn test_from_params_with_only_row() {
            process_csi_sequence_and_test("\x1b[3H", |params| {
                let result = CursorPositionRequest::from(params);
                assert_eq!(result.row, 2); // Should be 0-based (3-1=2)
                assert_eq!(result.col, 0); // Missing col defaults to 1, then 1-1=0
            });
        }

        #[test]
        fn test_from_params_with_empty() {
            process_csi_sequence_and_test("\x1b[H", |params| {
                let result = CursorPositionRequest::from(params);
                assert_eq!(result.row, 0); // Missing row defaults to 1, then 1-1=0
                assert_eq!(result.col, 0); // Missing col defaults to 1, then 1-1=0
            });
        }

        #[test]
        fn test_from_params_with_zeros() {
            process_csi_sequence_and_test("\x1b[0;0H", |params| {
                let result = CursorPositionRequest::from(params);
                assert_eq!(result.row, 0); // Zero becomes 1, then 1-1=0
                assert_eq!(result.col, 0); // Zero becomes 1, then 1-1=0
            });
        }

        #[test]
        fn test_from_params_with_column_only() {
            process_csi_sequence_and_test("\x1b[;5H", |params| {
                // Empty row, col=5
                let result = CursorPositionRequest::from(params);
                assert_eq!(result.row, 0); // Missing row defaults to 1, then 1-1=0
                assert_eq!(result.col, 4); // Should be 0-based (5-1=4)
            });
        }

        #[test]
        fn test_from_tuple() {
            let result = CursorPositionRequest::from((5, 10)); // Already 0-based from tuple
            assert_eq!(result.row, 4); // Tuple input is 1-based, so 5-1=4
            assert_eq!(result.col, 9); // Tuple input is 1-based, so 10-1=9
        }

        #[test]
        fn test_cursor_position_request_equality() {
            let request1 = CursorPositionRequest { row: 5, col: 10 };
            let request2 = CursorPositionRequest { row: 5, col: 10 };
            let request3 = CursorPositionRequest { row: 5, col: 11 };

            assert_eq!(request1, request2);
            assert_ne!(request1, request3);
        }

        #[test]
        fn test_cursor_position_request_debug() {
            let request = CursorPositionRequest { row: 5, col: 10 };
            let debug_output = format!("{request:?}");
            assert!(debug_output.contains("CursorPositionRequest"));
            assert!(debug_output.contains("row: 5"));
            assert!(debug_output.contains("col: 10"));
        }
    }
}
