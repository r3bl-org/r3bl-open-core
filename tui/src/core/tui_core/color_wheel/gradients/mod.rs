/*
 *   Copyright (c) 2023-2025 R3BL LLC
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
