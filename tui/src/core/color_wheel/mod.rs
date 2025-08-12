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
pub mod config;
pub mod gradients;
pub mod helpers;
pub mod lolcat;
pub mod policies;
pub mod types;

// Private implementation details.
mod color_wheel_impl;

// Re-export everything for backward compatibility.
pub use color_wheel_impl::*;
pub use config::*;
pub use gradients::*;
pub use helpers::*;
pub use lolcat::*;
pub use policies::*;
pub use types::*;
