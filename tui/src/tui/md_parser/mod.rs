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

//! The main entry point (function) for this Markdown parsing module is [parser#parse_markdown]. It
//! takes a string slice and returns a vector of [MdBlockElement]s.
//!
//! This module contains a fully functional Markdown parser. This parser supports standard Markdown
//! syntax as well as some extensions that are added to make it work w/ R3BL products.
//!
//! Here are some entry points into the codebase.
//!
//! 1. The main function [parse_markdown] that does the parsing of a string slice into a `Document`.
//!    The tests are provided alongside the code itself. And you can follow along to see how other
//!    smaller parsers are used to build up this big one that parses the whole of the Markdown
//!    document.
//! 2. The types [types] that are used to represent the Markdown document model [MdDocument],
//!    [MdBlockElement], [MdLineFragment] and all the other intermediate types & enums required for
//!    parsing.
//! 3. All the parsers related to parsing metadata specific for R3BL applications which are not
//!    standard Markdown can be found in [parser_metadata].
//! 4. All the parsers that are related to parsing the main "blocks" of Markdown, such as order
//!    lists, unordered lists, code blocks, text blocks, heading blocks, can be found [block].
//! 5. All the parsers that are related to parsing a single line of Markdown text, such as links,
//!    bold, italic, etc. can be found [parser_element].

// External use.
pub mod block;
pub mod parser;
pub mod parser_element;
pub mod parser_metadata;
pub mod types;

pub use block::*;
pub use parser::*;
pub use parser_element::*;
pub use parser_metadata::*;
pub use types::*;
