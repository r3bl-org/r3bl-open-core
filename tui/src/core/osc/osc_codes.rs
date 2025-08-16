// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! OSC sequence constants and codes.

/// OSC 9;4 sequence prefix: ESC ] 9 ; 4 ;
pub const START: &str = "\x1b]9;4;";
/// OSC 8 hyperlink sequence prefix: ESC ] 8 ; ;
pub const OSC8_START: &str = "\x1b]8;;";
/// Sequence terminator: ESC \\ (String Terminator)
pub const END: &str = "\x1b\\";
/// Parameter delimiter within OSC sequences
pub const DELIMITER: char = ';';
