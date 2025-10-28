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

// Skip rustfmt for rest of file.
// https://stackoverflow.com/a/75910283/2085356
#![cfg_attr(rustfmt, rustfmt_skip)]

// Attach sources.
mod color_wheel_config;
mod color_wheel_helpers;
mod color_wheel_policies;
mod color_wheel_types;
mod gradients;
mod lolcat;
mod color_wheel_impl;

// Re-export flat public API.
pub use color_wheel_config::*;
pub use color_wheel_helpers::*;
pub use color_wheel_impl::*;
pub use color_wheel_policies::*;
pub use color_wheel_types::*;
pub use gradients::*;
pub use lolcat::*;
