// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # String Allocation Performance Strategy
//!
//! This codebase is heavily optimized for zero-allocation rendering and formatting.
//! Depending on the specific use case, we use one of three core approaches for building
//! strings.
//!
//! ## Performance Hierarchy (fastest to slowest for standard formatting)
//!
//! 1. Direct [`push_str()`] - Zero overhead, direct memory copy.
//! 2. [`format_no_alloc!`] - Zero heap allocations, reuses existing heap.
//! 3. [`FastStringify`] - Temporary heap allocation, bypasses formatter state machine.
//! 4. [`inline_string!`] - Stack allocated, but spills to heap if > [`16`] bytes.
//! 5. [`format!`] / [`write!`] - Double whammy - always heap allocates (for [`format!`])
//!    and/or uses heavy formatter overhead (for [`write!`]).
//!
//! ## 1. [`FastStringify`] Trait (For High-Throughput [`ANSI`] & Custom Types)
//!
//! The standard [`Display`] trait using [`Formatter`] has significant overhead. Each
//! [`write!`] macro call expands to [`write_fmt`], which spins up a state machine to
//! parse arguments, check flags, and handle dynamic dispatch.
//!
//! [`FastStringify`] batches all content into a single buffer, then makes ONE
//! [`write_str()`] call. Unlike [`write!`], [`write_str`] completely bypasses the
//! formatting engine and copies the bytes directly.
//!
//! - Tradeoff: It actually introduces a short-lived heap allocation to build the batched
//!   string, but bypasses the expensive [`Formatter`] state machine. For complex types
//!   serialized millions of times (like [`ANSI`] codes), this is significantly faster
//!   than standard [`Display`].
//!
//! ## 2. [`inline_string!`] Macro (For the Stack / Editor Engine)
//!
//! [`inline_string!`] writes formatted text directly into a [`InlineString`] allocated on
//! the stack ([`16`] bytes). It acts as a drop-in replacement for [`format!`].
//!
//! - Use Case: The primary use case is the core [Editor Component], which processes
//!   thousands of individual characters or short grapheme clusters (1-4 bytes) per frame.
//!   It is also highly recommended for `tracing::*!` macros when logging small variables.
//! - Warning: If the formatted text exceeds [`16`] bytes, it will dynamically spill to
//!   the heap, incurring the same cost as [`format!`].
//!
//! ## 3. [`format_no_alloc!`] Macro (For Hot Loops)
//!
//! When you need to build dynamic strings in a hot loop (like a 60 FPS event loop) that
//! are guaranteed to exceed [`16`] bytes, [`inline_string!`] will spill. Instead, use
//! [`format_no_alloc!`].
//!
//! - Use Case: You hoist a [`String::with_capacity(N)`] outside the loop, and pass it to
//!   the macro inside the loop. It clears the buffer and writes into the existing heap
//!   capacity, ensuring absolutely zero heap allocations per tick.
//!
//! [`16`]: crate::core::DEFAULT_STRING_STORAGE_SIZE
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`Display`]: std::fmt::Display
//! [`FastStringify`]: crate::FastStringify
//! [`format!`]: std::format
//! [`format_no_alloc!`]: macro@crate::format_no_alloc
//! [`Formatter`]: std::fmt::Formatter
//! [`inline_string!`]: crate::inline_string
//! [`InlineString`]: crate::InlineString
//! [`push_str()`]: String::push_str
//! [`String::with_capacity(N)`]: String::with_capacity
//! [`write!`]: std::write
//! [`write_fmt`]: std::fmt::Formatter::write_fmt
//! [`write_str()`]: std::fmt::Formatter::write_str
//! [`write_str`]: std::fmt::Formatter::write_str
//! [Editor Component]: crate::EditorComponent

pub mod fast_stringify;
pub mod format_no_alloc;

pub use fast_stringify::*;
pub use format_no_alloc::*;
