// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Constant values used in [`CSI`] (Control Sequence Introducer) sequences, organized by
//! functional category.
//!
//! These constants are used by [`CsiSequence`] for building [`CSI`] output sequences, and
//! by [`SgrCode`] for text formatting parameters.
//!
//! See [constants module design] for the three-tier architecture.
//!
//! [`CSI`]: crate::CsiSequence
//! [`CsiSequence`]: crate::CsiSequence
//! [`SgrCode`]: crate::SgrCode
//! [constants module design]: mod@crate::constants#design

use crate::define_ansi_const;

// CSI sequence components.

define_ansi_const!(@csi_str : CSI_START = [""] => "CSI Start" : "Sequence start: `ESC [`");

/// Private Mode Prefix ([`CSI`]): Introduces [`DEC`] private mode parameters in [`CSI`]
/// sequences.
///
/// Value: `'?'` dec, `3F` hex.
///
/// Sequence: `CSI ?`.
///
/// [`CSI`]: crate::CsiSequence
/// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
pub const CSI_PRIVATE_MODE_PREFIX: char = '?';

/// Parameter Separator ([`CSI`]): Separates top-level parameters in [`CSI`] sequences.
///
/// Value: `';'` dec, `3B` hex.
///
/// Sequence: `CSI n ; m`.
///
/// Used to separate top-level parameters in [`CSI`] sequences:
/// - `ESC [ 1 ; 5 H` - Cursor position (row 1, column 5)
/// - `ESC [ 1 ; 31 m` - Bold + red foreground
///
/// [`CSI`]: crate::CsiSequence
pub const CSI_PARAM_SEPARATOR: char = ';';

/// Sub-Parameter Separator ([`CSI`]): Separates sub-parameters within a single [`CSI`]
/// parameter.
///
/// Value: `':'` dec, `3A` hex.
///
/// Sequence: `CSI n : m`.
///
/// Used to separate sub-parameters within a single [`CSI`] parameter:
/// - `ESC [ 38 : 5 : 196 m` - 256-color foreground (38 = fg extended, 5 = palette mode,
///   196 = index)
/// - `ESC [ 48 : 2 : 255 : 128 : 0 m` - RGB background (48 = bg extended, 2 = RGB mode,
///   255:128:0 = RGB)
///
/// Per [ITU-T Rec. T.416] (ISO 8613-6), the colon (`:`) is the recommended modern format
/// for sub-parameters, while semicolon (`;`) is supported for legacy compatibility.
///
/// [`CSI`]: crate::CsiSequence
/// [ITU-T Rec. T.416]: https://www.itu.int/rec/T-REC-T.416-199303-I
pub const CSI_SUB_PARAM_SEPARATOR: char = ':';

// Cursor Movement.

/// Cursor Up (CUU): Moves cursor up by n lines (default 1).
///
/// Value: `'A'` dec, `41` hex.
///
/// Sequence: `CSI n A`.
///
/// [`CSI`]: crate::CsiSequence
pub const CUU_CURSOR_UP: char = 'A';

/// Cursor Down (CUD): Moves cursor down by n lines (default 1).
///
/// Value: `'B'` dec, `42` hex.
///
/// Sequence: `CSI n B`.
///
/// [`CSI`]: crate::CsiSequence
pub const CUD_CURSOR_DOWN: char = 'B';

/// Cursor Forward (CUF): Moves cursor forward by n columns (default 1).
///
/// Value: `'C'` dec, `43` hex.
///
/// Sequence: `CSI n C`.
///
/// [`CSI`]: crate::CsiSequence
pub const CUF_CURSOR_FORWARD: char = 'C';

/// Cursor Backward (CUB): Moves cursor backward by n columns (default 1).
///
/// Value: `'D'` dec, `44` hex.
///
/// Sequence: `CSI n D`.
///
/// [`CSI`]: crate::CsiSequence
pub const CUB_CURSOR_BACKWARD: char = 'D';

/// Cursor Next Line (CNL): Moves cursor to beginning of line n lines down (default 1).
///
/// Value: `'E'` dec, `45` hex.
///
/// Sequence: `CSI n E`.
///
/// [`CSI`]: crate::CsiSequence
pub const CNL_CURSOR_NEXT_LINE: char = 'E';

