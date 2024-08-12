/*
 *   Copyright (c) 2024 R3BL LLC
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

use std::collections::{hash_map::Entry, HashMap};

use sha2::{Digest, Sha256};
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StringLength {
    StripAnsi,
    Unicode,
}

pub type MemoizedLenMap = HashMap<String, u16>;

mod to_from_string_impl {
    use super::*;

    impl std::str::FromStr for StringLength {
        type Err = String;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                "strip_ansi" => Ok(Self::StripAnsi),
                "unicode" => Ok(Self::Unicode),
                _ => Err(format!("Invalid StringLength variant: {}", s)),
            }
        }
    }

    impl std::fmt::Display for StringLength {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::StripAnsi => write!(f, "strip_ansi"),
                Self::Unicode => write!(f, "unicode"),
            }
        }
    }
}

impl StringLength {
    /// If the input can't be found in the memoized map, calculate the length and store
    /// it. Otherwise return the stored length.
    ///
    /// # Memoization
    ///
    /// The key is the [String] that needs to be measured using the variants. The value is
    /// the length.
    ///
    /// # Speedup, even for small strings
    ///
    /// | Variant                   | Cached | Speedup |
    /// |---------------------------|--------|---------|
    /// | [StringLength::Unicode]   | No     | None    |
    /// | [StringLength::StripAnsi] | Yes    | 70x     |
    ///
    /* cspell:disable-next-line  */
    /// Eg: For input: `"\u{1b}[31mfoo\u{1b}[0m";`
    /// - the uncached time is 700µs
    /// - the cached time is 10µs
    ///
    pub fn calculate(&self, input: &str, memoized_len_map: &mut MemoizedLenMap) -> u16 {
        match self {
            // Do not memoize (slower to do this).
            StringLength::Unicode => UnicodeWidthStr::width(input) as u16,

            // Memoize (faster to do this).
            StringLength::StripAnsi => match memoized_len_map.entry(input.to_string()) {
                Entry::Occupied(entry) => *entry.get(),
                Entry::Vacant(entry) => {
                    let stripped_input = strip_ansi::strip_ansi(input);
                    let stripped_input: &str = stripped_input.as_ref();
                    let length = UnicodeWidthStr::width(stripped_input) as u16;
                    entry.insert(length);
                    length
                }
            },
        }
    }

    /// SHA256 produces a 256-bit (32-byte) hash value, typically rendered as a hexadecimal
    /// number. However, here we are converting it to a u32.
    pub fn calculate_sha256(text: &str) -> u32 {
        let mut hasher = Sha256::new();
        hasher.update(text);
        let result = hasher.finalize();
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(&result.as_slice()[..4]);
        u32::from_le_bytes(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timed;

    #[test]
    fn test_strip_ansi_esc_seq_len_cache_speedup() {
        /* cspell: disable-next-line */
        let input = "\u{1b}[31mfoo\u{1b}[0m";
        let memoized_len_map = &mut MemoizedLenMap::new();

        assert!(!memoized_len_map.contains_key(input));

        let (_, duration_uncached) = timed!({
            let len = StringLength::StripAnsi.calculate(input, memoized_len_map);
            assert_eq!(len, 3);
            assert!(memoized_len_map.contains_key(input));
        });
        println!(
            "Execution time - U string_length(StripAnsi): {:?}",
            duration_uncached
        );

        let (_, duration_cached) = timed!({
            let len = StringLength::StripAnsi.calculate(input, memoized_len_map);
            assert_eq!(len, 3);
            assert!(memoized_len_map.contains_key(input));
        });
        println!(
            "Execution time - C string_length(StripAnsi): {:?}",
            duration_cached
        );
    }

    #[test]
    fn test_unicode_string_len_no_cache() {
        let input = "foo";
        let memoized_len_map = &mut MemoizedLenMap::new();

        assert!(!memoized_len_map.contains_key(input));

        let (_, duration_uncached) = timed!({
            let len = StringLength::Unicode.calculate(input, memoized_len_map);
            assert_eq!(len, 3);
            assert!(!memoized_len_map.contains_key(input));
        });
        println!(
            "Execution time - U string_length(Unicode): {:?}",
            duration_uncached
        );
    }
}
