/*
 *   Copyright (c) 2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

use std::fmt::Debug;

use r3bl_core::{ch, i16, RingBuffer as _};

use super::sizing;

pub const MIN_INDEX: CurIndexNumber = -1;
type CurIndexNumber = i16;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CurIndex(pub CurIndexNumber);

mod construct {
    use super::*;
    impl Default for CurIndex {
        fn default() -> Self { Self(MIN_INDEX) }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CurIndexLoc {
    EmptyHistory,
    Start(CurIndex),
    End(CurIndex),
    Middle(CurIndex),
}

impl CurIndexLoc {
    /// Determine the location of the current index in the history buffer.
    pub fn locate(cur_index: &CurIndex, versions: &sizing::HistoryBuffer) -> CurIndexLoc {
        if versions.is_empty() {
            CurIndexLoc::EmptyHistory
        } else if cur_index.0 == MIN_INDEX {
            CurIndexLoc::Start(*cur_index)
        } else if cur_index.0 == {
            // REVIEW: [ ] introduce Length and Index "newtype" in r3bl_core to replace off by one code below
            let max_index = ch(versions.len()) - ch(1);
            i16(max_index)
        } {
            CurIndexLoc::End(*cur_index)
        } else {
            CurIndexLoc::Middle(*cur_index)
        }
    }

    /// Increment the current index. If the current index is at the end of the history
    /// buffer, or the buffer is empty, this does nothing.
    pub fn inc(cur_index: &mut CurIndex, versions: &sizing::HistoryBuffer) {
        match Self::locate(cur_index, versions) {
            Self::EmptyHistory => {
                // Is empty. Nothing to increment.
            }
            Self::End(_) => {
                // Already at end of history buffer. Nothing to increment.
            }
            Self::Start(_) | Self::Middle(_) => {
                // Increment index.
                cur_index.0 += 1;
            }
        }
    }

    /// Decrement the current index. If the current index is at the start of the history
    /// buffer, or the buffer is empty, this does nothing.
    pub fn dec(cur_index: &mut CurIndex, versions: &sizing::HistoryBuffer) {
        match Self::locate(cur_index, versions) {
            Self::EmptyHistory => {
                // Is empty. Nothing to decrement.
            }
            Self::Start(_) => {
                // Already at start of history buffer. Nothing to decrement.
            }
            Self::End(_) | Self::Middle(_) => {
                // Decrement index.
                cur_index.0 -= 1;
            }
        }
    }
}

impl CurIndex {
    /// This won't be negative. Even if a negative number is passed in, it will be
    /// converted to 0.
    pub fn as_usize(self) -> usize {
        if self.0 < 0 {
            0
        } else {
            self.0.try_into().unwrap_or(self.0 as usize)
        }
    }

    /// Reset the current index to the start of the history buffer.
    pub fn clear(&mut self) { self.0 = MIN_INDEX; }
}

mod convert {
    use super::*;

    impl From<usize> for CurIndex {
        fn from(val: usize) -> Self { Self(val as CurIndexNumber) }
    }

    impl From<isize> for CurIndex {
        fn from(val: isize) -> Self { Self(val as CurIndexNumber) }
    }

    impl From<i32> for CurIndex {
        fn from(val: i32) -> Self { Self(val as CurIndexNumber) }
    }

    impl From<i16> for CurIndex {
        fn from(val: i16) -> Self { Self(val) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EditorContent;

    #[test]
    fn test_cur_index_locate_empty() {
        let versions = sizing::HistoryBuffer::new();
        let cur_index = CurIndex::default();
        assert_eq!(
            CurIndexLoc::locate(&cur_index, &versions),
            CurIndexLoc::EmptyHistory
        );
    }

    #[test]
    fn test_cur_index_locate_start() {
        let mut versions = sizing::HistoryBuffer::new();
        versions.add(EditorContent::default());
        let cur_index = CurIndex::default();
        assert_eq!(
            CurIndexLoc::locate(&cur_index, &versions),
            CurIndexLoc::Start(cur_index)
        );
    }

    #[test]
    fn test_cur_index_locate_end() {
        let mut versions = sizing::HistoryBuffer::new();
        versions.add(EditorContent::default());
        let cur_index = CurIndex::from(0);
        assert_eq!(
            CurIndexLoc::locate(&cur_index, &versions),
            CurIndexLoc::End(cur_index)
        );
    }

    #[test]
    fn test_cur_index_locate_middle() {
        let mut versions = sizing::HistoryBuffer::new();
        versions.add(EditorContent::default());
        versions.add(EditorContent::default());
        let cur_index = CurIndex::from(0);
        assert_eq!(
            CurIndexLoc::locate(&cur_index, &versions),
            CurIndexLoc::Middle(cur_index)
        );
    }
}
