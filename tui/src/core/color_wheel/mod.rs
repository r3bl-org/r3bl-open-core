/*
 *   Copyright (c) 2022-2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

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
