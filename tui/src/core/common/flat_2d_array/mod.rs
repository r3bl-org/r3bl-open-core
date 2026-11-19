// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! High-performance, [SIMD]-friendly 2D array abstraction. See [`Flat2DArray`] struct for
//! details.
//!
//! [SIMD]: https://en.wikipedia.org/wiki/SIMD

// Attach.
mod array_1d_simd_access;
mod array_2d_access;
mod core;

#[cfg(test)]
mod benches;

// Re-export.
pub use core::*;
