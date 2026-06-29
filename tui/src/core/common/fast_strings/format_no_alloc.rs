// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Extension trait and macro to perform zero-allocation formatting into preallocated
//! string. See [`PreallocatedStringExt`] and [`format_no_alloc!`] for details.
//!
//! [`format_no_alloc!`]: macro@crate::format_no_alloc
//! [`PreallocatedStringExt`]: crate::PreallocatedStringExt

#[allow(
    unused_imports,
    reason = "For short import statements in link ref defs"
)]
use crate::core::common::fast_strings;
use std::fmt::{Arguments, Write};

/// Trait to extend [`String`] with an ergonomic method for zero-allocation formatting.
///
/// This provides the underlying mechanics for the [`format_no_alloc!`] macro, but can
/// also be used directly when working manually with [`std::fmt::Arguments`].
///
/// See [`fast_strings`] for the central architecture covering all string performance and
/// allocation strategies in the codebase.
///
/// [`fast_strings`]: mod@fast_strings#string-allocation-performance-strategy
/// [`format_no_alloc!`]: macro@crate::format_no_alloc
pub trait PreallocatedStringExt {
    /// Clears the string, writes the formatted arguments without allocating new heap
    /// memory, and returns the borrowed string slice.
    fn format_into(&mut self, args: Arguments<'_>) -> &str;
}

impl PreallocatedStringExt for String {
    fn format_into(&mut self, args: Arguments<'_>) -> &str {
        self.clear();
        let _unused = Write::write_fmt(self, args);
        self.as_str()
    }
}

/// A highly ergonomic macro that acts as a zero-heap-allocation drop-in replacement for
/// [`format!`] when you are inside a hot loop and already have a pre-allocated [`String`]
/// buffer available to reuse.
///
/// It will clear the given buffer, write the format string directly into the existing
/// heap capacity using [`std::fmt::Write`], and evaluate to a `&str`.
///
/// See [`fast_strings`] for the central architecture covering all string performance and
/// allocation strategies in the codebase.
///
/// # How format args are passed to the trait method
///
/// In the macro code below, we call [`format_args!`]. This generates a return type of
/// [`Arguments`], which is then passed as the second parameter `args` of the call to the
/// [`format_into()`] method of the trait.
///
/// # Example
///
/// ```rust
/// use r3bl_tui::format_no_alloc;
///
/// // Allocate ONCE outside the hot loop
/// let mut title_buf = String::with_capacity(128);
///
/// for i in 0..100 {
///     // Zero heap allocations per tick!
///     let title_ref = format_no_alloc!(title_buf, "Tick count: {i}");
///     // title_ref is a `&str` and title_buf's capacity is reused.
/// }
/// ```
///
/// [`fast_strings`]:
///     mod@crate::core::common::fast_strings#string-allocation-performance-strategy
/// [`format_into()`]: PreallocatedStringExt::format_into
#[macro_export]
macro_rules! format_no_alloc {
    ($buf:expr, $($arg:tt)*) => {{
        use $crate::PreallocatedStringExt;
        $buf.format_into(format_args!($($arg)*))
    }};
}