/// Cursor Previous Line (CPL): Moves cursor to beginning of line n lines up (default 1).
///
/// Value: `'F'` dec, `46` hex.
///
/// Sequence: `CSI n F`.
///
/// [`CSI`]: crate::CsiSequence
pub const CPL_CURSOR_PREV_LINE: char = 'F';

/// Cursor Horizontal Absolute (CHA): Moves cursor to column n (default 1).
///
/// Value: `'G'` dec, `47` hex.
///
/// Sequence: `CSI n G`.
///
/// [`CSI`]: crate::CsiSequence
pub const CHA_CURSOR_COLUMN: char = 'G';

/// Cursor Position (CUP): Moves cursor to row n, column m (default 1,1).
///
/// Value: `'H'` dec, `48` hex.
///
/// Sequence: `CSI n ; m H`.
///
/// [`CSI`]: crate::CsiSequence
pub const CUP_CURSOR_POSITION: char = 'H';

/// Horizontal and Vertical Position (HVP): Moves cursor to row n, column m (default 1,1).
/// Same as CUP.
///
/// Value: `'f'` dec, `66` hex.
///
/// Sequence: `CSI n ; m f`.
///
/// [`CSI`]: crate::CsiSequence
pub const HVP_CURSOR_POSITION: char = 'f';

// Erasing.

/// Erase in Display (ED): Erases part or all of the screen.
/// `0` = erase from cursor to end of screen (default),
/// `1` = erase from start of screen to cursor,
/// `2` = erase entire screen,
/// `3` = erase entire screen and scrollback.
///
/// Value: `'J'` dec, `4A` hex.
///
/// Sequence: `CSI n J`.
///
/// [`CSI`]: crate::CsiSequence
pub const ED_ERASE_DISPLAY: char = 'J';

/// Erase in Line (EL): Erases part or all of the current line.
/// `0` = erase from cursor to end of line (default),
/// `1` = erase from start of line to cursor,
/// `2` = erase entire line.
///
/// Value: `'K'` dec, `4B` hex.
///
/// Sequence: `CSI n K`.
///
/// [`CSI`]: crate::CsiSequence
pub const EL_ERASE_LINE: char = 'K';

// Erase Display Parameters (ED).

/// Erase to End (ED `0`): Erase from cursor to end of screen (default for ED).
///
/// Value: `0`.
pub const ED_ERASE_TO_END: u16 = 0;

/// Erase from Start (ED `1`): Erase from start of screen to cursor.
///
/// Value: `1`.
pub const ED_ERASE_FROM_START: u16 = 1;

/// Erase All (ED `2`): Erase entire screen.
///
/// Value: `2`.
pub const ED_ERASE_ALL: u16 = 2;

/// Erase All and Scrollback (ED `3`): Erase entire screen and scrollback.
///
/// Value: `3`.
pub const ED_ERASE_ALL_AND_SCROLLBACK: u16 = 3;

// Erase Line Parameters (EL).

/// Erase to End (EL `0`): Erase from cursor to end of line (default for EL).
///
/// Value: `0`.
pub const EL_ERASE_TO_END: u16 = 0;

/// Erase from Start (EL `1`): Erase from start of line to cursor.
///
/// Value: `1`.
pub const EL_ERASE_FROM_START: u16 = 1;

/// Erase All (EL `2`): Erase entire line.
///
/// Value: `2`.
pub const EL_ERASE_ALL: u16 = 2;

// Scrolling.

/// Scroll Up (SU): Scrolls text up by n lines (default 1).
///
/// Value: `'S'` dec, `53` hex.
///
/// Sequence: `CSI n S`.
///
/// [`CSI`]: crate::CsiSequence
pub const SU_SCROLL_UP: char = 'S';

/// Scroll Down (SD): Scrolls text down by n lines (default 1).
///
/// Value: `'T'` dec, `54` hex.
///
/// Sequence: `CSI n T`.
///
/// [`CSI`]: crate::CsiSequence
pub const SD_SCROLL_DOWN: char = 'T';

/// Set Top and Bottom Margins ([`DECSTBM`]): Defines the scrolling region.
///
/// Value: `'r'` dec, `72` hex.
///
/// Sequence: `CSI top ; bottom r`.
///
/// [`CSI`]: crate::CsiSequence
/// [`DECSTBM`]: https://vt100.net/docs/vt510-rm/DECSTBM.html
pub const DECSTBM_SET_MARGINS: char = 'r';

