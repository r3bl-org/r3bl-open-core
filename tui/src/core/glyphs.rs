// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! More info:
//! - [List of all symbols]
//! - [Box drawing characters]
//! - [Block element characters]
//! - [Geometric shape characters]
//! - [Arrow]
//! - [Arrow symbols]
//! - [Brackets]
//! - [Supplemental arrow characters-A]
//! - [Supplemental arrow characters-B]
//! - [Dingbat characters]
//! - [Braille pattern characters]
//! - [Geometric shapes]
//! - [Miscellaneous symbol and arrow characters]
//! - [Tifinagh characters]
//! - [Ideographic characters]
//! - [Emotions kaomoji]
//! - [ASCII Art]
//!
//! [Arrow symbols]: https://symbl.cc/en/collections/arrow-symbols/
//! [Arrow]: https://symbl.cc/en/unicode/blocks/arrows/
//! [ASCII Art]: https://symbl.cc/en/text-art/
//! [Block element characters]: https://symbl.cc/en/unicode/blocks/block-elements/
//! [Box drawing characters]: https://symbl.cc/en/unicode/blocks/box-drawing/
//! [Brackets]: https://symbl.cc/en/collections/brackets/
//! [Braille pattern characters]: https://symbl.cc/en/unicode/blocks/braille-patterns/
//! [Dingbat characters]: https://symbl.cc/en/unicode/blocks/dingbats/
//! [Emotions kaomoji]: https://symbl.cc/en/kaomoji/
//! [Geometric shape characters]: https://symbl.cc/en/unicode/blocks/geometric-shapes/
//! [Ideographic characters]: https://symbl.cc/en/unicode/blocks/ideographic-description-characters/
//! [List of all symbols]: https://symbl.cc/en/unicode-table/#miscellaneous-technical
//! [Miscellaneous symbol and arrow characters]: https://symbl.cc/en/unicode/blocks/miscellaneous-symbols-and-arrows/
//! [Supplemental arrow characters-A]: https://symbl.cc/en/unicode/blocks/supplemental-arrows-a/
//! [Supplemental arrow characters-B]: https://symbl.cc/en/unicode/blocks/supplemental-arrows-b/
//! [Tifinagh characters]: https://symbl.cc/en/unicode/blocks/tifinagh/

pub const HELLO_GLYPH: &str = "ヾ(◕‿◕)ノ";
pub const HUG_GLYPH: &str = "⊂(◕‿◕)つ";
pub const BYE_GLYPH: &str = "٩(◕‿◕｡)۶";
pub const CELEBRATE_GLYPH: &str = "▓▒░(◕‿◕)░▒▓"; // "▓▒░(°◡°)░▒▓";
pub const WOW_GLYPH: &str = "ヽ(°〇°)ﾉ";
pub const SHRUG_GLYPH: &str = "┐(シ)┌";
pub const ERROR_GLYPH: &str = "(｡•́︿•̀｡)"; //'❌';
pub const SUSPICIOUS_GLYPH: &str = "(↼_↼)";
pub const SMILING_GLYPH: &str = "(◕‿◕)";

pub const SCREEN_BUFFER_GLYPH: &str = "▦";
pub const RIGHT_ARROW_GLYPH: &str = "→";
pub const RIGHT_ARROW_DASHED_GLYPH: &str = "⇢";
pub const CONSTRUCT_GLYPH: &str = "⣮";
pub const STATS_25P_GLYPH: &str = "◔";
pub const STATS_50P_GLYPH: &str = "◑";
pub const STATS_75P_GLYPH: &str = "◕";
pub const STATS_100P_GLYPH: &str = "●";
pub const CLOCK_TICK_GLYPH: &str = "↻"; // "↺"; //"✹"; //'❀'; //'✲';
pub const STOP_GLYPH: &str = "∎";
pub const TOP_UNDERLINE_GLYPH: &str = "‾";
pub const SPACER_GLYPH: &str = " ";
pub const SPACER_GLYPH_CHAR: char = ' ';
pub const ELLIPSIS_GLYPH: &str = "…";
pub const RENDER_GLYPH: &str = "◧";
pub const PAINT_GLYPH: &str = "■";
pub const BOX_FILL_GLYPH: &str = "▣";
pub const BOX_EMPTY_GLYPH: &str = "□";
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
pub const USER_INPUT_GLYPH: &str = "↹"; //"⿻";
pub const TERMINAL: &str = "";
pub const PROMPT: &str = "❯";

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
