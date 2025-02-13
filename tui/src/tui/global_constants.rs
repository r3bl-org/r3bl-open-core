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

use strum_macros::AsRefStr;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MinSize {
    Col = 65,
    Row = 11,
}

#[repr(usize)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DefaultSize {
    GlobalDataCacheSize = 1_000_000,
}

#[derive(Debug, Eq, PartialEq, AsRefStr)]
pub enum BorderGlyphCharacter {
    #[strum(to_string = "╮")]
    TopRight,

    #[strum(to_string = "╭")]
    TopLeft,

    #[strum(to_string = "╯")]
    BottomRight,

    #[strum(to_string = "╰")]
    BottomLeft,

    #[strum(to_string = "─")]
    Horizontal,

    #[strum(to_string = "│")]
    Vertical,

    #[strum(to_string = "┤")]
    LineUpDownLeft,

    #[strum(to_string = "├")]
    LineUpDownRight,
}

pub const DEFAULT_CURSOR_CHAR: &str = "▒";
pub const DEFAULT_SYN_HI_FILE_EXT: &str = "md";
