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

//! The main entry point (function) for this Markdown parsing module is
//! [parse_markdown()].
//! - It takes a string slice.
//! - And returns a vector of [MdBlock]s.
//!
//! This module contains a fully functional Markdown parser. This parser supports standard
//! Markdown syntax as well as some extensions that are added to make it work w/
//! [R3BL](https://r3bl.com) products.
//!
//! Here are some entry points into the codebase.
//!
//! 1. The main function [parse_markdown()] that does the parsing of a string slice into a
//!    [MdDocument]. The tests are provided alongside the code itself. And you can follow
//!    along to see how other smaller parsers are used to build up this big one that
//!    parses the whole of the Markdown document.
//! 2. The [mod@md_parser_types] contain all the types that are used to represent the
//!    Markdown document model, such as [MdDocument], [MdBlock], [MdLineFragment] and all
//!    the other intermediate types & enums required for parsing.
//! 3. All the parsers related to parsing metadata specific for [R3BL](https://r3bl.com)
//!    applications, which are not standard Markdown can be found in
//!    [mod@parse_metadata_kv] and [mod@parse_metadata_kcsv].
//! 4. All the parsers that are related to parsing the main "blocks" of Markdown, such as
//!    order lists, unordered lists, code blocks, text blocks, heading blocks, can be
//!    found in [mod@block].
//! 5. All the parsers that are related to parsing a single line of Markdown text, such as
//!    links, bold, italic, etc. can be found [mod@fragment].
//!
//! ## Video and blog post on this
//!
//! You can read all about this parser in [this blog post on
//! developerlife.com](https://developerlife.com/2024/06/28/md-parser-rust-from-r3bl-tui/).
//! You can watch a [video](https://youtu.be/SbwvSHZRb1E) about this parser on the YouTube
//! developerlife.com channel.
//!
//! To learn about nom fundamentals, here are some resources:
//! - Tutorial on nom parsing on [developerlife.com](https://developerlife.com/2023/02/20/guide-to-nom-parsing/).
//! - Video on nom parsing on [YouTube developerlife.com channel](https://youtu.be/v3tMwr_ysPg).
//!
//! ## Architecture and parsing order
//!
//! This diagram showcases the order in which the parsers are called and how they are
//! composed together to parse a Markdown document.
//!
//! <!--
//! diagram:
//! https://asciiflow.com/#/share/eJzdlL9qwzAQxl%2Fl0JRChhLo0Gz9M3Rop2YUCNUWsYgsGfkcxxhD6dyhQwh9ltKnyZNUtus0hAYrJaXQQyAZf%2F6d7rN0JdE8FmSsM6WGRPFCWDImJSULSsbnZ6MhJYVbjZoVigW6B0oSK42VWMB6%2BbxePv7T8UKpBojkNAJwlT5Bwm0qWMztLDS5HpxACTsR8wTQAEYCAmOtCHBX0aIacgvdTDHXxengG3331U8LWb0B3IWXygQzmHMrucZ9e4DPGlGiEmzOVSZcmb0xqeV99W3YfJoyJVP0ITu2k%2Fd617F5hpGx3viLVu7HDjkeYAlcO7n3vh%2Fqn8MiwUOpp8wkyIRR%2B9PctMJD2Kk7tujjy30tvHU6f3ZgQj9PrpywPYfe7O62sbr5sFxixIxtZpMh0yJ3Nek6%2B8S9%2F8q0h%2B114vpii71679jVMcgde6u%2FLv%2B6C%2F7eeG1cVCY%2FjnVdUFKR6gNnN4sV)
//! -->
//!
//! ```text
//! priority ┌────────────────────────────────────────────────────────────────────────┐
//!   high   │ parse_markdown() {                map to the correct                   │
//!     │    │   many0(                         ───────────────────►  MdBlock variant │
//!     │    │     parse_title_value()                                  Title         │
//!     │    │     parse_tags_list()                                    Tags          │
//!     │    │     parse_authors_list()                                 Authors       │
//!     │    │     parse_date_value()                                   Date          │
//!     │    │     parse_block_heading_opt_eol()                        Heading       │
//!     │    │     parse_block_smart_list()                             SmartList     │
//!     │    │     parse_block_code()                                   CodeBlock     │
//!     │    │     parse_block_markdown_text_with_or_without_new_line() Text          │
//!     │    │   )                                                                    │
//!     ▼    │ }                                                                      │
//! priority └────────────────────────────────────────────────────────────────────────┘
//!   low
//! ```
//! The parsing strategy in most cases is to parse the most specific thing first and then
//! parse the more general thing later. We often use the existence of `\n` (or `eol`) to
//! decide how far forwards we need to go into the input. And sometimes `\n` doesn't exist
//! and we simply use the entire input (or end of input or `eoi`). You might see functions
//! that have these suffixes in their names. Another term you might see is
//! `with_or_without_new_line` which makes the parsing strategy explicit in the name.
//!
//! The nature of `nom` parsers is to simply error out when they don't match. And leave
//! the `input` untouched, so that another parser have a go at it again. The nature of
//! these parsing functions is kind of recursive in nature. So it's important identify
//! edge and request_shutdown cases up front before diving into the parsing logic. You
//! will see this used in parsers which look for something more specific, if its not
//! found, they error out, and allow less specific parsers to have a go at it, and so on.
//!
//! ## The priority of parsers
//!
//! As we drill down into the implementation further, we see that the parsers are
//! prioritized in the order of their specificity. The most specific parsers are called
//! first and the least specific parsers are called last. This is done to ensure that the
//! most specific parsers get a chance to parse the input first. And if they fail, then
//! the less specific parsers get a chance to parse the input.
//!
//! <!--
//! diagram:
//! https://asciiflow.com/#/share/eJytlFFuwjAMhq8S5QkkHtD2MjhLJCsNBqK6CUpTUYaQpp2h4iB7RDtNT7I0sK1ABYNhVapdJ1%2F%2F2G7X3MgM%2BdgURANOcoWOj%2Fla8FLw8ejleSD4KnhPo2HwPJY%2BBIIvpMsRErIqhUy6dGKXBposLLWfg3XxbgsPBpdA2mCvz9bs3IQwjGXSrIa9juxtFlmM7bVp07wVpk7OMjQ%2Bh8J4TYCWGnVodRB0nRWsWVZX7%2F9VtvmNHkBrRXVV1dVbvd0xSf7OIh4TI3X7cSjkdwUh99KFOsYGF2aCLlfWIaBzYE27zx20cOILtMbv4LS05QtUWpJ%2BxclVWiJV6nUYzC5ipMXNLmdN3X6u7e4Kl3DqQWdy9pAzR1rY3CnzZpqao0oTW4ax9zZkXHu6u7r7%2BSfaMTaxlnr9SFPSq3kYODop4Sl1QVIffgzGnvf2ROO%2BL4cnFz%2FPiyb4hm%2B%2BAFpUbMk%3D)
//! -->
//!
//! ```text
//! parse_block_markdown_text_with_or_without_new_line() {
//!     many0(
//!       parse_inline_fragments_until_eol_or_eoi()
//!         )   │
//!   }         │                                                                 ──map to the correct──►
//!             └─► alt(                                                          MdLineFragment variant
//!                  ▲ parse_fragment_starts_with_underscore_err_on_new_line()      Italic
//!                  │ parse_fragment_starts_with_star_err_on_new_line()            Bold
//!     specialized  │ parse_fragment_starts_with_backtick_err_on_new_line()        InlineCode
//!     parsers ────►│ parse_fragment_starts_with_left_image_err_on_new_line()      Image
//!                  │ parse_fragment_starts_with_left_link_err_on_new_line()       Link
//!                  │ parse_fragment_starts_with_checkbox_into_str()               Plain
//!                  ▼ parse_fragment_starts_with_checkbox_checkbox_into_bool()     Checkbox
//!     catch all────► parse_fragment_plain_text_no_new_line()                      Plain
//!     parser       )
//! ```
//!
//! The last one on the list in the diagram above is
//! [parse_block_markdown_text_with_or_without_new_line()]. Let's zoom into this function
//! and see how it is composed.
//!
//! ## The "catch all" parser, which is the most complicated, and the lowest priority
//!
//! The most complicated parser is the "catch all" parser or the "plain text" parser. This
//! parser is the last one in the chain and it simply consumes the rest of the input and
//! turns it into a `MdBlock::Text`. This parser is the most complicated because it has to
//! deal with all the edge cases and request_shutdown cases that other parsers have not
//! dealt with. Such as special characters like `` ` ``, `*`, `_`, etc. They are all
//! listed here:
//!
//! - If the input does not start with a special char in this [get_sp_char_set_2()], then
//!   this is the "Normal case". In this case the input is split at the first occurrence
//!   of a special char in [get_sp_char_set_3()]. The "before" part is
//!   [MdLineFragment::Plain] and the "after" part is parsed again by a more specific
//!   parser.
//! - If the input starts with a special char in this [get_sp_char_set_2()] and it is not
//!   in the [get_sp_char_set_1()] with only 1 occurrence, then the behavior is different
//!   "Edge case -> Normal case". Otherwise the behavior is "Edge case -> Special case".
//!   - "Edge case -> Normal case" takes all the characters until `\n` or end of input and
//!     turns it into a [MdLineFragment::Plain].
//!   - "Edge case -> Special case" splits the `input` before and after the special char.
//!     The "before" part is turned into a [MdLineFragment::Plain] and the "after" part is
//!     parsed again by a more specific parser.
//!
//! The reason this parser gets called repeatedly is because it is the last one in the
//! chain. Its the lowest priority parser called by
//! [parse_inline_fragments_until_eol_or_eoi()], which itself is called:
//! 1. Repeatedly in a loop by [parse_block_markdown_text_with_or_without_new_line()].
//! 2. And by [parse_block_markdown_text_with_checkbox_policy_with_or_without_new_line()].

// External use.
pub mod atomics;
pub mod block;
pub mod convert_to_plain_text;
pub mod extended;
pub mod fragment;
pub mod md_parser_types;
pub mod parse_markdown;

pub use atomics::*;
pub use block::*;
pub use convert_to_plain_text::*;
pub use extended::*;
pub use fragment::*;
pub use md_parser_types::*;
pub use parse_markdown::*;
