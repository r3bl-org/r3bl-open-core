// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # Performance-optimized string building with `WriteToBuf`
//!
//! This module provides the `WriteToBuf` trait for high-performance string building
//! that avoids the overhead of Rust's formatting machinery.
//!
//! ## Performance hierarchy (fastest to slowest)
//!
//! 1. **Direct `push_str()`** - The absolute fastest when you have a `&str` ```rust # let
//!    mut buffer = String::new(); buffer.push_str("Hello, world!");  // Zero overhead,
//!    direct memory copy ```
//!
//! 2. **`WriteToBuf` trait** - Fast batched writing for complex types ```rust # use
//!    r3bl_tui::WriteToBuf; # struct MyType; # impl WriteToBuf for MyType { #     fn
//!    write_to_buf(&self, acc: &mut String) -> std::fmt::Result { # acc.push_str("batched
//!    content"); #         Ok(()) #     } # } # let my_value = MyType; # let mut buffer =
//!    String::new(); my_value.write_to_buf(&mut buffer)?;  // Batches all writes, one
//!    final formatter call # Ok::<(), std::fmt::Error>(()) ```
//!
//! 3. **`write!` with `FormatArgs`** - Slowest due to formatting overhead ```rust # use
//!    std::fmt::Write; # let mut buffer = String::new(); # let value = 42; write!(buffer,
//!    "Value: {}", value)?;  // Goes through formatter state machine # Ok::<(),
//!    std::fmt::Error>(()) ```
//!
//! ## Why `WriteToBuf` is faster than `Display`
//!
//! Even when using `WriteToBuf`, if you eventually need to format values with `write!`,
//! you still incur the `FormatArgs` overhead. However, `WriteToBuf` is faster because:
//!
//! 1. **Batching**: All content is accumulated in a single buffer
//! 2. **Single formatter call**: Only one `write_str()` call to the formatter
//! 3. **No repeated overhead**: Avoids multiple trips through the formatter state machine
//!
//! ## When to use each approach
//!
//! - **Use `push_str()`** when you have literal strings or `&str` values with no
//!   formatting
//! - **Use `WriteToBuf`** when implementing `Display` for complex types that build
//!   strings
//! - **Use `write!`** only when you actually need formatting capabilities
//!
//! ## Example: Optimal `Display` implementation
//!
//! ```rust
//! # use std::fmt::{Display, Formatter, Result};
//! # use r3bl_tui::{WriteToBuf, BufTextStorage};
//! struct MyComplexType {
//!     name: String,
//!     count: usize,
//! }
//!
//! impl WriteToBuf for MyComplexType {
//!     fn write_to_buf(&self, acc: &mut BufTextStorage) -> Result {
//!         // Use push_str for literals and strings - fastest!
//!         acc.push_str("MyComplexType { name: \"");
//!         acc.push_str(&self.name);
//!         acc.push_str("\", count: ");
//!         // Only use formatting when necessary
//!         use std::fmt::Write;
//!         write!(acc, "{}", self.count)?;
//!         acc.push_str(" }");
//!         Ok(())
//!     }
//! }
//!
//! impl Display for MyComplexType {
//!     fn fmt(&self, f: &mut Formatter<'_>) -> Result {
//!         let mut buffer = BufTextStorage::new();
//!         self.write_to_buf(&mut buffer)?;
//!         // Single write to formatter - minimizes overhead
//!         self.write_buf_to_fmt(&buffer, f)
//!     }
//! }
//! ```

use std::fmt::{Formatter, Result};

/// Buffer for building text efficiently.
///
/// We use `String` as the backing storage after performance testing showed:
/// - `SmallString<[u8; 64]>` had slightly worse performance due to stack allocation
///   overhead.
/// - `SmallString<[u8; 256]>` had even worse performance for small strings.
/// - Plain `String` provides the best balance of performance across all test cases.
///
/// This type alias allows us to easily experiment with different string-like data
/// structures in the future (e.g., `SmallString`, `String`, custom implementations)
/// without impacting the rest of the codebase.
pub type BufTextStorage = String;

/// Trait for efficiently writing text to a buffer.
///
/// ## Why `WriteToBuf` instead of Display/Formatter?
///
/// The standard [`std::fmt::Display`] trait uses [`std::fmt::Formatter`] which has
/// significant overhead:
/// 1. **Formatter State Machine**: Each [`write!`] call goes through the formatter's
///    internal state machine, checking formatting flags (alignment, padding, precision,
///    etc).
/// 2. **Multiple Function Calls**: Each [`write!`] has method call overhead, vtable
///    lookups for trait objects, and repeated bounds checking.
/// 3. **Buffer Management**: The formatter may need to reallocate its internal buffer
///    multiple times for many small writes.
///
/// By using `WriteToBuf` with a [`BufTextStorage`] buffer, we:
/// - Make direct string concatenations without formatter overhead.
/// - Batch all content into a single buffer.
/// - Make only ONE write to the formatter in the Display implementation using
///   [`core::fmt::Formatter::write_str`].
/// - Reduce the overhead from ~16% to ~5-8% in performance profiles.
///
/// The [`std::fmt::Display`] trait implementations still exist for API compatibility but
/// delegate to `WriteToBuf`. The [`std::fmt::Display`] trait will have to use use
/// [`core::fmt::Formatter::write_str`] to actually write the `acc` buffer.
pub trait WriteToBuf {
    /// Write the formatted representation to the provided buffer. You might want to
    /// call [`WriteToBuf::write_buf_to_fmt()`] when you are ready to actually write the
    /// buffer to the formatter if you are implementing the [`std::fmt::Display`] trait.
    ///
    /// # Errors
    ///
    /// Returns an error if the formatting operation fails.
    fn write_to_buf(&self, acc: &mut BufTextStorage) -> Result;

    /// Use [`core::fmt::Formatter::write_str`] to actually write the `acc` buffer when
    /// implementing the [`std::fmt::Display`] trait.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the formatter fails.
    fn write_buf_to_fmt(&self, acc: &BufTextStorage, f: &mut Formatter<'_>) -> Result {
        f.write_str(acc)
    }
}
