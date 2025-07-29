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

//! Iterator implementation for `GCStringOwned` type.
//!
//! This module provides iterator functionality for the `GCStringOwned` struct, allowing
//! users to iterate over grapheme cluster segments as string slices. The iterator handles
//! the complexity of grapheme cluster boundaries and provides a clean interface for
//! traversing Unicode text.

use super::GCStringOwned;

#[derive(Debug)]
pub struct GCStringIterator<'a> {
    gc_string: &'a GCStringOwned,
    index: usize,
}

impl<'a> GCStringIterator<'a> {
    /// Creates a new iterator for the given `GCStringOwned`.
    #[must_use]
    pub fn new(gc_string: &'a GCStringOwned) -> Self {
        Self {
            gc_string,
            index: 0,
        }
    }
}

impl<'a> Iterator for GCStringIterator<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        match self.gc_string.get_segment(self.index) {
            Some(segment) => {
                self.index += 1;
                Some(segment)
            }
            None => None, // Stop iteration when `get_segment` returns `None`.
        }
    }
}

/// This implementation allows the [`GCStringOwned`] to be used in a for loop
/// directly.
impl<'a> IntoIterator for &'a GCStringOwned {
    type Item = &'a str;
    type IntoIter = GCStringIterator<'a>;

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
}