// Line Operations.

/// Insert Line (IL): Inserts one or more blank lines, starting at the cursor.
/// Lines below cursor and in scrolling region move down.
///
/// Sequence: `CSI n L`
///
/// [`CSI`]: crate::CsiSequence
pub const IL_INSERT_LINE: char = 'L';

/// Delete Line (DL): Deletes one or more lines in the scrolling region, starting with
/// cursor line. Lines below cursor move up, blank lines added at bottom.
///
/// Sequence: `CSI n M`
///
/// [`CSI`]: crate::CsiSequence
pub const DL_DELETE_LINE: char = 'M';

// Character Operations.

/// Delete Character (DCH): Deletes one or more characters on current line.
/// Characters to the right shift left, blanks inserted at end.
///
/// Sequence: `CSI n P`
///
/// [`CSI`]: crate::CsiSequence
pub const DCH_DELETE_CHAR: char = 'P';

/// Insert Character (ICH): Inserts one or more blank characters at cursor position.
/// Characters to the right shift right, rightmost characters lost.
///
/// Sequence: `CSI n @`
///
/// [`CSI`]: crate::CsiSequence
pub const ICH_INSERT_CHAR: char = '@';

/// Erase Character (ECH): Erases one or more characters at cursor position.
/// Characters are replaced with blanks, no shifting occurs.
///
/// Sequence: `CSI n X`
///
/// [`CSI`]: crate::CsiSequence
pub const ECH_ERASE_CHAR: char = 'X';

// Additional Cursor Positioning.

/// Vertical Position Absolute (VPA): Moves cursor to specified row (default 1).
/// Horizontal position unchanged.
///
/// Sequence: `CSI n d`
///
/// [`CSI`]: crate::CsiSequence
pub const VPA_VERTICAL_POSITION: char = 'd';

// Text Formatting (SGR).

/// Select Graphic Rendition ([`SGR`]): Sets colors and text attributes.
///
/// Value: `'m'` dec, `6D` hex.
///
/// Sequence: `CSI n m`.
///
/// [`CSI`]: crate::CsiSequence
/// [`SGR`]: crate::SgrCode
pub const SGR_SET_GRAPHICS: char = 'm';

// SGR Parameters.

/// Reset All Attributes ([`SGR`] `0`): Resets all text attributes to default.
///
/// Value: `0`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_RESET: u16 = 0;

/// Bold ([`SGR`] `1`): Enables bold/bright text.
///
/// Value: `1`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_BOLD: u16 = 1;

/// Dim ([`SGR`] `2`): Enables dim/faint text.
///
/// Value: `2`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_DIM: u16 = 2;

/// Italic ([`SGR`] `3`): Enables italic text.
///
/// Value: `3`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_ITALIC: u16 = 3;

/// Underline ([`SGR`] `4`): Enables underlined text.
///
/// Value: `4`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_UNDERLINE: u16 = 4;

/// Slow Blink ([`SGR`] `5`): Enables slow blinking text.
///
/// Value: `5`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_BLINK: u16 = 5;

/// Rapid Blink ([`SGR`] `6`): Enables rapid blinking text.
///
/// Value: `6`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_RAPID_BLINK: u16 = 6;

/// Reverse ([`SGR`] `7`): Swaps foreground and background colors.
///
/// Value: `7`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_REVERSE: u16 = 7;

/// Hidden ([`SGR`] `8`): Conceals text (hidden/invisible).
///
/// Value: `8`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_HIDDEN: u16 = 8;

/// Strikethrough ([`SGR`] `9`): Enables strikethrough text.
///
/// Value: `9`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_STRIKETHROUGH: u16 = 9;

/// Reset Bold/Dim ([`SGR`] `22`): Disables bold and dim attributes.
///
/// Value: `22`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_RESET_BOLD_DIM: u16 = 22;

/// Reset Italic ([`SGR`] `23`): Disables italic attribute.
///
/// Value: `23`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_RESET_ITALIC: u16 = 23;

