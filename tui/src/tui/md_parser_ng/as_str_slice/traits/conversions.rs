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

use crate::{idx, len, AsStrSlice, GCString, InlineVec, List};

/// Implement [From] trait to allow automatic conversion from &[`GCString`] to
/// [`AsStrSlice`].
impl<'a> From<&'a [GCString]> for AsStrSlice<'a> {
    fn from(lines: &'a [GCString]) -> Self {
        let total_size = Self::calculate_total_size(lines);
        Self {
            lines,
            line_index: idx(0),
            char_index: idx(0),
            max_len: None,
            total_size: len(total_size),
            current_taken: len(0),
        }
    }
}

/// Implement [From] trait to allow automatic conversion from &[[`GCString`]; N] to
/// [`AsStrSlice`]. Primary use case is for tests where the inputs are hardcoded as
/// fixed-size arrays.
impl<'a, const N: usize> From<&'a [GCString; N]> for AsStrSlice<'a> {
    fn from(lines: &'a [GCString; N]) -> Self {
        let lines_slice = lines.as_slice();
        let total_size = Self::calculate_total_size(lines_slice);
        Self {
            lines: lines_slice,
            line_index: idx(0),
            char_index: idx(0),
            max_len: None,
            total_size: len(total_size),
            current_taken: len(0),
        }
    }
}

/// Implement [From] trait to allow automatic conversion from &[`Vec<GCString>`] to
/// [`AsStrSlice`].
impl<'a> From<&'a Vec<GCString>> for AsStrSlice<'a> {
    fn from(lines: &'a Vec<GCString>) -> Self {
        let total_size = Self::calculate_total_size(lines);
        Self {
            lines,
            line_index: idx(0),
            char_index: idx(0),
            max_len: None,
            total_size: len(total_size),
            current_taken: len(0),
        }
    }
}

/// Integrate with [`crate::List`] so that `List::from()` will work for
/// `InlineVec<AsStrSlice>`.
impl<'a> From<InlineVec<AsStrSlice<'a>>> for List<AsStrSlice<'a>> {
    fn from(other: InlineVec<AsStrSlice<'a>>) -> Self {
        let mut it = List::with_capacity(other.len());
        it.extend(other);
        it
    }
}
