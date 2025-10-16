// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.
use crate::{GCStringOwned, u16};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, hash_map::Entry};

/// Enum representing different methods for calculating the length of a string. The
/// [`Self::calculate`] function memoizes the length of the string for the
/// [`StringLength::StripAnsi`] variant to speed up computations.
///
/// # Variants
///
/// - `StripAnsi`: Calculates the length of the string after stripping ANSI escape
///   sequences.
/// - `Unicode`: Calculates the Unicode width of the string.
///
/// # Example
/// ```
/// use std::collections::HashMap;
/// use r3bl_tui::StringLength;
/* cspell:disable-next-line */
/// let input = "\u{1b}[31mfoo\u{1b}[0m";
/// let mut memoized_len_map = HashMap::new();
///
/// let length = StringLength::StripAnsi.calculate(input, &mut memoized_len_map);
/// assert_eq!(length, 3);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StringLength {
    StripAnsi,
    Unicode,
}

pub type MemoizedLenMap = HashMap<String, u16>;

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
    /// | [`StringLength::Unicode`]   | No     | None    |
    /// | [`StringLength::StripAnsi`] | Yes    | 70x     |
    /* cspell:disable-next-line */
    /// Eg: For input: `"\u{1b}[31mfoo\u{1b}[0m";` on a 13th Gen Intel® Core™ i5-13600K
    /// machine with 64GB of RAM running Ubuntu 24.04, the execution times are:
    /// - Uncached time is 700µs.
    /// - Cached time is 10µs.
    pub fn calculate(&self, input: &str, memoized_len_map: &mut MemoizedLenMap) -> u16 {
        match self {
            // Do not memoize (slower to do this).
            StringLength::Unicode => u16(*GCStringOwned::from(input).width()),

            // Memoize (faster to do this).
            StringLength::StripAnsi => match memoized_len_map.entry(input.to_string()) {
                Entry::Occupied(entry) => *entry.get(),
                Entry::Vacant(entry) => {
                    let stripped_input = strip_ansi::strip_ansi(input);
                    let stripped_input: &str = stripped_input.as_ref();
                    let length = u16(*GCStringOwned::from(stripped_input).width());
                    entry.insert(length);
                    length
                }
            },
        }
    }

    /// [SHA256](sha2) produces a 256-bit (32-byte) hash value, typically rendered as a
    /// hexadecimal number. However, here we are converting it to a u32. Here's an example
    /// of how long it takes to run on `foo`: 25.695µs. To provide some perspective of how
    /// long this is, it takes about the same time to run [`StringLength::Unicode`] on the
    /// same input, on a 13th Gen Intel® Core™ i5-13600K machine with 64GB of RAM running
    /// Ubuntu 24.04.
    #[must_use]
    pub fn calculate_sha256(text: &str) -> u32 {
        let mut hasher = Sha256::new();
        hasher.update(text);
        let result = hasher.finalize();
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(&result[..4]);
        u32::from_le_bytes(bytes)
    }
}

mod to_from_string_impl {
    use super::StringLength;

    impl std::str::FromStr for StringLength {
        type Err = String;

        fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
            match s {
                "strip_ansi" => Ok(Self::StripAnsi),
                "unicode" => Ok(Self::Unicode),
                _ => Err(format!("Invalid StringLength variant: {s}")),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timed;

    #[test]
    fn test_sha256() {
        let input = "foo";
        let (hash, duration) = timed!({
            let hash = StringLength::calculate_sha256(input);
            assert_eq!(hash, 1_806_968_364);
            hash
        });
        println!("Execution time - string_length(Sha256): {duration:?}");
        assert_eq!(hash, 1_806_968_364);
    }

    #[test]
    fn test_strip_ansi_esc_seq_len_cache_speedup() {
        /* cspell:disable-next-line */
        let input = "\u{1b}[31mfoo\u{1b}[0m";
        let memoized_len_map = &mut MemoizedLenMap::new();

        assert!(!memoized_len_map.contains_key(input));

        let ((), duration_uncached) = timed!({
            let len = StringLength::StripAnsi.calculate(input, memoized_len_map);
            assert_eq!(len, 3);
            assert!(memoized_len_map.contains_key(input));
        });
        println!("Execution time - U string_length(StripAnsi): {duration_uncached:?}");

        let ((), duration_cached) = timed!({
            let len = StringLength::StripAnsi.calculate(input, memoized_len_map);
            assert_eq!(len, 3);
            assert!(memoized_len_map.contains_key(input));
        });
        println!("Execution time - C string_length(StripAnsi): {duration_cached:?}");
    }

    #[test]
    fn test_unicode_string_len_no_cache() {
        let input = "foo";
        let memoized_len_map = &mut MemoizedLenMap::new();

        assert!(!memoized_len_map.contains_key(input));

        let ((), duration_uncached) = timed!({
            let len = StringLength::Unicode.calculate(input, memoized_len_map);
            assert_eq!(len, 3);
            assert!(!memoized_len_map.contains_key(input));
        });
        println!("Execution time - U string_length(Unicode): {duration_uncached:?}");
    }
}
