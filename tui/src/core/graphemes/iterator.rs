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

//! Iterator implementation for types implementing grapheme string traits.
//!
//! This module provides iterator functionality for any type that implements the
//! grapheme string data access trait, allowing users to iterate over grapheme
//! cluster segments as string slices. The iterator handles the complexity of
//! grapheme cluster boundaries and provides a clean interface for traversing
//! Unicode text.

use super::{GCStringOwned, GCStringRef, gc_string_common::GCStringData};

/// Generic iterator for any type that implements `GCStringData` trait.
/// This allows both `GCStringOwned` and `GCStringRef` to use the same iterator.
#[derive(Debug)]
pub struct GCStringIterator<'a, T: GCStringData> {
    gc_string: &'a T,
    index: usize,
}

impl<'a, T: GCStringData> GCStringIterator<'a, T> {
    /// Creates a new iterator for any type implementing `GCStringData`.
    #[must_use]
    pub fn new(gc_string: &'a T) -> Self {
        Self {
            gc_string,
            index: 0,
        }
    }
}

impl<'a, T: GCStringData> Iterator for GCStringIterator<'a, T> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(seg) = self.gc_string.get_segment(self.index) {
            self.index += 1;
            // Use the segment to extract the string slice from the underlying data
            let start = seg.start_byte_index.as_usize();
            let end = seg.end_byte_index.as_usize();
            let string_data = self.gc_string.string_data();
            Some(&string_data[start..end])
        } else {
            None
        }
    }
}

/// This implementation allows the [`GCStringOwned`] to be used in a for loop
/// directly.
impl<'a> IntoIterator for &'a GCStringOwned {
    type Item = &'a str;
    type IntoIter = GCStringIterator<'a, GCStringOwned>;

    fn into_iter(self) -> Self::IntoIter { GCStringIterator::new(self) }
}

/// This implementation allows [`GCStringRef`] to be used in a for loop
/// directly, just like `GCStringOwned`.
impl<'a> IntoIterator for &'a GCStringRef<'a> {
    type Item = &'a str;
    type IntoIter = GCStringIterator<'a, GCStringRef<'a>>;

    fn into_iter(self) -> Self::IntoIter { GCStringIterator::new(self) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iterator() {
        let gc_string = GCStringOwned::new("Hello, 世界🥞👨‍👩‍👧‍👦🙏🏽");
        let mut iter = gc_string.iter();

        assert_eq!(iter.next(), Some("H"));
        assert_eq!(iter.next(), Some("e"));
        assert_eq!(iter.next(), Some("l"));
        assert_eq!(iter.next(), Some("l"));
        assert_eq!(iter.next(), Some("o"));
        assert_eq!(iter.next(), Some(","));
        assert_eq!(iter.next(), Some(" "));
        assert_eq!(iter.next(), Some("世"));
        assert_eq!(iter.next(), Some("界"));
        assert_eq!(iter.next(), Some("🥞"));
        assert_eq!(iter.next(), Some("👨‍👩‍👧‍👦"));
        assert_eq!(iter.next(), Some("🙏🏽"));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_into_iterator_implementation() {
        let gc_string = GCStringOwned::new("Hello, 世界🥞");

        // Test that we can use the GCStringOwned directly in a for loop (this is why
        // IntoIterator is needed!)
        let mut collected = Vec::new();
        for segment in &gc_string {
            collected.push(segment.to_string());
        }

        assert_eq!(collected.len(), 10);
        assert_eq!(collected[0], "H");
        assert_eq!(collected[1], "e");
        assert_eq!(collected[6], " ");
        assert_eq!(collected[7], "世");
        assert_eq!(collected[8], "界");
        assert_eq!(collected[9], "🥞");

        // Test using for loop with explicit into_iter() call
        let mut explicit_collected = Vec::new();
        for segment in &gc_string {
            explicit_collected.push(segment.to_string());
        }
        assert_eq!(collected, explicit_collected);

        // Test using for loop to find specific graphemes
        let mut found_emoji = false;
        for segment in &gc_string {
            if segment == "🥞" {
                found_emoji = true;
                break;
            }
        }
        assert!(found_emoji);

        // Test using for loop with enumerate to get indices
        for (index, segment) in (&gc_string).into_iter().enumerate() {
            match index {
                0 => assert_eq!(segment, "H"),
                1 => assert_eq!(segment, "e"),
                7 => assert_eq!(segment, "世"),
                8 => assert_eq!(segment, "界"),
                9 => assert_eq!(segment, "🥞"),
                _ => {} // Other segments are valid too
            }
        }

        // Test using for loop to count specific types of characters
        let mut ascii_count = 0;
        let mut unicode_count = 0;
        for segment in &gc_string {
            if segment.is_ascii() {
                ascii_count += 1;
            } else {
                unicode_count += 1;
            }
        }
        assert_eq!(ascii_count, 7); // "H", "e", "l", "l", "o", ",", " "
        assert_eq!(unicode_count, 3); // "世", "界", "🥞"

        // Compare with manual iter() usage (without for loop)
        let iter_results: Vec<_> = gc_string.iter().map(ToString::to_string).collect();
        assert_eq!(iter_results, collected);
    }

    #[test]
    fn test_gcstring_ref_iterator_compatibility() {
        let gc_ref = GCStringRef::new("Hello, 世界🥞👨‍👩‍👧‍👦🙏🏽");
        let gc_owned = GCStringOwned::new("Hello, 世界🥞👨‍👩‍👧‍👦🙏🏽");

        // Both should produce the same iteration results
        let ref_results: Vec<_> = (&gc_ref).into_iter().collect();
        let owned_results: Vec<_> = (&gc_owned).into_iter().collect();

        assert_eq!(ref_results, owned_results);
        assert_eq!(ref_results.len(), 12);
        assert_eq!(ref_results[0], "H");
        assert_eq!(ref_results[7], "世");
        assert_eq!(ref_results[8], "界");
        assert_eq!(ref_results[9], "🥞");
        assert_eq!(ref_results[10], "👨‍👩‍👧‍👦");
        assert_eq!(ref_results[11], "🙏🏽");

        // Test using for loops with both types
        let mut ref_collected = Vec::new();
        for segment in &gc_ref {
            ref_collected.push(segment.to_string());
        }

        let mut owned_collected = Vec::new();
        for segment in &gc_owned {
            owned_collected.push(segment.to_string());
        }

        assert_eq!(ref_collected, owned_collected);
    }

    #[test]
    fn test_generic_iterator_with_both_types() {
        let text = "Hello, 🙏🏽 World!";

        let gc_owned = GCStringOwned::new(text);
        let gc_ref = GCStringRef::new(text);

        // Both can use .iter() method
        let owned_segments: Vec<_> = gc_owned.iter().collect();
        let ref_segments: Vec<_> = gc_ref.iter().collect();

        // Both can be used in for loops
        let mut owned_for_loop: Vec<_> = Vec::new();
        for segment in &gc_owned {
            owned_for_loop.push(segment);
        }

        let mut ref_for_loop: Vec<_> = Vec::new();
        for segment in &gc_ref {
            ref_for_loop.push(segment);
        }

        // All methods should produce identical results
        assert_eq!(owned_segments, ref_segments);
        assert_eq!(owned_segments, owned_for_loop);
        assert_eq!(owned_segments, ref_for_loop);

        // Verify some specific segments
        assert_eq!(owned_segments[0], "H");
        assert_eq!(owned_segments[7], "🙏🏽");
        assert_eq!(owned_segments[9], "W");
    }
}
