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

//! More info:
//! - <https://doc.rust-lang.org/reference/tokens.html#ascii-escapes>
//! - <https://notes.burke.libbey.me/ansi-escape-codes/>

use std::fmt::{Display, Formatter, Result};

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
    use super::*;

    pub const CSI: &str = "\x1b[";
    pub const SGR: &str = "m";

    impl Display for SgrCode {
        /// SGR: set graphics mode command.
        /// More info:
        /// - <https://notes.burke.libbey.me/ansi-escape-codes/>
        /// - <https://www.asciitable.com/>
        /// - <https://commons.wikimedia.org/wiki/File:Xterm_256color_chart.svg>
        /// - <https://en.wikipedia.org/wiki/ANSI_escape_code>
        #[rustfmt::skip]
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            match *self {
                SgrCode::Reset                    => write!(f, "{CSI}0{SGR}"),
                SgrCode::Bold                     => write!(f, "{CSI}1{SGR}"),
                SgrCode::Dim                      => write!(f, "{CSI}2{SGR}"),
                SgrCode::Italic                   => write!(f, "{CSI}3{SGR}"),
                SgrCode::Underline                => write!(f, "{CSI}4{SGR}"),
                SgrCode::SlowBlink                => write!(f, "{CSI}5{SGR}"),
                SgrCode::RapidBlink               => write!(f, "{CSI}6{SGR}"),
                SgrCode::Invert                   => write!(f, "{CSI}7{SGR}"),
                SgrCode::Hidden                   => write!(f, "{CSI}8{SGR}"),
                SgrCode::Strikethrough            => write!(f, "{CSI}9{SGR}"),
                SgrCode::Overline                 => write!(f, "{CSI}53{SGR}"),
                SgrCode::ForegroundAnsi256(index) => write!(f, "{CSI}38;5;{index}{SGR}"),
                SgrCode::BackgroundAnsi256(index) => write!(f, "{CSI}48;5;{index}{SGR}"),
                SgrCode::ForegroundRGB(r, g, b)   => write!(f, "{CSI}38;2;{r};{g};{b}{SGR}"),
                SgrCode::BackgroundRGB(r, g, b)   => write!(f, "{CSI}48;2;{r};{g};{b}{SGR}"),
            }

        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::SgrCode;

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
    fn dim() {
        let sgr_code = SgrCode::Dim;
        assert_eq!(sgr_code.to_string(), "\x1b[2m");
    }

    #[test]
    fn italic() {
        let sgr_code = SgrCode::Italic;
        assert_eq!(sgr_code.to_string(), "\x1b[3m");
    }

    #[test]
    fn underline() {
        let sgr_code = SgrCode::Underline;
        assert_eq!(sgr_code.to_string(), "\x1b[4m");
    }

    #[test]
    fn slowblink() {
        let sgr_code = SgrCode::SlowBlink;
        assert_eq!(sgr_code.to_string(), "\x1b[5m");
    }

    #[test]
    fn rapidblink() {
        let sgr_code = SgrCode::RapidBlink;
        assert_eq!(sgr_code.to_string(), "\x1b[6m");
    }

    #[test]
    fn invert() {
        let sgr_code = SgrCode::Invert;
        assert_eq!(sgr_code.to_string(), "\x1b[7m");
    }

    #[test]
    fn hidden() {
        let sgr_code = SgrCode::Hidden;
        assert_eq!(sgr_code.to_string(), "\x1b[8m");
    }

    #[test]
    fn strikethrough() {
        let sgr_code = SgrCode::Strikethrough;
        assert_eq!(sgr_code.to_string(), "\x1b[9m");
    }

    #[test]
    fn overline() {
        let sgr_code = SgrCode::Overline;
        assert_eq!(sgr_code.to_string(), "\x1b[53m");
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
