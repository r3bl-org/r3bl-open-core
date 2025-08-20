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

// Terminal title and tab control sequences

/// OSC 0 sequence: Set both window title and tab name (ESC ] 0 ;)
/// We only implement OSC 0 (title + tab). OSC 1 (icon only) and OSC 2
/// (title only) are not needed for modern terminal multiplexing where
/// consistent branding is preferred.
pub const OSC0_SET_TITLE_AND_TAB: &str = "\x1b]0;";

/// Alternative terminator: BEL character (0x07)
/// Some terminals prefer this over the `STRING_TERMINATOR`
pub const BELL_TERMINATOR: &str = "\x07";