/// Reset Underline ([`SGR`] `24`): Disables underline attribute.
///
/// Value: `24`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_RESET_UNDERLINE: u16 = 24;

/// Reset Blink ([`SGR`] `25`): Disables blink attribute.
///
/// Value: `25`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_RESET_BLINK: u16 = 25;

/// Reset Reverse ([`SGR`] `27`): Disables reverse/inverse attribute.
///
/// Value: `27`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_RESET_REVERSE: u16 = 27;

/// Reset Hidden ([`SGR`] `28`): Disables hidden/conceal attribute.
///
/// Value: `28`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_RESET_HIDDEN: u16 = 28;

/// Reset Strikethrough ([`SGR`] `29`): Disables strikethrough attribute.
///
/// Value: `29`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_RESET_STRIKETHROUGH: u16 = 29;

// Foreground Colors (30-37, 90-97).

/// Black Foreground ([`SGR`] `30`): Sets text color to black.
///
/// Value: `30`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_FG_BLACK: u16 = 30;

/// Red Foreground ([`SGR`] `31`): Sets text color to red.
///
/// Value: `31`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_FG_RED: u16 = 31;

/// Green Foreground ([`SGR`] `32`): Sets text color to green.
///
/// Value: `32`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_FG_GREEN: u16 = 32;

/// Yellow Foreground ([`SGR`] `33`): Sets text color to yellow.
///
/// Value: `33`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_FG_YELLOW: u16 = 33;

/// Blue Foreground ([`SGR`] `34`): Sets text color to blue.
///
/// Value: `34`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_FG_BLUE: u16 = 34;

/// Magenta Foreground ([`SGR`] `35`): Sets text color to magenta.
///
/// Value: `35`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_FG_MAGENTA: u16 = 35;

/// Cyan Foreground ([`SGR`] `36`): Sets text color to cyan.
///
/// Value: `36`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_FG_CYAN: u16 = 36;

/// White Foreground ([`SGR`] `37`): Sets text color to white/gray.
///
/// Value: `37`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_FG_WHITE: u16 = 37;

/// Default Foreground ([`SGR`] `39`): Resets text color to terminal default.
///
/// Value: `39`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_FG_DEFAULT: u16 = 39;

/// Bright Black Foreground ([`SGR`] `90`): Sets text color to bright black (dark gray).
///
/// Value: `90`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_FG_BRIGHT_BLACK: u16 = 90;

/// Bright Red Foreground ([`SGR`] `91`): Sets text color to bright red.
///
/// Value: `91`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_FG_BRIGHT_RED: u16 = 91;

/// Bright Green Foreground ([`SGR`] `92`): Sets text color to bright green.
///
/// Value: `92`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_FG_BRIGHT_GREEN: u16 = 92;

/// Bright Yellow Foreground ([`SGR`] `93`): Sets text color to bright yellow.
///
/// Value: `93`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_FG_BRIGHT_YELLOW: u16 = 93;

/// Bright Blue Foreground ([`SGR`] `94`): Sets text color to bright blue.
///
/// Value: `94`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_FG_BRIGHT_BLUE: u16 = 94;

/// Bright Magenta Foreground ([`SGR`] `95`): Sets text color to bright magenta.
///
/// Value: `95`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_FG_BRIGHT_MAGENTA: u16 = 95;

/// Bright Cyan Foreground ([`SGR`] `96`): Sets text color to bright cyan.
///
/// Value: `96`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_FG_BRIGHT_CYAN: u16 = 96;

/// Bright White Foreground ([`SGR`] `97`): Sets text color to bright white.
///
/// Value: `97`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_FG_BRIGHT_WHITE: u16 = 97;

// Background Colors (40-47, 100-107).

/// Black Background ([`SGR`] `40`): Sets background color to black.
///
/// Value: `40`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_BG_BLACK: u16 = 40;

/// Red Background ([`SGR`] `41`): Sets background color to red.
///
/// Value: `41`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_BG_RED: u16 = 41;

/// Green Background ([`SGR`] `42`): Sets background color to green.
///
/// Value: `42`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_BG_GREEN: u16 = 42;

/// Yellow Background ([`SGR`] `43`): Sets background color to yellow.
///
/// Value: `43`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_BG_YELLOW: u16 = 43;

