// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! High-performance, [SIMD]-friendly 2D array abstraction backed by a contiguous 1D
//! array.
//!
//! This module provides the [`Flat2DArray`] data structure which is designed for optimal
//! memory layout and cache locality. It is especially useful for rendering pipelines
//! (like terminal offscreen buffers) where ultra-fast bulk updates, rendering, and
//! diffing can natively benefit from auto-vectorized [SIMD] array operations.
//!
//! For comprehensive implementation details and usage patterns, see the [`Flat2DArray`]
//! struct.
//!
//! [SIMD]: https://en.wikipedia.org/wiki/SIMD

// Attach.
pub mod address_translation;
pub mod array_1d_simd_access;
pub mod array_2d_access;
pub mod core;
pub mod range_validation;

#[cfg(test)]
pub mod benches;

// Re-export.
pub use core::*;
