// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Segment-related types and utilities.
//!
//! This module contains the core types for representing grapheme cluster segments
//! and utilities for building and manipulating them.
//!
//! See the [module docs](crate::graphemes) for
//! comprehensive information about Unicode handling, grapheme clusters, and the three
//! types of indices used in this system.

pub mod seg;
pub mod seg_index;
pub mod seg_length;
pub mod segment_builder;

pub use seg::*;
pub use seg_index::*;
pub use seg_length::*;
pub use segment_builder::*;