/// Blue Background ([`SGR`] `44`): Sets background color to blue.
///
/// Value: `44`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_BG_BLUE: u16 = 44;

/// Magenta Background ([`SGR`] `45`): Sets background color to magenta.
///
/// Value: `45`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_BG_MAGENTA: u16 = 45;

/// Cyan Background ([`SGR`] `46`): Sets background color to cyan.
///
/// Value: `46`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_BG_CYAN: u16 = 46;

/// White Background ([`SGR`] `47`): Sets background color to white/gray.
///
/// Value: `47`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_BG_WHITE: u16 = 47;

/// Default Background ([`SGR`] `49`): Resets background color to terminal default.
///
/// Value: `49`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_BG_DEFAULT: u16 = 49;

/// Bright Black Background ([`SGR`] `100`): Sets background color to bright black (dark
/// gray).
///
/// Value: `100`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_BG_BRIGHT_BLACK: u16 = 100;

/// Bright Red Background ([`SGR`] `101`): Sets background color to bright red.
///
/// Value: `101`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_BG_BRIGHT_RED: u16 = 101;

/// Bright Green Background ([`SGR`] `102`): Sets background color to bright green.
///
/// Value: `102`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_BG_BRIGHT_GREEN: u16 = 102;

/// Bright Yellow Background ([`SGR`] `103`): Sets background color to bright yellow.
///
/// Value: `103`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_BG_BRIGHT_YELLOW: u16 = 103;

/// Bright Blue Background ([`SGR`] `104`): Sets background color to bright blue.
///
/// Value: `104`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_BG_BRIGHT_BLUE: u16 = 104;

/// Bright Magenta Background ([`SGR`] `105`): Sets background color to bright magenta.
///
/// Value: `105`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_BG_BRIGHT_MAGENTA: u16 = 105;

/// Bright Cyan Background ([`SGR`] `106`): Sets background color to bright cyan.
///
/// Value: `106`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_BG_BRIGHT_CYAN: u16 = 106;

/// Bright White Background ([`SGR`] `107`): Sets background color to bright white.
///
/// Value: `107`.
///
/// [`SGR`]: crate::SgrCode
pub const SGR_BG_BRIGHT_WHITE: u16 = 107;

// Extended Color Support (256-color and RGB).

/// Extended Foreground Color ([`SGR`] `38`): Introduces extended foreground color
/// sequences.
///
/// Value: `38`.
///
/// Used in sequences like:
/// - `ESC [ 38 : 5 : n m` - 256-color foreground (n = 0-255)
/// - `ESC [ 38 : 2 : r : g : b m` - RGB foreground (r,g,b = 0-255)
///
/// [`SGR`]: crate::SgrCode
pub const SGR_FG_EXTENDED: u16 = 38;

/// Extended Background Color ([`SGR`] `48`): Introduces extended background color
/// sequences.
///
/// Value: `48`.
///
/// Used in sequences like:
/// - `ESC [ 48 : 5 : n m` - 256-color background (n = 0-255)
/// - `ESC [ 48 : 2 : r : g : b m` - RGB background (r,g,b = 0-255)
///
/// [`SGR`]: crate::SgrCode
pub const SGR_BG_EXTENDED: u16 = 48;

/// 256-Color Mode Indicator ([`SGR`] `5`): Second parameter in 256-color sequences.
///
/// Value: `5`.
///
/// Used in sequences like:
/// - `ESC [ 38 : 5 : n m` - 256-color foreground
/// - `ESC [ 48 : 5 : n m` - 256-color background
///
/// [`SGR`]: crate::SgrCode
pub const SGR_COLOR_MODE_256: u16 = 5;

/// RGB Color Mode Indicator ([`SGR`] `2`): Second parameter in RGB color sequences.
///
/// Value: `2`.
///
/// Used in sequences like:
/// - `ESC [ 38 : 2 : r : g : b m` - RGB foreground
/// - `ESC [ 48 : 2 : r : g : b m` - RGB background
///
/// [`SGR`]: crate::SgrCode
pub const SGR_COLOR_MODE_RGB: u16 = 2;

// Cursor Save/Restore (CSI versions).

define_ansi_const!(@csi_str : SCP_SAVE_CURSOR_STR = ["s"] =>
    "Save Cursor (SCP)" : "Save cursor position sequence string. Alternative to `ESC 7`."
);

