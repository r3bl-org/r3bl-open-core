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

//! More info:
//! - [List of all symbols](https://symbl.cc/en/unicode-table/#miscellaneous-technical)
//! - [Box drawing characters](https://symbl.cc/en/unicode/blocks/box-drawing/)
//! - [Block element characters](https://symbl.cc/en/unicode/blocks/block-elements/)
//! - [Geometric shape characters](https://symbl.cc/en/unicode/blocks/geometric-shapes/)
//! - [Arrow characters](https://symbl.cc/en/unicode/blocks/arrows/)
//! - [Supplemental arrow characters-A](https://symbl.cc/en/unicode/blocks/supplemental-arrows-a/)
//! - [Supplemental arrow characters-B](https://symbl.cc/en/unicode/blocks/supplemental-arrows-b/)
//! - [Dingbat characters](https://symbl.cc/en/unicode/blocks/dingbats/)
//! - [Braille pattern characters](https://symbl.cc/en/unicode/blocks/braille-patterns/)
//! - [Miscellaneous symbol and arrow characters](https://symbl.cc/en/unicode/blocks/miscellaneous-symbols-and-arrows/)
//! - [Tifinagh characters](https://symbl.cc/en/unicode/blocks/tifinagh/)
//! - [Ideographic characters](https://symbl.cc/en/unicode/blocks/ideographic-description-characters/)
//! - [Emotions kaomoji](https://symbl.cc/en/kaomoji/)
//! - [Art](https://symbl.cc/en/text-art/)

// 01: [x] impl glyphs

pub const HELLO_GLYPH: &str = "ヾ(◕‿◕)ノ";
pub const HUG_GLYPH: &str = "⊂(◕‿◕)つ";
pub const BYE_GLYPH: &str = "٩(◕‿◕｡)۶";
pub const CELEBRATE_GLYPH: &str = "▓▒░(°◡°)░▒▓";
pub const WOW_GLYPH: &str = "ヽ(°〇°)ﾉ";
pub const SHRUG_GLYPH: &str = "┐(シ)┌";
pub const ERROR_GLYPH: &str = "(｡•́︿•̀｡)"; //'❌';
pub const SUSPICIOUS_GLYPH: &str = "(↼_↼)";
pub const SMILING_GLYPH: &str = "(◕‿◕)";

pub const CONSTRUCT_GLYPH: &str = "⣮";
pub const STATS_GLYPH: &str = "◕";
pub const CLOCK_TICK_GLYPH: &str = "✹"; //'❀'; //'✲';
pub const STOP_GLYPH: &str = "∎";
pub const TOP_UNDERLINE_GLYPH: &str = "‾";
pub const SPACER_GLYPH: &str = " ";
pub const ELLIPSIS_GLYPH: &str = "…";
pub const RENDER_GLYPH: &str = "◧";
pub const PAINT_GLYPH: &str = "■";
pub const LIGHT_CHECK_MARK_GLYPH: &str = "🗸";
pub const HEAVY_CHECK_MARK_GLYPH: &str = "✓";
pub const PAREN_LEFT_GLYPH: &str = "❬";
pub const PAREN_RIGHT_GLYPH: &str = "❭";
pub const FANCY_BULLET_GLYPH: &str = "⮻";
pub const CUT_GLYPH: &str = "✀";
pub const FOCUS_GLYPH: &str = "⭆"; // '⬕'; //'◕';
pub const DOT_GLYPH: &str = "●";
pub const POINTER_DOTTED_GLYPH: &str = "ⴾ";
pub const GAME_CHAR_GLYPH: &str = "𜱐";
pub const TIRE_MARKS_GLYPH: &str = "␩";
pub const VERT_LINE_DASHED_GLYPH: &str = "┆";
pub const DIRECTION_GLYPH: &str = "➤";

