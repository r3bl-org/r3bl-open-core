// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Trait for high-performance string building for complex types. See [`FastStringify`]
//! and [`BufTextStorage`] for details.

use std::fmt::{Display, Formatter, Result};

/// High-performance string building for complex types that avoids formatter overhead.
///
/// This trait requires [`Display`] as a supertrait, meaning any type implementing
/// [`FastStringify`] **must also implement** [`Display`]. This allows the trait to
/// provide optimized string building while maintaining compatibility with Rust's
/// standard formatting infrastructure.
///
/// # How to Implement
///
/// Both [`FastStringify`] and [`Display`] must be implemented. Follow these steps:
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
/// 2. **Implement [`Display`] (required)** - Call [`write_to_buf()`] then [`write_buf_to_fmt()`]:
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
/// [`Display`]: Display
/// [`Formatter`]: std::fmt::Formatter
/// [`write!`]: std::write
/// [`write_str`]: std::fmt::Formatter::write_str
/// [`FormatArgs`]: std::fmt::Arguments
/// [`push_str`]: String::push_str
/// [`write_to_buf()`]: FastStringify::write_to_buf
/// [`write_buf_to_fmt()`]: FastStringify::write_buf_to_fmt
#[rustfmt::skip]
pub trait FastStringify: Display {
    /// Write the formatted representation to the buffer. Use [`push_str`] for strings,
    /// [`write!`] only when formatting is needed.
    ///
    /// # Errors
    /// Returns an error if writing to the buffer fails (formatting error).
    ///
    /// [`push_str`]: String::push_str
    /// [`write!`]: std::write
    fn write_to_buf(&self, acc: &mut BufTextStorage) -> Result;

    /// Write the buffer to formatter. Call from [`Display::fmt`] after
    /// [`write_to_buf`].
    ///
    /// # Errors
    /// Returns an error if writing to the formatter fails (formatting error).
    ///
    /// [`Display::fmt`]: Display::fmt
    /// [`write_to_buf`]: FastStringify::write_to_buf
    fn write_buf_to_fmt(&self, acc: &BufTextStorage, f: &mut Formatter<'_>) -> Result {
        f.write_str(acc)
    }
}

/// Buffer for building text efficiently.
///
/// We use [`String`] as the backing storage after performance testing showed:
/// - [`SmallString<[u8; 64]>`] had slightly worse performance due to stack allocation
///   overhead.
/// - [`SmallString<[u8; 256]>`] had even worse performance for small strings.
/// - Plain [`String`] provides the best balance of performance across all test cases.
///
/// # Why [`String`] Wins for Short-Lived Buffers
///
/// This buffer is created fresh in every [`Display::fmt`] call and immediately dropped
/// after use, making it extremely short-lived. For this usage pattern, [`String`]
/// outperforms `SmallString` because:
///
/// ## Stack Allocation Cost
///
/// - [`SmallString<[u8; 256]>`] allocates 256 bytes on the stack **every time**,
///   regardless of actual usage
/// - This touches multiple CPU cache lines (typically 64 bytes each) just to set up the
///   buffer
/// - [`String`] only allocates 24 bytes on the stack (pointer, length, capacity), with
///   actual content on the heap
///
/// ## Unpredictable Size Problem
///
/// Types implementing [`FastStringify`] have unpredictable output sizes:
/// - ANSI styled text: `"\x1b[38;5;42mHello\x1b[0m"` ≈ 20 bytes
/// - Complex types: could be 10 bytes or 500 bytes
/// - With [`SmallString<[u8; 64]>`]: waste stack space for small strings, pay both stack
///   AND heap for large strings
/// - With [`String`]: only pay for what you actually use
///
/// ## Modern Allocator Efficiency
///
/// For short-lived heap allocations, modern allocators are highly optimized:
/// - Thread-local caches make allocation often just a pointer bump
/// - Deallocation returns memory to thread-local free list for immediate reuse
/// - The next [`Display::fmt`] call likely reuses the same heap block
///
/// ## Example Comparison
///
/// ```text
/// SmallString<[u8; 256]> approach:
/// - Stack: 256 bytes allocated upfront
/// - Typical usage: 20 bytes
/// - Wasted: 236 bytes per call
///
/// String approach:
/// - Stack: 24 bytes (ptr, len, cap)
/// - Heap: ~32 bytes (only when needed)
/// - Wasted: ~12 bytes on heap (better locality)
/// ```
///
/// ## When SmallString Would Win
///
/// `SmallString` would be better if the buffer was:
/// - **Long-lived**: Amortizes stack allocation over many operations
/// - **Predictable size**: 90%+ of strings fit in inline capacity
/// - **Hot loop**: Allocator pressure becomes a bottleneck
///
/// But for this use case (short-lived, unpredictable size), [`String`] is optimal.
///
/// This type alias allows us to easily experiment with different string-like data
/// structures in the future without impacting the rest of the codebase.
///
/// [`String`]: std::string::String
/// [`Display::fmt`]: Display::fmt
/// [`SmallString<[u8; 64]>`]: smallstr::SmallString
/// [`SmallString<[u8; 256]>`]: smallstr::SmallString
pub type BufTextStorage = String;
