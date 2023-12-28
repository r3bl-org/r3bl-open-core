/*
 *   Copyright (c) 2023 R3BL LLC
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

use r3bl_ansi_color::Color;

// Colors.
pub const LIZARD_GREEN_COLOR: Color = Color::Rgb(20, 244, 0);
pub const DUSTY_LIGHT_BLUE_COLOR: Color = Color::Rgb(171, 204, 242);
pub const LIGHT_GRAY_COLOR: Color = Color::Rgb(94, 103, 111);
pub const SUCCESS_COLOR: Color = LIZARD_GREEN_COLOR;
pub const FAILED_COLOR: Color = Color::Rgb(200, 1, 1);

// Giti
pub const DELETE_BRANCH: &str = "Yes, delete branch";
pub const DELETE_BRANCHES: &str = "Yes, delete branches";
pub const EXIT: &str = "Exit";
