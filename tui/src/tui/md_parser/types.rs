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

/// This corresponds to a single Markdown document, which is produced after a successful parse
/// operation [crate::parse_markdown].
pub type Document<'a> = Vec<Block<'a>>;
/// Alias for [Document].
pub type Blocks<'a> = Vec<Block<'a>>;

/// This roughly corresponds to a single line of text. Each line is made up of one or more
/// [Fragment].
pub type Fragments<'a> = Vec<Fragment<'a>>;
/// Alias for [Fragments].
pub type Line<'a> = Vec<Fragment<'a>>;
/// Alias for Vec<[Line]>.
pub type Lines<'a> = Vec<Line<'a>>;

/// These are blocks of Markdown. Blocks are the top-level elements of a Markdown document. A
/// Markdown document once parsed is turned into a [Vec] of these.
#[derive(Clone, Debug, PartialEq)]
pub enum Block<'a> {
    Heading((Level, Fragments<'a>)),
    OrderedList(Lines<'a>),
    UnorderedList(Lines<'a>),
    Text(Fragments<'a>),
    CodeBlock(CodeBlock<'a>),
    Title(&'a str),
    Tags(Vec<&'a str>),
}

/// These are things that show up in a single line of Markdown text [Fragments]. They do
/// not include other Markdown blocks (like code blocks, lists, headings, etc).
#[derive(Clone, Debug, PartialEq)]
pub enum Fragment<'a> {
    Link((&'a str, &'a str)),
    Image((&'a str, &'a str)),
    InlineCode(&'a str),
    Bold(&'a str),
    BoldItalic(&'a str),
    Italic(&'a str),
    Plain(&'a str),
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Level {
    Heading1 = 1,
    Heading2,
    Heading3,
    Heading4,
    Heading5,
    Heading6,
}

impl From<usize> for Level {
    fn from(size: usize) -> Self {
        match size {
            1 => Level::Heading1,
            2 => Level::Heading2,
            3 => Level::Heading3,
            4 => Level::Heading4,
            5 => Level::Heading5,
            6 => Level::Heading6,
            _ => Level::Heading6,
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
    pub const UNKNOWN_LANGUAGE: &str = "__UNKNOWN_LANGUAGE__";
    pub const SPACE: &str = " ";
    pub const PERIOD: &str = ".";
    pub const UNORDERED_LIST: &str = "-";
    pub const BITALIC_1: &str = "***";
    pub const BITALIC_2: &str = "___";
    pub const BOLD_1: &str = "**";
    pub const BOLD_2: &str = "__";
    pub const ITALIC_1: &str = "*";
    pub const ITALIC_2: &str = "_";
    pub const BACKTICK: &str = "`";
    pub const LEFT_BRACKET: &str = "[";
    pub const RIGHT_BRACKET: &str = "]";
    pub const LEFT_PAREN: &str = "(";
    pub const RIGHT_PAREN: &str = ")";
    pub const LEFT_IMG: &str = "![";
    pub const RIGHT_IMG: &str = "]";
    pub const NEW_LINE: &str = "\n";
    pub const CODE_BLOCK: &str = "```";
}

#[derive(Clone, Debug, PartialEq)]
pub struct CodeBlock<'a> {
    pub language: &'a str,
    pub text: &'a str,
}

mod code_block_impl {
    use super::*;

    impl<'a> From<(&'a str, &'a str)> for CodeBlock<'a> {
        fn from((language, text): (&'a str, &'a str)) -> Self { CodeBlock { language, text } }
    }
}
