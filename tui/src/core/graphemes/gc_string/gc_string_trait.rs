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

use super::owned::{gc_string_owned::wide_segments::ContainsWideSegments, GCStringOwned};
use crate::{ChUnit, ColIndex, ColWidth, Seg, SegIndex, SegWidth};

pub fn gc_string_owned(arg_from: impl Into<GCStringOwned>) -> GCStringOwned {
    arg_from.into()
}

/// `GCString` trait that provides an abstraction over string-like types that work with
/// graphemes. This trait defines the essential operations for Unicode text handling
/// without exposing ownership details of the underlying implementation.
///
/// This trait can be implemented by various string types (owned, borrowed, etc.) to
/// provide a unified interface for grapheme cluster operations throughout the codebase.
///
/// ## Associated Type Benefits
///
/// The `StringResult` associated type allows each implementation to return its
/// appropriate string result type:
/// - `GCStringOwned` returns `SegStringOwned` (owns the string data)
/// - Future `GCStringRef` can return `SegStringRef` (borrows the string data)
/// - Custom implementations can define their own result types
///
/// This design maintains type safety while allowing the trait to work with both owned
/// and borrowed string types without forcing unnecessary allocations or copies.
///
/// ## Example Usage
///
/// ```rust
/// use r3bl_tui::{GCString, GCStringOwned, col};
///
/// fn process_string<T: GCString>(input: &T) -> Option<T::StringResult> {
///     input.get_string_at(col(5))
/// }
///
/// // Works with owned strings
/// let owned = GCStringOwned::new("Hello, world!");
/// let result_owned = process_string(&owned); // Returns Option<SegStringOwned>
///
/// // Will work with borrowed strings (future implementation)
/// // let borrowed = GCStringRef::new("Hello, world!");
/// // let result_ref = process_string(&borrowed); // Returns Option<SegStringRef>
/// ```
pub trait GCString {
    /// Associated type for the string result type this implementation returns.
    /// This allows each implementation to define its own return type for string slicing
    /// operations, enabling flexibility for both owned and borrowed string types.
    type StringResult: Debug;

    /// Returns the number of grapheme clusters in this grapheme string.
    fn len(&self) -> SegWidth;

    /// Returns true if the string contains no grapheme clusters.
    fn is_empty(&self) -> bool;

    /// Returns the maximum segment index of this grapheme string.
    fn get_max_seg_index(&self) -> SegIndex;

    /// Get a segment at the given index.
    fn get(&self, seg_index: impl Into<SegIndex>) -> Option<Seg>;

    /// Returns an iterator over the segments in this grapheme string.
    fn seg_iter(&self) -> Box<dyn Iterator<Item = &Seg> + '_>;

    /// Returns an iterator over the grapheme cluster segments.
    fn iter(&self) -> Box<dyn Iterator<Item = Seg> + '_>;

    /// Returns the underlying string as a string slice.
    fn as_str(&self) -> &str;

    /// Returns the display width of this grapheme string.
    fn display_width(&self) -> ColWidth;

    /// Returns the byte size of the underlying string.
    fn bytes_size(&self) -> ChUnit;

    /// Checks if the string contains any wide segments (characters wider than 1 column).
    fn contains_wide_segments(&self) -> ContainsWideSegments;

    /// Truncate at the end to fit within the given column width.
    fn trunc_end_to_fit(&self, col_width: impl Into<ColWidth>) -> &str;

    /// Truncate at the end by the given column width.
    fn trunc_end_by(&self, col_width: impl Into<ColWidth>) -> &str;

    /// Truncate at the start by the given column width.
    fn trunc_start_by(&self, col_width: impl Into<ColWidth>) -> &str;

    /// Get a string slice at the given column index.
    fn get_string_at(&self, col_index: impl Into<ColIndex>)
    -> Option<Self::StringResult>;

    /// Get a string slice to the right of the given column index.
    fn get_string_at_right_of(
        &self,
        col_index: impl Into<ColIndex>,
    ) -> Option<Self::StringResult>;

    /// Get a string slice to the left of the given column index.
    fn get_string_at_left_of(
        &self,
        col_index: impl Into<ColIndex>,
    ) -> Option<Self::StringResult>;

    /// Get the string at the end (last segment).
    fn get_string_at_end(&self) -> Option<Self::StringResult>;
}
