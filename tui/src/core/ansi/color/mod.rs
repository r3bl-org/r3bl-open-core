// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Core ANSI color types and conversions.
//!
//! This module provides:
//! - **Primitive types**: [`RgbValue`] (24-bit truecolor) and [`AnsiValue`] (256-color
//!   palette)
//! - **Abstraction**: [`TransformColor`] trait for color representation conversion
//! - **Wrapper type**: [`TuiColor`] for higher-level color handling in the crate
//! - **Conversion logic**: RGB↔ANSI256 conversion and color space utilities
//!
//! Both types implement the [`TransformColor`] trait to enable conversion between
//! different color representations.
//!
//! [`TransformColor`]: crate::TransformColor
//! [`TuiColor`]: crate::TuiColor

// Attach.
mod ansi_value;
mod convert;
mod rgb_value;
mod transform_color;

// Re-export.
pub use ansi_value::*;
pub use convert::*;
pub use rgb_value::*;
pub use transform_color::*;
