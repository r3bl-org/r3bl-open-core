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

//! More info:
//! - <https://doc.rust-lang.org/reference/tokens.html#ascii-escapes>
//! - <https://notes.burke.libbey.me/ansi-escape-codes/>

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SgrCode {
    Reset,
    Bold,
    Dim,
    Italic,
    Underline,
    Overline,
    SlowBlink,
    RapidBlink,
    Invert,
    Hidden,
    Strikethrough,
    ForegroundAnsi256(u8),
    BackgroundAnsi256(u8),
    ForegroundRGB(u8, u8, u8),
    BackgroundRGB(u8, u8, u8),
}

pub mod sgr_code_impl {
    use crate::*;
    use std::fmt::{Display, Formatter, Result};

    impl Display for SgrCode {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            write!(f, "{}", make_sgr_code(*self))
        }
    }

    pub const CSI: &str = "\x1b[";
    pub const SGR: &str = "m";

    /// SGR: set graphics mode command.
    /// More info:
    /// - <https://notes.burke.libbey.me/ansi-escape-codes/>
    /// - <https://www.asciitable.com/>
    /// - <https://commons.wikimedia.org/wiki/File:Xterm_256color_chart.svg>
    /// - <https://en.wikipedia.org/wiki/ANSI_escape_code>
    #[rustfmt::skip]
    fn make_sgr_code(sgr_code: SgrCode) -> String {
        match sgr_code {
            SgrCode::Reset             => format!("{CSI}0{SGR}"),
            SgrCode::Bold              => format!("{CSI}1{SGR}"),
            SgrCode::Dim               => format!("{CSI}2{SGR}"),
            SgrCode::Italic            => format!("{CSI}3{SGR}"),
            SgrCode::Underline         => format!("{CSI}4{SGR}"),
            SgrCode::SlowBlink         => format!("{CSI}5{SGR}"),
            SgrCode::RapidBlink        => format!("{CSI}6{SGR}"),
            SgrCode::Invert            => format!("{CSI}7{SGR}"),
            SgrCode::Hidden            => format!("{CSI}8{SGR}"),
            SgrCode::Strikethrough     => format!("{CSI}9{SGR}"),
            SgrCode::Overline           => format!("{CSI}53{SGR}"),
            SgrCode::ForegroundAnsi256(index) => format!("{CSI}38;5;{index}{SGR}"),
            SgrCode::BackgroundAnsi256(index) => format!("{CSI}48;5;{index}{SGR}"),
            SgrCode::ForegroundRGB(r, g, b) => format!("{CSI}38;2;{r};{g};{b}{SGR}"),
            SgrCode::BackgroundRGB(r, g, b) => format!("{CSI}48;2;{r};{g};{b}{SGR}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SgrCode;
    use pretty_assertions::assert_eq;

    #[test]
    fn bold() {
        let sgr_code = SgrCode::Bold;
        assert_eq!(sgr_code.to_string(), "\x1b[1m");
    }

    #[test]
    fn reset() {
        let sgr_code = SgrCode::Reset;
        assert_eq!(sgr_code.to_string(), "\x1b[0m");
    }

    #[test]
    fn fg_color_ansi256() {
        let sgr_code = SgrCode::ForegroundAnsi256(150);
        assert_eq!(sgr_code.to_string(), "\x1b[38;5;150m");
    }

    #[test]
    fn bg_color_ansi256() {
        let sgr_code = SgrCode::BackgroundAnsi256(150);
        assert_eq!(sgr_code.to_string(), "\x1b[48;5;150m");
    }

    #[test]
    fn fg_color_rgb() {
        let sgr_code = SgrCode::ForegroundRGB(175, 215, 135);
        assert_eq!(sgr_code.to_string(), "\x1b[38;2;175;215;135m");
    }

    #[test]
    fn bg_color_rgb() {
        let sgr_code = SgrCode::BackgroundRGB(175, 215, 135);
        assert_eq!(sgr_code.to_string(), "\x1b[48;2;175;215;135m");
    }
}