/// Save Cursor Position (SCP): Complete sequence bytes, alternative to `ESC 7`.
///
/// Value: `\x1b[s`.
///
/// Sequence: `ESC [ s`.
///
/// [`CSI`]: crate::CsiSequence
pub const SCP_SAVE_CURSOR_BYTES: &[u8] = b"\x1b[s";

/// Save Cursor Position (SCP): Final byte for save cursor sequence, alternative to `ESC
/// 7`.
///
/// Value: `'s'` dec, `73` hex.
///
/// Sequence: `CSI s`.
///
/// [`CSI`]: crate::CsiSequence
pub const SCP_SAVE_CURSOR: char = 's';

define_ansi_const!(@csi_str : RCP_RESTORE_CURSOR_STR = ["u"] =>
    "Restore Cursor (RCP)" : "Restore cursor position sequence string. Alternative to `ESC 8`."
);

/// Restore Cursor Position (RCP): Complete sequence bytes, alternative to `ESC 8`.
///
/// Value: `\x1b[u`.
///
/// Sequence: `ESC [ u`.
///
/// [`CSI`]: crate::CsiSequence
pub const RCP_RESTORE_CURSOR_BYTES: &[u8] = b"\x1b[u";

/// Restore Cursor Position (RCP): Final byte for restore cursor sequence, alternative to
/// `ESC 8`.
///
/// Value: `'u'` dec, `75` hex.
///
/// Sequence: `CSI u`.
///
/// [`CSI`]: crate::CsiSequence
pub const RCP_RESTORE_CURSOR: char = 'u';

// Erase Display (ED) Full Sequences.

define_ansi_const!(@csi_str : CSI_ERASE_DISPLAY_ALL = ["2J"] =>
    "Erase Display All (ED 2)" : "Erase entire screen sequence string."
);

// Device Status.

/// Device Status Report ([`DSR`]): Requests device status or cursor position.
/// `5` = request status, `6` = request cursor position.
///
/// Value: `'n'` dec, `6E` hex.
///
/// Sequence: `CSI n n`.
///
/// [`CSI`]: crate::CsiSequence
/// [`DSR`]: crate::DsrSequence
pub const DSR_DEVICE_STATUS: char = 'n';

// Mode Setting.

/// Set Mode (SM): Sets various terminal modes.
///
/// Value: `'h'` dec, `68` hex.
///
/// Sequence: `CSI n h`.
///
/// [`CSI`]: crate::CsiSequence
pub const SM_SET_MODE: char = 'h';

/// Reset Mode (RM): Resets various terminal modes.
///
/// Value: `'l'` dec, `6C` hex.
///
/// Sequence: `CSI n l`.
///
/// [`CSI`]: crate::CsiSequence
pub const RM_RESET_MODE: char = 'l';

// Private Mode Setting (with ? prefix).

/// Set Private Mode (SM): Sets [`DEC`] private modes.
///
/// Value: `'h'` dec, `68` hex.
///
/// Sequence: `CSI ? n h`.
///
/// [`CSI`]: crate::CsiSequence
/// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
pub const SM_SET_PRIVATE_MODE: char = 'h';

/// Reset Private Mode (RM): Resets [`DEC`] private modes.
///
/// Value: `'l'` dec, `6C` hex.
///
/// Sequence: `CSI ? n l`.
///
/// [`CSI`]: crate::CsiSequence
/// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
pub const RM_RESET_PRIVATE_MODE: char = 'l';

// Common Private Mode Numbers.

/// Show Cursor (DECTCEM): Private mode number `25` to show the cursor.
///
/// [`CSI`]: crate::CsiSequence
pub const DECTCEM_SHOW_CURSOR: u16 = 25;

// NOTE: General terminal mode constants (DEC modes 1-7, alternate screen buffer,
// mouse tracking, bracketed paste, etc.) are defined in the parent `protocols`
// module constants file (`protocols/generic_ansi_constants.rs`) rather than here.
//
// This separation reflects the architectural distinction:
// - **This file (`csi_codes/csi_constants.rs`)**: CSI-specific sequencing details
// - **Parent file (`protocols/generic_ansi_constants.rs`)**: General ANSI/terminal
//   feature constants
