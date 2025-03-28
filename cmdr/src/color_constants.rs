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

use r3bl_core::ASTColor;

pub enum DefaultColors {
    LizardGreen,
    SlateGrey,
    SilverMetallic,
    FrozenBlue,
    MoonlightBlue,
    NightBlue,
    GuardsRed,
    Orange,
}

impl DefaultColors {
    pub fn as_ansi_color(&self) -> ASTColor {
        match self {
            DefaultColors::LizardGreen => ASTColor::Rgb(20, 244, 0),
            DefaultColors::SlateGrey => ASTColor::Rgb(94, 103, 111),
            DefaultColors::SilverMetallic => ASTColor::Rgb(213, 217, 220),
            DefaultColors::FrozenBlue => ASTColor::Rgb(171, 204, 242),
            DefaultColors::MoonlightBlue => ASTColor::Rgb(31, 36, 46),
            DefaultColors::NightBlue => ASTColor::Rgb(14, 17, 23),
            DefaultColors::GuardsRed => ASTColor::Rgb(200, 1, 1),
            DefaultColors::Orange => ASTColor::Rgb(255, 132, 18),
        }
    }
}
