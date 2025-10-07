// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::fmt::{Formatter, Result};

/// High-performance string building for complex types that avoids formatter overhead.
///
/// # How to Implement
///
/// Implement this trait for any type that needs fast [`Display`] formatting:
///
/// 1. **Implement [`write_to_buf()`]** - Build your string using [`push_str`] on the buffer:
///
///    ```rust
///    # use r3bl_tui::{FastStringify, BufTextStorage};
///    # use std::fmt::{Result, Write};
///    # struct MyType { value: i32 }
///    impl FastStringify for MyType {
///        fn write_to_buf(&self, acc: &mut BufTextStorage) -> Result {
///            acc.push_str("MyType { value: ");
///            write!(acc, "{}", self.value)?;  // Use write! only when formatting needed
///            acc.push_str(" }");
///            Ok(())
///        }
///    }
///    ```
///
/// 2. **Implement [`Display`]** - Call [`write_to_buf()`] then [`write_buf_to_fmt()`]:
///
///    ```rust
///    # use r3bl_tui::{FastStringify, BufTextStorage};
///    # use std::fmt::{Display, Formatter, Result, Write};
///    # struct MyType { value: i32 }
///    # impl FastStringify for MyType {
///    #     fn write_to_buf(&self, acc: &mut BufTextStorage) -> Result {
///    #         acc.push_str("MyType { value: ");
///    #         write!(acc, "{}", self.value)?;
///    #         acc.push_str(" }");
///    #         Ok(())
///    #     }
///    # }
///    impl Display for MyType {
///        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
///            let mut buffer = BufTextStorage::new();
///            self.write_to_buf(&mut buffer)?;      // Build string in buffer
///            self.write_buf_to_fmt(&buffer, f)     // Single write to formatter
///        }
///    }
///    ```
/// # Why Use This?
///
/// The standard [`Display`] trait using [`Formatter`] has significant overhead for types
/// that build strings from many pieces. Each [`write!`] call goes through the formatter's
/// state machine, checking flags and doing bounds checks. [`FastStringify`] batches all
/// content into a single buffer, then makes ONE write to the formatter using
/// [`write_str`], reducing overhead from ~16% to ~5-8% in performance profiles.
///
/// # Performance Hierarchy (fastest to slowest)
///
/// 1. **Direct `push_str()`** - The absolute fastest when you have a `&str`
///
///    ```rust
///    # let mut buffer = String::new();
///    buffer.push_str("Hello, world!");  // Zero overhead, direct memory copy
///    ```
///
/// 2. **[`FastStringify`] trait** - Fast batched writing for complex types. See
///    [implementation example] above.
///
/// 3. **[`write!`] with [`FormatArgs`]** - Slowest due to formatting overhead
///
///    ```rust
///    # use std::fmt::Write;
///    # let mut buffer = String::new();
///    # let value = 42;
///    // Goes through formatter state machine
///    write!(buffer, "Value: {}", value)?;
///    # Ok::<(), std::fmt::Error>(())
///    ```
///
/// [implementation example]: #how-to-implement
/// [`Display`]: std::fmt::Display
/// [`Formatter`]: std::fmt::Formatter
/// [`write!`]: std::write
/// [`write_str`]: std::fmt::Formatter::write_str
/// [`FormatArgs`]: std::fmt::Arguments
/// [`push_str`]: String::push_str
/// [`write_to_buf()`]: FastStringify::write_to_buf
/// [`write_buf_to_fmt()`]: FastStringify::write_buf_to_fmt
#[rustfmt::skip]
pub trait FastStringify {
    /// Write the formatted representation to the buffer. Use [`push_str`] for strings,
    /// [`write!`] only when formatting is needed.
    ///
    /// [`push_str`]: String::push_str
    /// [`write!`]: std::write
    fn write_to_buf(&self, acc: &mut BufTextStorage) -> Result;

    /// Write the buffer to formatter. Call from [`Display::fmt`] after
    /// [`write_to_buf`].
    ///
    /// [`Display::fmt`]: std::fmt::Display::fmt
    /// [`write_to_buf`]: FastStringify::write_to_buf
    fn write_buf_to_fmt(&self, acc: &BufTextStorage, f: &mut Formatter<'_>) -> Result {
        f.write_str(acc)
    }
}

/// Buffer for building text efficiently.
///
/// We use [`String`] as the backing storage after performance testing showed:
/// - `SmallString<[u8; 64]>` had slightly worse performance due to stack allocation
///   overhead.
/// - `SmallString<[u8; 256]>` had even worse performance for small strings.
/// - Plain [`String`] provides the best balance of performance across all test cases.
///
/// This type alias allows us to easily experiment with different string-like data
/// structures in the future (e.g., `SmallString`, [`String`], custom implementations)
/// without impacting the rest of the codebase.
///
/// [`String`]: std::string::String
pub type BufTextStorage = String;
