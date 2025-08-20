// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! ANSI escape code generation with optimized performance.
//!
//! ## Performance Optimization
//!
//! This module uses a lookup table approach to avoid the overhead of Rust's formatting
//! machinery. Even though `write!` macro writes to an in-memory buffer, it still incurs
//! significant overhead:
//!
//! 1. **Format machinery dispatch**: The `write!` macro expands to `format_args!` which
//!    creates a `fmt::Arguments` struct and dispatches through the Display trait.
//!
//! 2. **Integer formatting overhead**: For number placeholders like `{index}`, Rust's
//!    integer Display implementation:
//!    - Allocates temporary buffers
//!    - Performs division/modulo operations in a loop
//!    - Handles sign, radix, padding, alignment (even when unused)
//!    - Builds the string representation digit by digit at runtime
//!
//! 3. **Hot path impact**: ANSI codes are generated millions of times per second in a
//!    TUI:
//!    - Every styled text segment
//!    - Every color change
//!    - Every style change (bold, underline, etc.)
//!    - Multiple times per frame at 60 FPS
//!
//! ## Optimization Strategy
//!
//! We use a pre-computed lookup table for all possible u8 values (0-255):
//! - Eliminates integer-to-string conversion
//! - Removes format machinery dispatch
//! - Avoids temporary allocations
//! - Reduces to simple array lookup + memcpy
//!
//! This optimization targets the 45M samples shown in flamegraph profiling for ANSI
//! formatting.
//!
//! More info:
//! - <https://doc.rust-lang.org/reference/tokens.html#ascii-escapes>
//! - <https://notes.burke.libbey.me/ansi-escape-codes/>

use std::fmt::{Display, Formatter, Result};

use crate::{BufTextStorage, WriteToBuf};

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

const CSI: &str = "\x1b[";
const SGR: &str = "m";

/// Lookup table for u8 to string conversion to avoid runtime formatting overhead.
/// Pre-computed at compile time for all possible u8 values (0-255).
const U8_STRINGS: [&str; 256] = [
    "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15",
    "16", "17", "18", "19", "20", "21", "22", "23", "24", "25", "26", "27", "28", "29",
    "30", "31", "32", "33", "34", "35", "36", "37", "38", "39", "40", "41", "42", "43",
    "44", "45", "46", "47", "48", "49", "50", "51", "52", "53", "54", "55", "56", "57",
    "58", "59", "60", "61", "62", "63", "64", "65", "66", "67", "68", "69", "70", "71",
    "72", "73", "74", "75", "76", "77", "78", "79", "80", "81", "82", "83", "84", "85",
    "86", "87", "88", "89", "90", "91", "92", "93", "94", "95", "96", "97", "98", "99",
    "100", "101", "102", "103", "104", "105", "106", "107", "108", "109", "110", "111",
    "112", "113", "114", "115", "116", "117", "118", "119", "120", "121", "122", "123",
    "124", "125", "126", "127", "128", "129", "130", "131", "132", "133", "134", "135",
    "136", "137", "138", "139", "140", "141", "142", "143", "144", "145", "146", "147",
    "148", "149", "150", "151", "152", "153", "154", "155", "156", "157", "158", "159",
    "160", "161", "162", "163", "164", "165", "166", "167", "168", "169", "170", "171",
    "172", "173", "174", "175", "176", "177", "178", "179", "180", "181", "182", "183",
    "184", "185", "186", "187", "188", "189", "190", "191", "192", "193", "194", "195",
    "196", "197", "198", "199", "200", "201", "202", "203", "204", "205", "206", "207",
    "208", "209", "210", "211", "212", "213", "214", "215", "216", "217", "218", "219",
    "220", "221", "222", "223", "224", "225", "226", "227", "228", "229", "230", "231",
    "232", "233", "234", "235", "236", "237", "238", "239", "240", "241", "242", "243",
    "244", "245", "246", "247", "248", "249", "250", "251", "252", "253", "254", "255",
];

impl Display for SgrCode {
    /// SGR: set graphics mode command.
    /// More info:
    /// - <https://notes.burke.libbey.me/ansi-escape-codes/>
    /// - <https://www.asciitable.com/>
    /// - <https://commons.wikimedia.org/wiki/File:Xterm_256color_chart.svg>
    /// - <https://en.wikipedia.org/wiki/ANSI_escape_code>
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        // Delegate to WriteToBuf for consistency.
        let mut acc = BufTextStorage::new();
        self.write_to_buf(&mut acc)?;
        self.write_buf_to_fmt(&acc, f)
    }
}

