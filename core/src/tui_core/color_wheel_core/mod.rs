/*
 *   Copyright (c) 2024 R3BL LLC
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

// Attach sources.
pub mod ansi_256_color_gradients;
pub mod color_wheel_control;
pub mod color_utils;
pub mod truecolor_gradient;
pub mod converter;
pub mod policies;
pub mod defaults;

// Re-export.
pub use ansi_256_color_gradients::*;
pub use color_wheel_control::*;
pub use color_utils::*;
pub use truecolor_gradient::*;
pub use converter::*;
pub use policies::*;
pub use defaults::*;