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

use crate::List;

/// This corresponds to a single Markdown document, which is produced after a successful parse
/// operation [crate::parse_markdown].
pub type MdDocument<'a> = List<MdBlockElement<'a>>;

/// Alias for [MdDocument].
pub type Blocks<'a> = MdDocument<'a>;

/// This roughly corresponds to a single line of text. Each line is made up of one or more
/// [MdLineFragment].
pub type MdLineFragments<'a> = List<MdLineFragment<'a>>;

/// Alias for [MdLineFragments].
pub type FragmentsInOneLine<'a> = MdLineFragments<'a>;

/// Alias for [List] of [FragmentsInOneLine].
pub type Lines<'a> = List<FragmentsInOneLine<'a>>;

#[derive(Clone, Debug, PartialEq)]
pub struct HeadingData<'a> {
    pub level: HeadingLevel,
    pub text: &'a str,
}

/// These are blocks of Markdown. Blocks are the top-level elements of a Markdown document. A
/// Markdown document once parsed is turned into a [Vec] of these.
#[derive(Clone, Debug, PartialEq)]
pub enum MdBlockElement<'a> {
    Heading(HeadingData<'a>),
    OrderedList(Lines<'a>),
    UnorderedList(Lines<'a>),
    Text(MdLineFragments<'a>),
    CodeBlock(List<CodeBlockLine<'a>>),
    Title(&'a str),
    Tags(List<&'a str>),
}

/// These are things that show up in a single line of Markdown text [MdLineFragments]. They do
/// not include other Markdown blocks (like code blocks, lists, headings, etc).
#[derive(Clone, Debug, PartialEq)]
pub enum MdLineFragment<'a> {
    UnorderedListItem,
    OrderedListItemNumber(usize),
    Plain(&'a str),
    Bold(&'a str),
    Italic(&'a str),
    BoldItalic(&'a str),
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
    use super::*;

    impl<'a> HyperlinkData<'a> {
        pub fn new(text: &'a str, url: &'a str) -> Self { Self { text, url } }
    }

    impl<'a> From<(&'a str, &'a str)> for HyperlinkData<'a> {
        fn from((text, url): (&'a str, &'a str)) -> Self { Self { text, url } }
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum HeadingLevel {
    Heading1 = 1,
    Heading2,
    Heading3,
    Heading4,
    Heading5,
    Heading6,
}

impl From<HeadingLevel> for usize {
    fn from(level: HeadingLevel) -> Self { (level as u8).into() }
}

impl From<usize> for HeadingLevel {
    fn from(size: usize) -> Self {
        match size {
            1 => HeadingLevel::Heading1,
            2 => HeadingLevel::Heading2,
            3 => HeadingLevel::Heading3,
            4 => HeadingLevel::Heading4,
            5 => HeadingLevel::Heading5,
            6 => HeadingLevel::Heading6,
            _ => HeadingLevel::Heading6,
        }
    }
}

/// All the Markdown literals that are used to perform parsing.
pub mod constants {
    pub const TITLE: &str = "@title";
    pub const TAGS: &str = "@tags";
    pub const COLON: &str = ":";
    pub const COMMA: &str = ",";
    pub const QUOTE: &str = "\"";
    pub const HEADING_CHAR: char = '#';
    pub const SPACE: &str = " ";
    pub const PERIOD: &str = ".";
    pub const UNORDERED_LIST: &str = "-";
    pub const BITALIC_1: &str = "***";
    pub const BITALIC_2: &str = "___";
    pub const BOLD_1: &str = "**";
    pub const BOLD_2: &str = "__";
    pub const ITALIC_1: &str = "*";
    pub const ITALIC_2: &str = "_";
    pub const BACK_TICK: &str = "`";
    pub const LEFT_BRACKET: &str = "[";
    pub const RIGHT_BRACKET: &str = "]";
    pub const LEFT_PARENTHESIS: &str = "(";
    pub const RIGHT_PARENTHESIS: &str = ")";
    pub const LEFT_IMAGE: &str = "![";
    pub const RIGHT_IMAGE: &str = "]";
    pub const NEW_LINE: &str = "\n";
    pub const CODE_BLOCK_START_PARTIAL: &str = "```";
    pub const CODE_BLOCK_END: &str = "```";
    pub const CHECKED: &str = "[x]";
    pub const UNCHECKED: &str = "[ ]";
    pub const CHECKED_OUTPUT: &str = "[≡]";
    pub const UNCHECKED_OUTPUT: &str = "[ ]";
}

#[derive(Debug, PartialEq, Clone)]
pub struct CodeBlockLine<'a> {
    pub language: Option<&'a str>,
    pub content: CodeBlockLineContent<'a>,
}

/// Alias for [List] of [CodeBlockLine].
pub type CodeBlockLines<'a> = List<CodeBlockLine<'a>>;

#[derive(Debug, PartialEq, Clone)]
pub enum CodeBlockLineContent<'a> {
    Text(&'a str),
    StartTag,
    EndTag,
}