pub const LOADING_GLYPH: &str = "░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
█▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀█
█░██░██░██░██░██░██░██░██░██░░░░░░░░░░█
█░██░██░██░██░██░██░██░██░██░░░░░░░░░░█
█▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄█
░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
░░█░░░░█▀▀▀█░█▀▀█░█▀▀▄░▀█▀░█▄░░█░█▀▀█░░
░░█░░░░█░░░█░█▄▄█░█░░█░░█░░█░█░█░█░▄▄░░
░░█▄▄█░█▄▄▄█░█░░█░█▄▄▀░▄█▄░█░░▀█░█▄▄█░░
░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░";
pub const CAT_GLYPH: &str = "░░▄▄▄░░░░░░░░░░░░░░░░░░░░░░░░░░░░▄▄▄░░
░▄████▄░░░░░░░░░░░░░░░░░░░░░░░▄▄████▄░
░██░▀▀███▄▄░▄▄▄████████▄▄▄░▄▄███▀░███░
░██░░░░░▀███████▀████▀▀██████▀░░░░███░
░██▄░░░░░░░░░▀█▀░███░░░██▀▀░░░░░░░██▀░
░▀██▄▄░░░░░░░░░░░░▀░░░░▀░░░░░░░▄▄▄██░░
░░▀██▀░░░░░░░░░░░░░░░░░░░░░░░░░▀███▀░░
░░▄██░░░░░░░░░░░░░░░░░░░░░░░░░░░░██▄░░
░░████▀░░███░░░░░░░░░░░░░░███░░█████░░
░░███▀░░░█████░░░░░░░░░░█████░░░▀███░░
░░██░░░░░░▀▀▀▀░░░░░░░░░░▀▀▀▀░░░░░▀██░░
▄▄███▄▄▄▄░░░░░░░░░░░░░░░░░░░░▄▄▄▄███▄▄
░▄▄██▄▄░░░▄█░░░░▄▀▀▀▀▄░░░░█▄░░░▄███▄▄░
▀░░▄████▀▀▀▀░░░░░▀▄▄▀░░░░░▀▀▀▀████▄░░▀
░▄▀░░▀███▄▄░░░█▄▄█▀▀█▄▄▀░░░▄▄██▀░░░▀▄░
░░░░░░░░▀███▄▄░░░░░░░░░░▄▄███▀░░░░░░░░
░░░░░░░░░░▀▀████▄▄▄▄▄▄████▀▀░░░░░░░░░░
░░░░░░░░░░░░░░▀▀▀▀▀▀▀▀▀▀░░░░░░░░░░░░░░";
pub const KITTY_GLYPH: &str = "░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
░░░░░░░░░░▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄░░░░░░░░░
░░░░░░░░▄▀░░░░░░░░░░░░▄░░░░░░░▀▄░░░░░░░
░░░░░░░░█░░▄░░░░▄░░░░░░░░░░░░░░█░░░░░░░
░░░░░░░░█░░░░░░░░░░░░▄█▄▄░░▄░░░█░▄▄▄░░░
░▄▄▄▄▄░░█░░░░░░▀░░░░▀█░░▀▄░░░░░█▀▀░██░░
░██▄▀██▄█░░░▄░░░░░░░██░░░░▀▀▀▀▀░░░░██░░
░░▀██▄▀██░░░░░░░░▀░██▀░░░░░░░░░░░░░▀██░
░░░░▀████░▀░░░░▄░░░██░░░▄█░░░░▄░▄█░░██░
░░░░░░░▀█░░░░▄░░░░░██░░░░▄░░░▄░░▄░░░██░
░░░░░░░▄█▄░░░░░░░░░░░▀▄░░▀▀▀▀▀▀▀▀░░▄▀░░
░░░░░░█▀▀█████████▀▀▀▀████████████▀░░░░
░░░░░░████▀░░███▀░░░░░░▀███░░▀██▀░░░░░░
░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░";
pub const HOMER_GLYPH: &str = "░░░░░░▄▄▄▄███▄▄▄▄░░░░░░░░░░░░░
░░░▄▄█▀░░░░░░░░░▀▀▄▄░░░░░░░░░░
░░█▀░░░░░░░░░░░░░░░▀█▄░░░░░░░░
░█▀░░░░░░░░░░░░░░░░░░█▄░░░░░░░
██░░░░░░░░░░░░░░░░░░░░█▄░░░░░░
█░░░░░░░░░░░░░░░░░░░░░░█▄░░░░░
██░░░░░░░░░░░░▄▄▄▄▄█▀▀▀██▄░░░░
▀█░░░░░░░░░▄█▀▀░░▀▀█▄░░░░█▄░░░
░█▄░▄░░░░░▄█░░░░░░░░█▄░█░░█░░░
░▄█▄██▄░░░█▄░░██░░░░██▄▄▄██░░░
░████░▀▀░░░█▄░░░░░░▄█░░░░░██░░
░█░░██▄▄░░░░▀██▄▄██▀▄▄▄▄▄▄█░░░
░░▄█▀░░░░░░░░░▄▄██▀▀▀▀▀▀▀░▀█▄░
░░▀█░░░░░░░▄█▀▀░░░░░░░░░░░░░█▄
░░░▀█▄▄█▀░█▀░░░░░░░░░░░░░░░▄█▀
░░░░░░██░▄█░░░█▀██▀▀█▀██▀▀▀▀░░
░░░░░▄█░░▀█░░▀█░█░░██░██░░░░░░
░░░░██▀█▄░▀█▄░▀▀████▀▀██░░░░░░
░░░░█░░░▀▀█▄▀█▄▄▄▄▄▄▄▄██▄░░░░░";
pub const VADER_GLYPH: &str = "░░░░░░░░░░░░░░▄▄▄▄▄░░░░░░░░░░░░░░
░░░░░░░░░░▄██████████▄▄░░░░░░░░░░
░░░░░░░░▄██████░█░██████▄░░░░░░░░
░░░░░░▄████████░█░████████░░░░░░░
░░░░░░█████████░█░█████████░░░░░░
░░░░░▄█████████░█░█████████░░░░░░
░░░░░██████████████████████░░░░░░
░░░░░██████████████████████▄░░░░░
░░░░▄████░░░░░▀█▄█▀░░░░░████░░░░░
░░░▄█████░░░░░░█▄█░░░░░░█████░░░░
░░▄████████▄▄▄█████▄▄▄████████░░░
░▄████▀███████████████████▀████░░
░████▀██████████░██████████▀████░
████▀██████████░█░█▀████████▀████
███▀▀░░░▀▀█▀█░█░█░█░█▀█▀▀░░░▀▀███
░▀░░░░░░░░░░█░█░█░█░█░░░░░░░░░░▀░
░░░░░░░░░░░░▀███████▀░░░░░░░░░░░░
░░░░░░░░░░░░░░▀▀█▀▀░░░░░░░░░░░░░░";
pub const I_LOVE_YOU: &str = "░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
░░░▄▄▄▄▄▄░░░░▄▄▄░░░░▄▄▄░░░░░░
░░░▀████▀░░▄█████▄▄█████▄░░░░
░░░░░██░░░████████████████░░░
░░░░░██░░░████████████████░░░
░░░░░██░░░▀██████████████▀░░░
░░░░▄██▄░░░░▀██████████▀░░░░░
░░░██████░░░░░▀██████▀░░░░░░░
░░░░░░░░░░░░░░░░▀██▀░░░░░░░░░
░░░░░░░░░░░░░▄▄░░░░░░░░░░░░░░
░░▀███░███▀▄█▀▀█▄░▀██▀░▀██▀░░
░░░░▀█▄█▀░▄█░░░░█▄░██░░░██░░░
░░░░░░█░░░██░░░░██░██░░░██░░░
░░░░░░█░░░░█▄░░▄█░░██░░░██░░░
░░░░▄███▄░░░▀██▀░░░░▀███▀░░░░
░░░░░░░░░░░░░░░░░░░░░░░░░░░░░";
