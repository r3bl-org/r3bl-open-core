// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Erase mode enums for ED (Erase Display) and EL (Erase Line) CSI sequences.
//!
//! These enums make illegal states unrepresentable by restricting the valid values
//! for erase operations to only those defined by the ECMA-48 standard.

use crate::NumericConversions;

/// Erase display modes for ED (Erase in Display) - `ESC [ n J`.
///
/// Per ECMA-48 / VT100 specification, only values 0-3 are valid.
/// Using an enum makes invalid modes (like 5) unrepresentable.
///
/// # Example
///
/// ```rust
/// use r3bl_tui::{EraseDisplayMode, CsiSequence};
///
/// // Clear from cursor to end of screen
/// let clear_below = CsiSequence::EraseDisplay(EraseDisplayMode::FromCursorToEnd);
///
/// // Clear entire screen (like the `clear` command)
/// let clear_all = CsiSequence::EraseDisplay(EraseDisplayMode::EntireScreen);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EraseDisplayMode {
    /// Erase from cursor to end of screen (ED 0) - default.
    ///
    /// Clears all characters from the cursor position to the end of the screen,
    /// including the character at the cursor.
    #[default]
    FromCursorToEnd = 0,

    /// Erase from start of screen to cursor (ED 1).
    ///
    /// Clears all characters from the beginning of the screen to the cursor
    /// position, including the character at the cursor.
    FromStartToCursor = 1,

    /// Erase entire screen (ED 2).
    ///
    /// Clears all characters on the screen. Cursor position is not changed.
    /// This is equivalent to the `clear` command behavior.
    EntireScreen = 2,

    /// Erase entire screen and scrollback buffer (ED 3).
    ///
    /// Clears all characters on the screen AND the scrollback buffer.
    /// This is an xterm extension, not part of the original VT100 spec.
    EntireScreenAndScrollback = 3,
}

impl EraseDisplayMode {
    /// Convert from a raw parameter value.
    ///
    /// Returns the default mode ([`FromCursorToEnd`]) for invalid values,
    /// matching VT100 terminal behavior.
    ///
    /// [`FromCursorToEnd`]: Self::FromCursorToEnd
    #[must_use]
    pub const fn from_param(value: u16) -> Self {
        match value {
            1 => Self::FromStartToCursor,
            2 => Self::EntireScreen,
            3 => Self::EntireScreenAndScrollback,
            // 0 is the default per VT100 spec; invalid values also default to this.
            _ => Self::FromCursorToEnd,
        }
    }
}

impl NumericConversions for EraseDisplayMode {
    fn as_usize(&self) -> usize { *self as usize }
    fn as_u16(&self) -> u16 { *self as u16 }
}

impl From<u16> for EraseDisplayMode {
    fn from(value: u16) -> Self { Self::from_param(value) }
}

/// Erase line modes for EL (Erase in Line) - `ESC [ n K`.
///
/// Per ECMA-48 / VT100 specification, only values 0-2 are valid.
/// Using an enum makes invalid modes (like 5) unrepresentable.
///
/// # Example
///
/// ```rust
/// use r3bl_tui::{EraseLineMode, CsiSequence};
///
/// // Clear from cursor to end of line
/// let clear_right = CsiSequence::EraseLine(EraseLineMode::FromCursorToEnd);
///
/// // Clear entire line
/// let clear_line = CsiSequence::EraseLine(EraseLineMode::EntireLine);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EraseLineMode {
    /// Erase from cursor to end of line (EL 0) - default.
    ///
    /// Clears all characters from the cursor position to the end of the line,
    /// including the character at the cursor.
    #[default]
    FromCursorToEnd = 0,

    /// Erase from start of line to cursor (EL 1).
    ///
    /// Clears all characters from the beginning of the line to the cursor
    /// position, including the character at the cursor.
    FromStartToCursor = 1,

    /// Erase entire line (EL 2).
    ///
    /// Clears all characters on the current line. Cursor position is not changed.
    EntireLine = 2,
}

impl EraseLineMode {
    /// Convert from a raw parameter value.
    ///
    /// Returns the default mode ([`FromCursorToEnd`]) for invalid values,
    /// matching VT100 terminal behavior.
    ///
    /// [`FromCursorToEnd`]: Self::FromCursorToEnd
    #[must_use]
    pub const fn from_param(value: u16) -> Self {
        match value {
            1 => Self::FromStartToCursor,
            2 => Self::EntireLine,
            // 0 is the default per VT100 spec; invalid values also default to this.
            _ => Self::FromCursorToEnd,
        }
    }
}

impl NumericConversions for EraseLineMode {
    fn as_usize(&self) -> usize { *self as usize }
    fn as_u16(&self) -> u16 { *self as u16 }
}

impl From<u16> for EraseLineMode {
    fn from(value: u16) -> Self { Self::from_param(value) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_erase_display_mode_from_param() {
        assert_eq!(EraseDisplayMode::from_param(0), EraseDisplayMode::FromCursorToEnd);
        assert_eq!(EraseDisplayMode::from_param(1), EraseDisplayMode::FromStartToCursor);
        assert_eq!(EraseDisplayMode::from_param(2), EraseDisplayMode::EntireScreen);
        assert_eq!(
            EraseDisplayMode::from_param(3),
            EraseDisplayMode::EntireScreenAndScrollback
        );
        // Invalid values default to FromCursorToEnd
        assert_eq!(EraseDisplayMode::from_param(4), EraseDisplayMode::FromCursorToEnd);
        assert_eq!(EraseDisplayMode::from_param(99), EraseDisplayMode::FromCursorToEnd);
    }

    #[test]
    fn test_erase_display_mode_as_u16() {
        assert_eq!(EraseDisplayMode::FromCursorToEnd.as_u16(), 0);
        assert_eq!(EraseDisplayMode::FromStartToCursor.as_u16(), 1);
        assert_eq!(EraseDisplayMode::EntireScreen.as_u16(), 2);
        assert_eq!(EraseDisplayMode::EntireScreenAndScrollback.as_u16(), 3);
    }

    #[test]
    fn test_erase_display_mode_default() {
        assert_eq!(EraseDisplayMode::default(), EraseDisplayMode::FromCursorToEnd);
    }

    #[test]
    fn test_erase_line_mode_from_param() {
        assert_eq!(EraseLineMode::from_param(0), EraseLineMode::FromCursorToEnd);
        assert_eq!(EraseLineMode::from_param(1), EraseLineMode::FromStartToCursor);
        assert_eq!(EraseLineMode::from_param(2), EraseLineMode::EntireLine);
        // Invalid values default to FromCursorToEnd
        assert_eq!(EraseLineMode::from_param(3), EraseLineMode::FromCursorToEnd);
        assert_eq!(EraseLineMode::from_param(99), EraseLineMode::FromCursorToEnd);
    }

    #[test]
    fn test_erase_line_mode_as_u16() {
        assert_eq!(EraseLineMode::FromCursorToEnd.as_u16(), 0);
        assert_eq!(EraseLineMode::FromStartToCursor.as_u16(), 1);
        assert_eq!(EraseLineMode::EntireLine.as_u16(), 2);
    }

    #[test]
    fn test_erase_line_mode_default() {
        assert_eq!(EraseLineMode::default(), EraseLineMode::FromCursorToEnd);
    }

    #[test]
    fn test_from_trait_implementations() {
        let display_mode: EraseDisplayMode = 2_u16.into();
        assert_eq!(display_mode, EraseDisplayMode::EntireScreen);

        let line_mode: EraseLineMode = 1_u16.into();
        assert_eq!(line_mode, EraseLineMode::FromStartToCursor);
    }
}
