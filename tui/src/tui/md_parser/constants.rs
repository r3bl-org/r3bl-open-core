// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

/// All the Markdown literals that are used to perform parsing.
pub const AUTHORS: &str = "@authors";
pub const DATE: &str = "@date";
pub const TITLE: &str = "@title";
pub const TAGS: &str = "@tags";
pub const COLON: &str = ":";
pub const COLON_CHAR: char = ':';
pub const COMMA: &str = ",";
pub const COMMA_CHAR: char = ',';
pub const QUOTE: &str = "\"";
pub const HEADING: &str = "#";
pub const HEADING_CHAR: char = '#';
pub const SPACE: &str = " ";
pub const SPACE_CHAR: char = ' ';
pub const PERIOD: &str = ".";
pub const LIST_PREFIX_BASE_WIDTH: usize = 2;

/// Only for output to terminal.
pub const LIST_SPACE_DISPLAY: &str = "─";
pub const LIST_SPACE_DISPLAY_CHAR: char = '─';

/// Only for output to terminal.
pub const LIST_SPACE_END_DISPLAY_FIRST_LINE: &str = "┤"; // "┼" or "|" or "┤"

/// Only for output to terminal.
pub const LIST_SPACE_END_DISPLAY_REST_LINE: &str = "│"; // "|";

pub const UNORDERED_LIST: &str = "-";
pub const UNORDERED_LIST_PREFIX: &str = "- ";
pub const ORDERED_LIST_PARTIAL_PREFIX: &str = ". ";
pub const STAR: &str = "*";
pub const UNDERSCORE: &str = "_";
pub const BACK_TICK: &str = "`";
pub const BACK_TICK_CHAR: char = '`';
pub const LEFT_BRACKET: &str = "[";
pub const RIGHT_BRACKET: &str = "]";
pub const LEFT_PARENTHESIS: &str = "(";
pub const RIGHT_PARENTHESIS: &str = ")";
pub const LEFT_IMAGE: &str = "![";
pub const RIGHT_IMAGE: &str = "]";
pub const NEW_LINE: &str = "\n";
pub const NEW_LINE_CHAR: char = '\n';
pub const CODE_BLOCK_START_PARTIAL: &str = "```";
pub const CODE_BLOCK_END: &str = "```";
pub const CHECKED: &str = "[x]";
pub const UNCHECKED: &str = "[ ]";
pub const CHECKED_UPPER: &str = "[X]";
pub const UNCHECKED_UPPER: &str = "[ ]"; // Same as lowercase
pub const CHECKED_OUTPUT: &str = "┊✔┊";
pub const UNCHECKED_OUTPUT: &str = "┊┈┊";
pub const EXCLAMATION: &str = "!";

pub const TAB_CHAR: char = '\t';
pub const NULL_CHAR: char = '\0';
pub const NULL_STR: &str = "\0";
pub const NEWLINE_OR_NULL: &str = "\n\0";
