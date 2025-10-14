// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! The fundamental primitive type for all coordinate systems.
//!
//! This module provides the single primitive building block used throughout the
//! coordinates module:
//!
//! - [`ChUnit`]: The fundamental character unit type (wraps [`prim@u16`])
//!
//! All other coordinate types (indices, lengths, positions, sizes) are built on top of
//! this primitive and live in the [`buffer_coords`](super::buffer_coords) module.

// Attach source file.
pub mod ch_unit;

// Re-export.
pub use ch_unit::*;
