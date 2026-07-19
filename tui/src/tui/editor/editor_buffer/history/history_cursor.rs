// Copyright (c) 2025-2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{CIndex, CLength, LengthOps, WideningCastToI32, c_index};
use std::{fmt::Debug,
          ops::{Deref, DerefMut}};

/// The current cursor position in the history buffer.
///
/// This cursor keeps track of the current version in the history buffer. It works with
/// the history buffer [`EditorHistory`] to allow undoing and redoing actions.
///
/// - If it's `None`, then the current cursor is at the start of the history buffer. This
///   does not mean that the history buffer is empty. The current cursor can be `None` and
///   the length of the buffer can be greater than 0.
/// - If it's `Some(index)`, then the current cursor is at the index in the history
///   buffer. Redoing an action will increment the cursor. Undoing an action will
///   decrement the cursor.
/// - Undoing and then redoing will truncate / remove all the "dangling" redo versions.
/// - If the current cursor is at the end of the history buffer, then there are no redo
///   versions.
///
/// [`EditorHistory`]: crate::EditorHistory
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct HistoryCursor(pub Option<CIndex>);

/// This is a state machine that represents the location of the current cursor in the
/// history buffer.
///
/// - It encodes all the possible states that the current cursor can be in as it is
///   manipulated using [`Self::inc`] and [`Self::dec`].
/// - This state information can be queried using [`Self::locate`].
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum HistoryCursorLoc {
    /// The history buffer is empty. Regardless of the current cursor, there are no
    /// versions to undo or redo.
    EmptyHistory,
    /// Current cursor is None.
    Start,
    /// Current cursor is Some(it), where it >= 0.
    End(CIndex),
    /// Current cursor is Some(it), where it >= 0.
    Middle(CIndex),
}

impl HistoryCursorLoc {
    /// Determine the location of the current cursor in the history buffer.
    #[must_use]
    pub fn locate(cursor: &HistoryCursor, versions_len: CLength) -> HistoryCursorLoc {
        if versions_len.is_empty() {
            return HistoryCursorLoc::EmptyHistory;
        }

        match cursor.0 {
            None => HistoryCursorLoc::Start,
            Some(inner) => {
                if inner == versions_len.convert_to_index() {
                    HistoryCursorLoc::End(inner)
                } else {
                    HistoryCursorLoc::Middle(inner)
                }
            }
        }
    }

    /// Increment the current cursor.
    /// - If it's a `None`, set it to `Some(c_index(0))`.
    /// - If the current cursor is at the end of the history buffer, or the buffer is
    ///   empty, this does nothing.
    pub fn inc(cursor: &mut HistoryCursor, versions_len: CLength) {
        match Self::locate(cursor, versions_len) {
            Self::EmptyHistory | Self::End(_) => {}
            Self::Start => {
                cursor.0 = Some(c_index(0));
            }
            Self::Middle(_) => {
                if let Some(index) = cursor.0 {
                    cursor.0 = Some(index + c_index(1));
                }
            }
        }
    }

    /// Decrement the current cursor.
    /// - If it's at `Some(c_index(0))` then set it to `None`.
    /// - If the current cursor is at the start of the history buffer, or the buffer is
    ///   empty, this does nothing.
    pub fn dec(cursor: &mut HistoryCursor, versions_len: CLength) {
        match Self::locate(cursor, versions_len) {
            Self::EmptyHistory => {}
            Self::Start => {
                cursor.0 = None;
            }
            Self::End(_) | Self::Middle(_) => {
                if let Some(index) = cursor.0 {
                    if index > c_index(0) {
                        cursor.0 = Some(index - 1i32);
                    } else {
                        cursor.0 = None;
                    }
                }
            }
        }
    }
}

impl HistoryCursor {
    /// If `self.0` is None, it will be converted to `c_index(0)`.
    #[must_use]
    pub fn as_index(self) -> CIndex { self.0.unwrap_or(c_index(0)) }

    /// Reset the history cursor to the start of the history buffer.
    pub fn clear(&mut self) { self.0 = None; }
}

