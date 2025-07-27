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

use crate::{InlineVec, List};

/// This corresponds to a single Markdown document, which is produced after a successful
/// parse operation [`crate::parse_markdown()`].
pub type MdDocument<'a> = List<MdElement<'a>>;

/// Alias for [`MdDocument`].
pub type Blocks<'a> = MdDocument<'a>;

/// This roughly corresponds to a single line of text. Each line is made up of one or more
/// [`MdLineFragment`].
pub type MdLineFragments<'a> = List<MdLineFragment<'a>>;

/// Alias for [`MdLineFragments`].
pub type FragmentsInOneLine<'a> = MdLineFragments<'a>;

/// Alias for [List] of [`FragmentsInOneLine`].
pub type Lines<'a> = List<FragmentsInOneLine<'a>>;

#[derive(Clone, Debug, PartialEq)]
pub struct HeadingData<'a> {
    pub level: HeadingLevel,
    pub text: &'a str,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum CheckboxParsePolicy {
    IgnoreCheckbox,
    ParseCheckbox,
}

/// A Markdown document once parsed is turned into a [Vec] of "blocks".
///
/// A block roughly represents a single line of text, and is the top-level entity of a
/// Markdown document and roughly represents a single line of text.
/// - It is the intermediate representation (IR) of a single line of text.
/// - There are some exceptions such as smart lists and code blocks which represent
///   multiple lines of text.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq)]
pub enum MdElement<'a> {
    Heading(HeadingData<'a>),
    SmartList((Lines<'a>, BulletKind, usize)),
    Text(MdLineFragments<'a>),
    CodeBlock(List<CodeBlockLine<'a>>),
    Title(&'a str),
    Date(&'a str),
    Tags(List<&'a str>),
    Authors(List<&'a str>),
}

/// These are things that show up in a single line of Markdown text [`MdLineFragments`].
/// They do not include other Markdown blocks (like code blocks, lists, headings, etc).
#[derive(Clone, Debug, PartialEq)]
pub enum MdLineFragment<'a> {
    UnorderedListBullet {
        indent: usize,
        is_first_line: bool,
    },
    OrderedListBullet {
        indent: usize,
        number: usize,
        is_first_line: bool,
    },
    Plain(&'a str),
    Bold(&'a str),
    Italic(&'a str),
    InlineCode(&'a str),
    Link(HyperlinkData<'a>),
    Image(HyperlinkData<'a>),
    Checkbox(bool),
}

#[derive(Clone, Debug, PartialEq)]
pub struct HyperlinkData<'a> {
    pub text: &'a str,
    pub url: &'a str,
}

mod hyperlink_data_impl {
    use super::HyperlinkData;

    impl<'a> HyperlinkData<'a> {
        #[must_use]
        pub fn new(text: &'a str, url: &'a str) -> Self { Self { text, url } }
    }

    impl<'a> From<(&'a str, &'a str)> for HyperlinkData<'a> {
        fn from((text, url): (&'a str, &'a str)) -> Self { Self { text, url } }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct HeadingLevel {
    pub level: usize,
}

impl From<HeadingLevel> for usize {
    fn from(other: HeadingLevel) -> Self { other.level }
}

impl From<usize> for HeadingLevel {
    fn from(size: usize) -> Self { HeadingLevel { level: size } }
}

/// All the Markdown literals that are used to perform parsing.
pub mod constants {
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
}

#[derive(Debug, PartialEq, Clone)]
pub struct CodeBlockLine<'a> {
    pub language: Option<&'a str>,
    pub content: CodeBlockLineContent<'a>,
}

/// Alias for [List] of [`CodeBlockLine`].
pub type CodeBlockLines<'a> = List<CodeBlockLine<'a>>;

#[derive(Debug, PartialEq, Clone)]
pub enum CodeBlockLineContent<'a> {
    Text(&'a str),
    StartTag,
    EndTag,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum BulletKind {
    Ordered(usize),
    Unordered,
}

/// Holds a single list item for a given indent level. This may contain multiple lines
/// which are stored in the `content_lines` field. Take a look at
/// [`crate::md_parser::block::parse_smart_list_block::parse_smart_list`]
/// for more details.
#[derive(Clone, Debug, PartialEq)]
pub struct SmartListIR<'a, T> {
    /// Spaces before the bullet (for all the lines in this list).
    pub indent: usize,
    /// Unordered or ordered.
    pub bullet_kind: BulletKind,
    /// Does not contain any bullets.
    pub content_lines: InlineVec<SmartListLine<'a, T>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SmartListLine<'a, T> {
    /// Spaces before the bullet (for all the lines in this list).
    pub indent: usize,
    /// Unordered or ordered.
    pub bullet_str: &'a str,
    /// Does not contain any bullets or any spaces for the indent prefix.
    pub content: T,
}

impl<'a, T> SmartListLine<'a, T> {
    pub fn new(indent: usize, bullet_str: &'a str, content: T) -> Self {
        Self {
            indent,
            bullet_str,
            content,
        }
    }
}

// Type aliases for backward compatibility.

pub type SmartListIRStr<'a> = SmartListIR<'a, &'a str>;
pub type SmartListLineStr<'a> = SmartListLine<'a, &'a str>;
