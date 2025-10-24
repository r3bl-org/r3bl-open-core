// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! This module contains all the constant values used in CSI (Control Sequence Introducer)
//! sequences, organized by functional category.

// CSI sequence components.

/// CSI sequence start: ESC [
pub const CSI_START: &str = "\x1b[";

/// Private mode prefix for CSI sequences
pub const CSI_PRIVATE_MODE_PREFIX: char = '?';

/// Parameter separator in CSI sequences (semicolon)
///
/// Used to separate top-level parameters in CSI sequences:
/// - `ESC[1;5H` - Cursor position (row 1, column 5)
/// - `ESC[1;31m` - Bold + red foreground
pub const CSI_PARAM_SEPARATOR: char = ';';

/// Sub-parameter separator in CSI sequences (colon)
///
/// Used to separate sub-parameters within a single CSI parameter:
/// - `ESC[38:5:196m` - 256-color foreground (38 = fg extended, 5 = palette mode, 196 =
///   index)
/// - `ESC[48:2:255:128:0m` - RGB background (48 = bg extended, 2 = RGB mode, 255:128:0 =
///   RGB)
///
/// Per ITU-T Rec. T.416 (ISO 8613-6), the colon (`:`) is the recommended modern format
/// for sub-parameters, while semicolon (`;`) is supported for legacy compatibility.
pub const CSI_SUB_PARAM_SEPARATOR: char = ':';

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

// Erase Display Parameters (ED).

/// Erase from cursor to end of screen (default for ED)
pub const ED_ERASE_TO_END: u16 = 0;

/// Erase from start of screen to cursor
pub const ED_ERASE_FROM_START: u16 = 1;

/// Erase entire screen
pub const ED_ERASE_ALL: u16 = 2;

/// Erase entire screen and scrollback
pub const ED_ERASE_ALL_AND_SCROLLBACK: u16 = 3;

// Erase Line Parameters (EL).

/// Erase from cursor to end of line (default for EL)
pub const EL_ERASE_TO_END: u16 = 0;

/// Erase from start of line to cursor
pub const EL_ERASE_FROM_START: u16 = 1;

/// Erase entire line
pub const EL_ERASE_ALL: u16 = 2;

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

// Extended Color Support (256-color and RGB).

/// Extended foreground color (SGR 38)
///
/// Used in sequences like:
/// - `ESC[38:5:nM` - 256-color foreground (n = 0-255)
/// - `ESC[38:2:r:g:bM` - RGB foreground (r,g,b = 0-255)
pub const SGR_FG_EXTENDED: u16 = 38;

/// Extended background color (SGR 48)
///
/// Used in sequences like:
/// - `ESC[48:5:nM` - 256-color background (n = 0-255)
/// - `ESC[48:2:r:g:bM` - RGB background (r,g,b = 0-255)
pub const SGR_BG_EXTENDED: u16 = 48;

/// 256-color mode indicator
///
/// Second parameter in 256-color sequences:
/// - `ESC[38:5:nM` - 256-color foreground
/// - `ESC[48:5:nM` - 256-color background
pub const SGR_COLOR_MODE_256: u16 = 5;

/// RGB color mode indicator
///
/// Second parameter in RGB color sequences:
/// - `ESC[38:2:r:g:bM` - RGB foreground
/// - `ESC[48:2:r:g:bM` - RGB background
pub const SGR_COLOR_MODE_RGB: u16 = 2;

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

/// Show cursor (DECTCEM)
pub const DECTCEM_SHOW_CURSOR: u16 = 25;

// NOTE: General terminal mode constants (DEC modes 1-7, alternate screen buffer,
// mouse tracking, bracketed paste, etc.) are defined in the parent `protocols`
// module constants file (`protocols/generic_ansi_constants.rs`) rather than here.
//
// This separation reflects the architectural distinction:
// - **This file (`csi_codes/csi_constants.rs`)**: CSI-specific sequencing details
// - **Parent file (`protocols/generic_ansi_constants.rs`)**: General ANSI/terminal feature constants
