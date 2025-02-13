/*
 *   Copyright (c) 2024-2025 R3BL LLC
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
use crate::{Ansi256Color, TransformColor, convert_rgb_into_ansi256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl TransformColor for RgbColor {
    fn as_rgb(&self) -> RgbColor { *self }

    fn as_ansi256(&self) -> Ansi256Color { convert_rgb_into_ansi256(*self) }

    fn as_grayscale(&self) -> Ansi256Color {
        convert_rgb_into_ansi256(*self).as_grayscale()
    }
}
