// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! This module provides comprehensive color wheel functionality for terminal
//! applications.
//!
//! ## Organization:
//! - `types` - Core data types (`Seed`, `ColorWheelControl`, etc.)
//! - `config` - Configuration types and utilities
//! - `gradients` - ANSI 256 and truecolor gradient generation
//! - `helpers` - Color calculation utilities
//! - `policies` - Text colorization policies
//! - `lolcat` - Lolcat-style colorization API
//! - `impl` - Main `ColorWheel` implementation

// Attach sources.
pub mod color_wheel_config;
pub mod gradients;
pub mod color_wheel_helpers;
pub mod lolcat;
pub mod color_wheel_policies;
pub mod color_wheel_types;

// Private implementation details.
mod color_wheel_impl;

// Re-export everything for backward compatibility.
pub use color_wheel_impl::*;
pub use color_wheel_config::*;
pub use gradients::*;
pub use color_wheel_helpers::*;
pub use lolcat::*;
pub use color_wheel_policies::*;
pub use color_wheel_types::*;
