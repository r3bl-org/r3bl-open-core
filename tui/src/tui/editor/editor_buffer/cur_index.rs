// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{Index, Length, LengthOps, NumericValue, idx};
use std::{fmt::Debug,
          ops::{Deref, DerefMut}};

/// The current index in the history buffer.
///
/// This index is used to keep track of the current version in the history buffer. It
/// works with the history buffer [`super::history::EditorHistory`] to allow undoing and
/// redoing actions.
///
/// - If it's `None`, then the current index is at the start of the history buffer. This
///   does not mean that the history buffer is empty. The current index can be `None` and
///   the length of the buffer can be greater than 0.
/// - If it's `Some(index)`, then the current index is at the index in the history buffer.
///   Redoing an action will increment the index. Undoing an action will decrement the
///   index.
/// - Undoing and then redoing will truncate / remove all the "dangling" redo versions.
/// - If the current index is at the end of the history buffer, then there are no redo
///   versions.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct CurIndex(pub Option<Index>);

/// This is a state machine that represents the location of the current index in the
/// history buffer.
///
/// - It encodes all the possible states that the current index can be in as it is
///   manipulated using [`Self::inc`] and [`Self::dec`].
/// - This state information can be queried using [`Self::locate`].
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CurIndexLoc {
    /// The history buffer is empty. Regardless of the current index, there are no
    /// versions to undo or redo.
    EmptyHistory,
    /// Current index is None.
    Start,
    /// Current index is Some(it), where it >= 0.
    End(Index),
    /// Current index is Some(it), where it >= 0.
    Middle(Index),
}

impl CurIndexLoc {
    /// Determine the location of the current index in the history buffer.
    #[must_use]
    pub fn locate(cur_index: &CurIndex, versions_len: Length) -> CurIndexLoc {
        if versions_len.is_zero() {
            // Is empty.
            return CurIndexLoc::EmptyHistory;
        }

        match cur_index.0 {
            None => {
                // cur_index is None.
                CurIndexLoc::Start
            }
            Some(inner) => {
                if inner == versions_len.convert_to_index() {
                    CurIndexLoc::End(inner)
                } else {
                    CurIndexLoc::Middle(inner)
                }
            }
        }
    }

    /// Increment the current index.
    /// - If it's a `None`, set it to `Some(0)`.
    /// - If the current index is at the end of the history buffer, or the buffer is
    ///   empty, this does nothing.
    pub fn inc(cur_index: &mut CurIndex, versions_len: Length) {
        match Self::locate(cur_index, versions_len) {
            Self::EmptyHistory | Self::End(_) => {
                // Either:
                // - EmptyHistory -> Nothing to increment.
                // - Already at end of history buffer -> Nothing to increment.
            }
            Self::Start => {
                // Set index to Some(0) from None.
                cur_index.0 = Some(idx(0));
            }
            Self::Middle(_) => {
                // Increment index.
                if let Some(index) = cur_index.0 {
                    cur_index.0 = Some(index + idx(1));
                }
            }
        }
    }

    /// Decrement the current index.
    /// - If it's at `Some(0)` then set it to `None`.
    /// - If the current index is at the start of the history buffer, or the buffer is
    ///   empty, this does nothing.
    pub fn dec(cur_index: &mut CurIndex, versions_len: Length) {
        match Self::locate(cur_index, versions_len) {
            Self::EmptyHistory => {
                // Is empty. Nothing to decrement.
            }
            Self::Start => {
                // Already at start of history buffer. Nothing to decrement.
                cur_index.0 = None;
            }
            Self::End(_) | Self::Middle(_) => {
                if let Some(index) = cur_index.0 {
                    if index > idx(0) {
                        // Decrement index.
                        cur_index.0 = Some(index - idx(1));
                    } else {
                        // Set index to None from Some(0).
                        cur_index.0 = None;
                    }
                }
            }
        }
    }
}

impl CurIndex {
    /// If `self.0` is None, it will be converted to 0.
    #[must_use]
    pub fn as_index(self) -> Index { self.0.unwrap_or(idx(0)) }

    /// Reset the current index to the start of the history buffer.
    pub fn clear(&mut self) { self.0 = None; }
}

mod impl_deref {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Deref for CurIndex {
        type Target = Option<Index>;

        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl DerefMut for CurIndex {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }
}

mod convert {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl From<usize> for CurIndex {
        fn from(val: usize) -> Self { CurIndex(Some(Index(val.into()))) }
    }

    impl From<isize> for CurIndex {
        fn from(val: isize) -> Self { CurIndex(Some(Index(val.into()))) }
    }

    impl From<i32> for CurIndex {
        fn from(val: i32) -> Self { CurIndex(Some(Index(val.into()))) }
    }

    impl From<i16> for CurIndex {
        fn from(val: i16) -> Self { CurIndex(Some(Index(val.into()))) }
    }
}

#[cfg(test)]
#[allow(clippy::useless_vec)]
mod tests {
    use super::*;
    use crate::{EditorContent, len};

    #[test]
    fn test_cur_index_locate_empty() {
        let versions = Vec::<EditorContent>::new();
        let cur_index = CurIndex::default();
        assert_eq!(
            CurIndexLoc::locate(&cur_index, len(versions.len())),
            CurIndexLoc::EmptyHistory
        );
    }

    #[test]
    fn test_cur_index_locate_start() {
        let versions = vec![EditorContent::default()];
        let cur_index = CurIndex::default();
        assert_eq!(
            CurIndexLoc::locate(&cur_index, len(versions.len())),
            CurIndexLoc::Start
        );
    }

    #[test]
    fn test_cur_index_locate_end() {
        let versions = vec![EditorContent::default()];
        let cur_index = CurIndex::from(0);
        assert_eq!(
            CurIndexLoc::locate(&cur_index, len(versions.len())),
            CurIndexLoc::End(cur_index.as_index())
        );
    }

    #[test]
    fn test_cur_index_locate_middle() {
        let versions = vec![EditorContent::default(), EditorContent::default()];
        let cur_index = CurIndex::from(0);
        assert_eq!(
            CurIndexLoc::locate(&cur_index, len(versions.len())),
            CurIndexLoc::Middle(cur_index.as_index())
        );
    }
}
