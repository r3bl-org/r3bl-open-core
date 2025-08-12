// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Gradient generation functionality for color wheels.
//!
//! This module provides gradient generation for different color systems:
//! - `ansi_256` - ANSI 256 color gradients with predefined color palettes
//! - `truecolor` - True color (RGB) gradients with customizable stops
//!
//! Previously these were in separate files in the `color_wheel_core` module,
//! but have been reorganized into this dedicated gradients submodule.

// Attach sources.
pub mod ansi_256;
pub mod truecolor;

// Re-export.
pub use ansi_256::*;
pub use truecolor::*;
