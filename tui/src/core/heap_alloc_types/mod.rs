// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Heap-allocated collection types for scenarios requiring minimal stack footprint.
//!
//! # Why Heap Allocation?
//!
//! While heap allocation has overhead, certain scenarios benefit from heap storage:
//! - **Deep recursion**: Recursive parsers can create 300+ stack frames
//! - **Stack safety**: Prevents overflow when call stacks get deep
//! - **Minimal stack footprint**: Only pointer + metadata on stack (24 bytes for Vec)
//!
//! # Types in this Module
//!
//! ## [`ParseList`] - Stack-safe parsing
//!
//! Uses [Vec] for absolute stack safety with recursive parsers. Only 24 bytes on stack
//! regardless of content size, preventing stack overflow in deeply nested parsing
//! operations.
//!
//! # See Also
//!
//! - [`crate::stack_alloc_types`] - Stack-allocated types for performance-critical paths

pub mod parse_list;

// Re-export.
pub use parse_list::*;