/// [`WriteToBuf`] implementation for optimized performance.
/// Uses direct string concatenation and lookup tables to avoid formatting overhead.
impl WriteToBuf for SgrCode {
    #[allow(clippy::too_many_lines)]
    fn write_to_buf(&self, buf: &mut crate::BufTextStorage) -> Result {
        match *self {
            SgrCode::Reset => {
                buf.push_str(CSI);
                buf.push('0');
                buf.push_str(SGR);
                Ok(())
            }
            SgrCode::Bold => {
                buf.push_str(CSI);
                buf.push('1');
                buf.push_str(SGR);
                Ok(())
            }
            SgrCode::Dim => {
                buf.push_str(CSI);
                buf.push('2');
                buf.push_str(SGR);
                Ok(())
            }
            SgrCode::Italic => {
                buf.push_str(CSI);
                buf.push('3');
                buf.push_str(SGR);
                Ok(())
            }
            SgrCode::Underline => {
                buf.push_str(CSI);
                buf.push('4');
                buf.push_str(SGR);
                Ok(())
            }
            SgrCode::SlowBlink => {
                buf.push_str(CSI);
                buf.push('5');
                buf.push_str(SGR);
                Ok(())
            }
            SgrCode::RapidBlink => {
                buf.push_str(CSI);
                buf.push('6');
                buf.push_str(SGR);
                Ok(())
            }
            SgrCode::Invert => {
                buf.push_str(CSI);
                buf.push('7');
                buf.push_str(SGR);
                Ok(())
            }
            SgrCode::Hidden => {
                buf.push_str(CSI);
                buf.push('8');
                buf.push_str(SGR);
                Ok(())
            }
            SgrCode::Strikethrough => {
                buf.push_str(CSI);
                buf.push('9');
                buf.push_str(SGR);
                Ok(())
            }
            SgrCode::Overline => {
                buf.push_str(CSI);
                buf.push_str("53");
                buf.push_str(SGR);
                Ok(())
            }
            SgrCode::ForegroundAnsi256(index) => {
                buf.push_str(CSI);
                buf.push_str("38;5;");
                buf.push_str(U8_STRINGS[index as usize]);
                buf.push_str(SGR);
                Ok(())
            }
            SgrCode::BackgroundAnsi256(index) => {
                buf.push_str(CSI);
                buf.push_str("48;5;");
                buf.push_str(U8_STRINGS[index as usize]);
                buf.push_str(SGR);
                Ok(())
            }
            SgrCode::ForegroundRGB(r, g, b) => {
                buf.push_str(CSI);
                buf.push_str("38;2;");
                buf.push_str(U8_STRINGS[r as usize]);
                buf.push(';');
                buf.push_str(U8_STRINGS[g as usize]);
                buf.push(';');
                buf.push_str(U8_STRINGS[b as usize]);
                buf.push_str(SGR);
                Ok(())
            }
            SgrCode::BackgroundRGB(r, g, b) => {
                buf.push_str(CSI);
                buf.push_str("48;2;");
                buf.push_str(U8_STRINGS[r as usize]);
                buf.push(';');
                buf.push_str(U8_STRINGS[g as usize]);
                buf.push(';');
                buf.push_str(U8_STRINGS[b as usize]);
                buf.push_str(SGR);
                Ok(())
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

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

#[cfg(test)]
mod benchmarks {
    extern crate test;
    use std::fmt::Write;

    use test::Bencher;

    use crate::{BufTextStorage, SgrCode, WriteToBuf};

    #[bench]
    fn bench_ansi256_formatting_all_values(b: &mut Bencher) {
        let codes: Vec<SgrCode> = (0..=255).map(SgrCode::ForegroundAnsi256).collect();
        b.iter(|| {
            let mut buf = BufTextStorage::new();
            for code in &codes {
                code.write_to_buf(&mut buf).unwrap();
            }
            test::black_box(buf);
        });
    }

    #[bench]
    fn bench_rgb_formatting_all_values(b: &mut Bencher) {
        let codes: Vec<SgrCode> = (0..=255u8)
            .map(|i| SgrCode::ForegroundRGB(i, i.wrapping_add(50), i.wrapping_add(100)))
            .collect();
        b.iter(|| {
            let mut buf = BufTextStorage::new();
            for code in &codes {
                code.write_to_buf(&mut buf).unwrap();
            }
            test::black_box(buf);
        });
    }

    #[bench]
    fn bench_mixed_sgr_codes(b: &mut Bencher) {
        let codes = vec![
            SgrCode::Reset,
            SgrCode::Bold,
            SgrCode::ForegroundAnsi256(150),
            SgrCode::BackgroundAnsi256(42),
            SgrCode::ForegroundRGB(175, 215, 135),
            SgrCode::Underline,
            SgrCode::BackgroundRGB(50, 100, 150),
        ];
        b.iter(|| {
            let mut buf = BufTextStorage::new();
            for _ in 0..100 {
                for code in &codes {
                    code.write_to_buf(&mut buf).unwrap();
                }
            }
            test::black_box(buf);
        });
    }

    #[bench]
    fn bench_simple_codes_only(b: &mut Bencher) {
        let codes = vec![
            SgrCode::Reset,
            SgrCode::Bold,
            SgrCode::Dim,
            SgrCode::Italic,
            SgrCode::Underline,
            SgrCode::Strikethrough,
        ];
        b.iter(|| {
            let mut buf = BufTextStorage::new();
            for _ in 0..1000 {
                for code in &codes {
                    code.write_to_buf(&mut buf).unwrap();
                }
            }
            test::black_box(buf);
        });
    }

    // Benchmark to compare old write! approach (if we had kept it)
    #[bench]
    fn bench_write_macro_for_comparison(b: &mut Bencher) {
        b.iter(|| {
            let mut buf = BufTextStorage::new();
            for i in 0..=255u8 {
                write!(buf, "\x1b[38;5;{i}m").unwrap();
            }
            test::black_box(buf);
        });
    }

    // Benchmark to show lookup table approach speed
    #[bench]
    fn bench_lookup_table_direct(b: &mut Bencher) {
        const U8_STRINGS: [&str; 256] = [
            "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13",
            "14", "15", "16", "17", "18", "19", "20", "21", "22", "23", "24", "25", "26",
            "27", "28", "29", "30", "31", "32", "33", "34", "35", "36", "37", "38", "39",
            "40", "41", "42", "43", "44", "45", "46", "47", "48", "49", "50", "51", "52",
            "53", "54", "55", "56", "57", "58", "59", "60", "61", "62", "63", "64", "65",
            "66", "67", "68", "69", "70", "71", "72", "73", "74", "75", "76", "77", "78",
            "79", "80", "81", "82", "83", "84", "85", "86", "87", "88", "89", "90", "91",
            "92", "93", "94", "95", "96", "97", "98", "99", "100", "101", "102", "103",
            "104", "105", "106", "107", "108", "109", "110", "111", "112", "113", "114",
            "115", "116", "117", "118", "119", "120", "121", "122", "123", "124", "125",
            "126", "127", "128", "129", "130", "131", "132", "133", "134", "135", "136",
            "137", "138", "139", "140", "141", "142", "143", "144", "145", "146", "147",
            "148", "149", "150", "151", "152", "153", "154", "155", "156", "157", "158",
            "159", "160", "161", "162", "163", "164", "165", "166", "167", "168", "169",
            "170", "171", "172", "173", "174", "175", "176", "177", "178", "179", "180",
            "181", "182", "183", "184", "185", "186", "187", "188", "189", "190", "191",
            "192", "193", "194", "195", "196", "197", "198", "199", "200", "201", "202",
            "203", "204", "205", "206", "207", "208", "209", "210", "211", "212", "213",
            "214", "215", "216", "217", "218", "219", "220", "221", "222", "223", "224",
            "225", "226", "227", "228", "229", "230", "231", "232", "233", "234", "235",
            "236", "237", "238", "239", "240", "241", "242", "243", "244", "245", "246",
            "247", "248", "249", "250", "251", "252", "253", "254", "255",
        ];

        b.iter(|| {
            let mut buf = BufTextStorage::new();
            for i in 0..=255u8 {
                buf.push_str("\x1b[38;5;");
                buf.push_str(U8_STRINGS[i as usize]);
                buf.push('m');
            }
            test::black_box(buf);
        });
    }
}