mod impl_deref {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Deref for HistoryCursor {
        type Target = Option<CIndex>;

        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl DerefMut for HistoryCursor {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }
}

mod convert {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl From<usize> for HistoryCursor {
        fn from(val: usize) -> HistoryCursor { HistoryCursor(Some(c_index(val))) }
    }

    impl From<isize> for HistoryCursor {
        fn from(val: isize) -> HistoryCursor {
            // XMARK: Intentional numeric casting using as.
            #[allow(clippy::as_conversions, clippy::cast_sign_loss)]
            HistoryCursor(Some(c_index(val as usize)))
        }
    }

    impl From<i32> for HistoryCursor {
        fn from(val: i32) -> HistoryCursor { HistoryCursor(Some(c_index(val))) }
    }

    impl From<i16> for HistoryCursor {
        fn from(val: i16) -> HistoryCursor {
            HistoryCursor(Some(c_index(val.as_i32_widening())))
        }
    }
}

#[cfg(test)]
#[allow(clippy::useless_vec)]
mod tests {
    use super::*;
    use crate::{EditorContent, c_len};

    #[test]
    fn test_history_cursor_locate_empty() {
        let versions = Vec::<EditorContent>::new();
        let cursor = HistoryCursor::default();
        assert_eq!(
            HistoryCursorLoc::locate(&cursor, c_len(versions.len())),
            HistoryCursorLoc::EmptyHistory
        );
    }

    #[test]
    fn test_history_cursor_locate_start() {
        let versions = vec![EditorContent::default()];
        let cursor = HistoryCursor::default();
        assert_eq!(
            HistoryCursorLoc::locate(&cursor, c_len(versions.len())),
            HistoryCursorLoc::Start
        );
    }

    #[test]
    fn test_history_cursor_locate_end() {
        let versions = vec![EditorContent::default()];
        let cursor = HistoryCursor::from(0);
        assert_eq!(
            HistoryCursorLoc::locate(&cursor, c_len(versions.len())),
            HistoryCursorLoc::End(cursor.as_index())
        );
    }

    #[test]
    fn test_history_cursor_locate_middle() {
        let versions = vec![EditorContent::default(), EditorContent::default()];
        let cursor = HistoryCursor::from(0);
        assert_eq!(
            HistoryCursorLoc::locate(&cursor, c_len(versions.len())),
            HistoryCursorLoc::Middle(cursor.as_index())
        );
    }

    #[test]
    fn test_history_cursor_inc_dec() {
        let mut cursor = HistoryCursor::default();
        let len = c_len(3);

        // Start -> inc -> Some(0)
        HistoryCursorLoc::inc(&mut cursor, len);
        assert_eq!(cursor, HistoryCursor::from(0));

        // Middle Some(0) -> inc -> Some(1)
        HistoryCursorLoc::inc(&mut cursor, len);
        assert_eq!(cursor, HistoryCursor::from(1));

        // Middle Some(1) -> inc -> Some(2)
        HistoryCursorLoc::inc(&mut cursor, len);
        assert_eq!(cursor, HistoryCursor::from(2));

        // End Some(2) -> inc -> stays Some(2)
        HistoryCursorLoc::inc(&mut cursor, len);
        assert_eq!(cursor, HistoryCursor::from(2));

        // End Some(2) -> dec -> Some(1)
        HistoryCursorLoc::dec(&mut cursor, len);
        assert_eq!(cursor, HistoryCursor::from(1));

        // Middle Some(1) -> dec -> Some(0)
        HistoryCursorLoc::dec(&mut cursor, len);
        assert_eq!(cursor, HistoryCursor::from(0));

        // Some(0) -> dec -> None
        HistoryCursorLoc::dec(&mut cursor, len);
        assert_eq!(cursor, HistoryCursor::default());

        // Start None -> dec -> stays None
        HistoryCursorLoc::dec(&mut cursor, len);
        assert_eq!(cursor, HistoryCursor::default());
    }
}
