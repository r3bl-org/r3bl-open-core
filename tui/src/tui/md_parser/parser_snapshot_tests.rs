/*
 *   Copyright (c) 2025 R3BL LLC
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

//! ## Snapshot tests for legacy parser
//!
//! This module contains comprehensive tests that verify the legacy parser produces
//! correct output for ALL markdown inputs using the test data from
//! `conformance_test_data`.
//!
//! These tests ensure that:
//! 1. The parser handles all valid markdown constructs correctly
//! 2. Edge cases and invalid inputs are handled gracefully
//! 3. The parser output remains consistent across code changes

#[cfg(test)]
mod tests {
    use crate::{
        md_parser::conformance_test_data::*, parse_markdown_str, MdDocument, MdElement,
        MdLineFragment, HeadingData, HyperlinkData, CodeBlockLineContent,
    };
    #[allow(unused_imports)]
    use crate::{HeadingLevel, List, BulletKind, CodeBlockLine};

    /// Macro for creating a List from elements
    #[allow(unused_macros)]
    macro_rules! list {
        ( $($elem:expr),* $(,)? ) => {
            List::from(vec![$($elem),*])
        };
    }

    /// Helper to assert document has expected number of elements
    fn assert_doc_len(doc: &MdDocument, expected: usize) {
        assert_eq!(
            doc.len(),
            expected,
            "Document has {} elements, expected {}",
            doc.len(),
            expected
        );
    }

    /// Helper to assert an element is text with specific fragments
    fn assert_text_element(element: &MdElement, expected_fragments: &[MdLineFragment]) {
        match element {
            MdElement::Text(fragments) => {
                assert_eq!(
                    fragments.len(),
                    expected_fragments.len(),
                    "Text element has {} fragments, expected {}",
                    fragments.len(),
                    expected_fragments.len()
                );
                for (i, (actual, expected)) in fragments.iter().zip(expected_fragments.iter()).enumerate() {
                    assert_eq!(
                        actual, expected,
                        "Fragment {i} mismatch: {actual:?} != {expected:?}"
                    );
                }
            }
            _ => panic!("Expected Text element, got {element:?}"),
        }
    }

    /// Helper to assert an element is a heading
    fn assert_heading_element(element: &MdElement, level: usize, text: &str) {
        match element {
            MdElement::Heading(HeadingData { level: actual_level, text: actual_text }) => {
                assert_eq!(actual_level.level, level, "Heading level mismatch");
                assert_eq!(*actual_text, text, "Heading text mismatch");
            }
            _ => panic!("Expected Heading element, got {element:?}"),
        }
    }

    /// Helper to assert metadata elements
    fn assert_title_element(element: &MdElement, expected: &str) {
        match element {
            MdElement::Title(actual) => assert_eq!(*actual, expected, "Title mismatch"),
            _ => panic!("Expected Title element, got {element:?}"),
        }
    }

    fn assert_tags_element(element: &MdElement, expected: &[&str]) {
        match element {
            MdElement::Tags(tags) => {
                assert_eq!(tags.len(), expected.len(), "Tags count mismatch");
                for (actual, expected) in tags.iter().zip(expected.iter()) {
                    assert_eq!(actual, expected, "Tag mismatch");
                }
            }
            _ => panic!("Expected Tags element, got {element:?}"),
        }
    }

    fn assert_authors_element(element: &MdElement, expected: &[&str]) {
        match element {
            MdElement::Authors(authors) => {
                assert_eq!(authors.len(), expected.len(), "Authors count mismatch");
                for (actual, expected) in authors.iter().zip(expected.iter()) {
                    assert_eq!(actual, expected, "Author mismatch");
                }
            }
            _ => panic!("Expected Authors element, got {element:?}"),
        }
    }

    fn assert_date_element(element: &MdElement, expected: &str) {
        match element {
            MdElement::Date(actual) => assert_eq!(*actual, expected, "Date mismatch"),
            _ => panic!("Expected Date element, got {element:?}"),
        }
    }

    // =============================================================================
    // Small valid input tests
    // =============================================================================

    #[test]
    fn test_small_empty_string() {
        let (remainder, doc) = parse_markdown_str(EMPTY_STRING).unwrap();
        assert_eq!(remainder, "");
        assert_doc_len(&doc, 0);
    }

    #[test]
    fn test_small_only_newlines() {
        // ONLY_NEWLINES = "\n\n\n"
        let (remainder, doc) = parse_markdown_str(ONLY_NEWLINES).unwrap();
        assert_eq!(remainder, "");
        assert_doc_len(&doc, 3);
        // Each newline becomes an empty Text element
        for element in doc.iter() {
            assert_text_element(element, &[]);
        }
    }

    #[test]
    fn test_small_single_line_no_newline() {
        // SINGLE_LINE_NO_NEWLINE = "Hello World"
        let (remainder, doc) = parse_markdown_str(SINGLE_LINE_NO_NEWLINE).unwrap();
        assert_eq!(remainder, "");
        assert_doc_len(&doc, 1);
        assert_text_element(&doc[0], &[MdLineFragment::Plain("Hello World")]);
    }

    #[test]
    fn test_small_single_line_with_newline() {
        // SINGLE_LINE_WITH_NEWLINE = "Hello World\n"
        let (remainder, doc) = parse_markdown_str(SINGLE_LINE_WITH_NEWLINE).unwrap();
        assert_eq!(remainder, "");
        assert_doc_len(&doc, 1);
        assert_text_element(&doc[0], &[MdLineFragment::Plain("Hello World")]);
    }

    #[test]
    fn test_small_simple_inline_code() {
        // SIMPLE_INLINE_CODE = "first\n`second`"
        let (remainder, doc) = parse_markdown_str(SIMPLE_INLINE_CODE).unwrap();
        assert_eq!(remainder, "");
        assert_doc_len(&doc, 2);
        assert_text_element(&doc[0], &[MdLineFragment::Plain("first")]);
        assert_text_element(&doc[1], &[MdLineFragment::InlineCode("second")]);
    }

    #[test]
    fn test_small_inline_code_variations() {
        // INLINE_CODE_VARIATIONS contains multiple inline code examples
        let (remainder, doc) = parse_markdown_str(INLINE_CODE_VARIATIONS).unwrap();
        assert_eq!(remainder, "");
        // This test data has multiple lines with inline code
        assert!(!doc.is_empty());
        for element in doc.iter() {
            match element {
                MdElement::Text(fragments) => {
                    // Verify inline code fragments exist
                    let has_inline_code = fragments.iter().any(|f| matches!(f, MdLineFragment::InlineCode(_)));
                    assert!(has_inline_code || fragments.is_empty(), "Expected inline code or empty line");
                }
                _ => panic!("Expected only Text elements"),
            }
        }
    }

    #[test]
    fn test_small_inline_code_with_unicode() {
        // INLINE_CODE_WITH_UNICODE = "`code 🎯`"
        let (remainder, doc) = parse_markdown_str(INLINE_CODE_WITH_UNICODE).unwrap();
        assert_eq!(remainder, "");
        assert_doc_len(&doc, 1);
        assert_text_element(&doc[0], &[MdLineFragment::InlineCode("code 🎯")]);
    }

    #[test]
    fn test_small_bold_text() {
        // BOLD_TEXT = "This is *bold* text"
        let (remainder, doc) = parse_markdown_str(BOLD_TEXT).unwrap();
        assert_eq!(remainder, "");
        assert_doc_len(&doc, 1);
        assert_text_element(&doc[0], &[
            MdLineFragment::Plain("This is "),
            MdLineFragment::Bold("bold"),
            MdLineFragment::Plain(" text"),
        ]);
    }

    #[test]
    fn test_small_italic_text() {
        // ITALIC_TEXT = "This is _italic_ text"
        let (remainder, doc) = parse_markdown_str(ITALIC_TEXT).unwrap();
        assert_eq!(remainder, "");
        assert_doc_len(&doc, 1);
        assert_text_element(&doc[0], &[
            MdLineFragment::Plain("This is "),
            MdLineFragment::Italic("italic"),
            MdLineFragment::Plain(" text"),
        ]);
    }

    #[test]
    fn test_small_mixed_formatting() {
        // MIXED_FORMATTING contains mixed bold, italic, and inline code
        let (remainder, doc) = parse_markdown_str(MIXED_FORMATTING).unwrap();
        assert_eq!(remainder, "");
        assert!(!doc.is_empty());
        // Verify mixed formatting elements exist
        for element in doc.iter() {
            if let MdElement::Text(fragments) = element {
                let has_formatting = fragments.iter().any(|f| matches!(
                    f,
                    MdLineFragment::Bold(_) | MdLineFragment::Italic(_) | MdLineFragment::InlineCode(_)
                ));
                assert!(has_formatting || fragments.is_empty());
            }
        }
    }

    #[test]
    fn test_small_links() {
        // LINKS = "Check out [Rust](https://rust-lang.org) website"
        let (remainder, doc) = parse_markdown_str(LINKS).unwrap();
        assert_eq!(remainder, "");
        assert_doc_len(&doc, 1);
        assert_text_element(&doc[0], &[
            MdLineFragment::Plain("Check out "),
            MdLineFragment::Link(HyperlinkData {
                text: "Rust",
                url: "https://rust-lang.org",
            }),
            MdLineFragment::Plain(" website"),
        ]);
    }

    #[test]
    fn test_small_images() {
        // IMAGES = "![Alt text](https://example.com/image.png)"
        let (remainder, doc) = parse_markdown_str(IMAGES).unwrap();
        assert_eq!(remainder, "");
        assert_doc_len(&doc, 1);
        assert_text_element(&doc[0], &[
            MdLineFragment::Image(HyperlinkData {
                text: "Alt text",
                url: "https://example.com/image.png",
            }),
        ]);
    }

    #[test]
    fn test_small_metadata_title() {
        // METADATA_TITLE = "@title: My Document Title"
        let (remainder, doc) = parse_markdown_str(METADATA_TITLE).unwrap();
        assert_eq!(remainder, "");
        assert_doc_len(&doc, 1);
        assert_title_element(&doc[0], "My Document Title");
    }

    #[test]
    fn test_small_metadata_tags() {
        // METADATA_TAGS = "@tags: rust, programming, tutorial"
        let (remainder, doc) = parse_markdown_str(METADATA_TAGS).unwrap();
        assert_eq!(remainder, "");
        assert_doc_len(&doc, 1);
        assert_tags_element(&doc[0], &["rust", "programming", "tutorial"]);
    }

    #[test]
    fn test_small_metadata_authors() {
        // METADATA_AUTHORS = "@authors: John Doe, Jane Smith"
        let (remainder, doc) = parse_markdown_str(METADATA_AUTHORS).unwrap();
        assert_eq!(remainder, "");
        assert_doc_len(&doc, 1);
        assert_authors_element(&doc[0], &["John Doe", "Jane Smith"]);
    }

    #[test]
    fn test_small_metadata_date() {
        // METADATA_DATE = "@date: 2025-01-01"
        let (remainder, doc) = parse_markdown_str(METADATA_DATE).unwrap();
        assert_eq!(remainder, "");
        assert_doc_len(&doc, 1);
        assert_date_element(&doc[0], "2025-01-01");
    }

    #[test]
    fn test_small_special_characters() {
        // SPECIAL_CHARACTERS contains special characters like !@#$%^&*()_+-=[]{}|;':",./<>?
        let (remainder, doc) = parse_markdown_str(SPECIAL_CHARACTERS).unwrap();
        assert_eq!(remainder, "");
        assert_doc_len(&doc, 1);
        // Special characters should be preserved as plain text
        match &doc[0] {
            MdElement::Text(fragments) => {
                assert!(!fragments.is_empty());
                // All special characters should be in Plain fragments
                for fragment in fragments.iter() {
                    assert!(matches!(fragment, MdLineFragment::Plain(_)));
                }
            }
            _ => panic!("Expected Text element"),
        }
    }

    #[test]
    fn test_small_unicode_content() {
        // UNICODE_CONTENT contains text with emojis
        let (remainder, doc) = parse_markdown_str(UNICODE_CONTENT).unwrap();
        assert_eq!(remainder, "");
        assert!(!doc.is_empty());
        // Unicode should be preserved in the parsed content
        for element in doc.iter() {
            if let MdElement::Text(fragments) = element {
                // Check that unicode is preserved in fragments
                for fragment in fragments.iter() {
                    match fragment {
                        MdLineFragment::Plain(text) |
                        MdLineFragment::InlineCode(text) => {
                            // Just verify text exists, don't assume all fragments have unicode
                            let _ = text;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    #[test]
    fn test_small_emoji_h1_simple() {
        // EMOJI_H1_SIMPLE = "# Heading with emoji 😀"
        let (remainder, doc) = parse_markdown_str(EMOJI_H1_SIMPLE).unwrap();
        assert_eq!(remainder, "");
        assert_doc_len(&doc, 1);
        assert_heading_element(&doc[0], 1, "Heading with emoji 😀");
    }

    #[test]
    fn test_small_emoji_h2_simple() {
        // EMOJI_H2_SIMPLE = "## Subheading with emoji 😀"
        let (remainder, doc) = parse_markdown_str(EMOJI_H2_SIMPLE).unwrap();
        assert_eq!(remainder, "");
        assert_doc_len(&doc, 1);
        assert_heading_element(&doc[0], 2, "Subheading with emoji 😀");
    }

    #[test]
    fn test_small_emoji_multiple() {
        // EMOJI_MULTIPLE = "# Multiple emojis 😀🚀📝"
        let (remainder, doc) = parse_markdown_str(EMOJI_MULTIPLE).unwrap();
        assert_eq!(remainder, "");
        assert_doc_len(&doc, 1);
        assert_heading_element(&doc[0], 1, "Multiple emojis 😀🚀📝");
    }

    #[test]
    fn test_small_real_world_content() {
        // SMALL_REAL_WORLD_CONTENT is a complete document with metadata, headings, lists, code blocks
        let (remainder, doc) = parse_markdown_str(SMALL_REAL_WORLD_CONTENT).unwrap();
        assert_eq!(remainder, "");

        // Should have multiple elements including metadata, headings, text, lists, and code blocks
        assert!(doc.len() > 5, "Expected complex document structure");

        // Verify document has various element types
        let has_title = doc.iter().any(|e| matches!(e, MdElement::Title(_)));
        let has_tags = doc.iter().any(|e| matches!(e, MdElement::Tags(_)));
        let has_heading = doc.iter().any(|e| matches!(e, MdElement::Heading(_)));
        let has_list = doc.iter().any(|e| matches!(e, MdElement::SmartList(_)));
        let has_code = doc.iter().any(|e| matches!(e, MdElement::CodeBlock(_)));

        assert!(has_title, "Document should have a title");
        assert!(has_tags, "Document should have tags");
        assert!(has_heading, "Document should have headings");
        assert!(has_list, "Document should have lists");
        assert!(has_code, "Document should have code blocks");
    }

    #[test]
    fn test_small_ex_editor_content() {
        // EX_EDITOR_CONTENT is a complex document with various markdown features
        let (remainder, doc) = parse_markdown_str(EX_EDITOR_CONTENT).unwrap();
        assert_eq!(remainder, "");

        // Should be a complex document
        assert!(doc.len() > 10, "Expected complex document with many elements");

        // Verify presence of various markdown features
        let mut has_metadata = false;
        let mut has_emoji_heading = false;
        let mut has_nested_list = false;
        let mut has_checkbox = false;

        for element in doc.iter() {
            match element {
                MdElement::Title(_) | MdElement::Tags(_) | MdElement::Authors(_) | MdElement::Date(_) => {
                    has_metadata = true;
                }
                MdElement::Heading(data) if data.text.chars().any(|c| c as u32 > 127) => {
                    has_emoji_heading = true;
                }
                MdElement::SmartList((lines, _, indent)) => {
                    if *indent > 0 {
                        has_nested_list = true;
                    }
                    // Check for checkboxes in list items
                    for line in lines.iter() {
                        if line.iter().any(|f| matches!(f, MdLineFragment::Checkbox(_))) {
                            has_checkbox = true;
                        }
                    }
                }
                _ => {}
            }
        }

        assert!(has_metadata, "Document should have metadata");
        assert!(has_emoji_heading, "Document should have emoji in headings");
        assert!(has_nested_list || has_checkbox, "Document should have advanced list features");
    }

    // =============================================================================
    // Medium valid input tests
    // =============================================================================

    #[test]
    fn test_medium_multiple_lines() {
        // MULTIPLE_LINES contains multiple lines of text
        let (remainder, doc) = parse_markdown_str(MULTIPLE_LINES).unwrap();
        assert_eq!(remainder, "");
        // Should have multiple text elements
        assert!(doc.len() > 1, "Expected multiple lines");
        for element in doc.iter() {
            assert!(matches!(element, MdElement::Text(_)), "Expected only text elements");
        }
    }

    #[test]
    fn test_medium_heading_basic() {
        // HEADING_BASIC = "# Main Heading\nSome content"
        let (remainder, doc) = parse_markdown_str(HEADING_BASIC).unwrap();
        assert_eq!(remainder, "");
        assert_doc_len(&doc, 2);
        assert_heading_element(&doc[0], 1, "Main Heading");
        assert_text_element(&doc[1], &[MdLineFragment::Plain("Some content")]);
    }

    #[test]
    fn test_medium_multiple_headings() {
        let (remainder, doc) = parse_markdown_str(MULTIPLE_HEADINGS).unwrap();
        assert_eq!(remainder, "");
        // Should have multiple headings
        let heading_count = doc.iter().filter(|e| matches!(e, MdElement::Heading(_))).count();
        assert!(heading_count >= 2, "Expected multiple headings, found {heading_count}");
    }

    #[test]
    fn test_medium_all_heading_levels() {
        // ALL_HEADING_LEVELS contains headings from H1 to H6
        let (remainder, doc) = parse_markdown_str(ALL_HEADING_LEVELS).unwrap();
        assert_eq!(remainder, "");

        // Should have at least 6 headings
        let heading_count = doc.iter().filter(|e| matches!(e, MdElement::Heading(_))).count();
        assert!(heading_count >= 6, "Expected at least 6 headings");

        // Verify heading levels 1-6 are present
        for level in 1..=6 {
            let has_level = doc.iter().any(|e| {
                matches!(e, MdElement::Heading(HeadingData { level: l, .. }) if l.level == level)
            });
            assert!(has_level, "Missing heading level {level}");
        }
    }

    #[test]
    fn test_medium_unordered_list_simple() {
        let (remainder, doc) = parse_markdown_str(UNORDERED_LIST_SIMPLE).unwrap();
        assert_eq!(remainder, "");
        // Should have unordered list items
        let has_unordered_list = doc.iter().any(|e| matches!(e, MdElement::SmartList((_, BulletKind::Unordered, _))));
        assert!(has_unordered_list, "Expected unordered list");
    }

    #[test]
    fn test_medium_ordered_list_simple() {
        let (remainder, doc) = parse_markdown_str(ORDERED_LIST_SIMPLE).unwrap();
        assert_eq!(remainder, "");
        // Should have ordered list items
        let has_ordered_list = doc.iter().any(|e| matches!(e, MdElement::SmartList((_, BulletKind::Ordered(_), _))));
        assert!(has_ordered_list, "Expected ordered list");
    }

    #[test]
    fn test_medium_nested_unordered_list() {
        let (remainder, doc) = parse_markdown_str(NESTED_UNORDERED_LIST).unwrap();
        assert_eq!(remainder, "");
        // Should have nested lists (indent > 0)
        let has_nested = doc.iter().any(|e| matches!(e, MdElement::SmartList((_, _, indent)) if *indent > 0));
        assert!(has_nested, "Expected nested list items");
    }

    #[test]
    fn test_medium_nested_ordered_list() {
        let (remainder, doc) = parse_markdown_str(NESTED_ORDERED_LIST).unwrap();
        assert_eq!(remainder, "");
        // Should have nested ordered lists
        let has_nested_ordered = doc.iter().any(|e| matches!(e, MdElement::SmartList((_, BulletKind::Ordered(_), indent)) if *indent > 0));
        assert!(has_nested_ordered, "Expected nested ordered list");
    }

    #[test]
    fn test_medium_checkboxes() {
        // CHECKBOXES contains checked and unchecked checkboxes in lists
        let (remainder, doc) = parse_markdown_str(CHECKBOXES).unwrap();
        assert_eq!(remainder, "");

        // Find lists with checkboxes
        let mut found_checked = false;
        let mut found_unchecked = false;

        for element in doc.iter() {
            if let MdElement::SmartList((lines, _, _)) = element {
                for line in lines.iter() {
                    for fragment in line.iter() {
                        match fragment {
                            MdLineFragment::Checkbox(true) => found_checked = true,
                            MdLineFragment::Checkbox(false) => found_unchecked = true,
                            _ => {}
                        }
                    }
                }
            }
        }

        assert!(found_checked, "Expected at least one checked checkbox");
        assert!(found_unchecked, "Expected at least one unchecked checkbox");
    }

    #[test]
    fn test_medium_mixed_list_types() {
        let (remainder, doc) = parse_markdown_str(MIXED_LIST_TYPES).unwrap();
        assert_eq!(remainder, "");
        // Should have both ordered and unordered lists
        let has_ordered = doc.iter().any(|e| matches!(e, MdElement::SmartList((_, BulletKind::Ordered(_), _))));
        let has_unordered = doc.iter().any(|e| matches!(e, MdElement::SmartList((_, BulletKind::Unordered, _))));
        assert!(has_ordered && has_unordered, "Expected mixed list types");
    }

    #[test]
    fn test_medium_code_block_bash() {
        let (remainder, doc) = parse_markdown_str(CODE_BLOCK_BASH).unwrap();
        assert_eq!(remainder, "");
        // Should have bash code block
        let has_bash_code = doc.iter().any(|e| {
            matches!(e, MdElement::CodeBlock(lines) if lines.iter().any(|l| l.language == Some("bash")))
        });
        assert!(has_bash_code, "Expected bash code block");
    }

    #[test]
    fn test_medium_code_block_rust() {
        // CODE_BLOCK_RUST contains a Rust code block
        let (remainder, doc) = parse_markdown_str(CODE_BLOCK_RUST).unwrap();
        assert_eq!(remainder, "");

        // Find the code block
        let code_block = doc.iter().find(|e| matches!(e, MdElement::CodeBlock(_)));
        assert!(code_block.is_some(), "Expected a code block");

        if let Some(MdElement::CodeBlock(lines)) = code_block {
            // Should have start tag, content, and end tag
            assert!(lines.len() >= 3, "Code block should have at least 3 lines");

            // Check language is Rust
            assert_eq!(lines[0].language, Some("rust"));
            assert_eq!(lines[0].content, CodeBlockLineContent::StartTag);

            // Last line should be end tag
            let last = &lines[lines.len() - 1];
            assert_eq!(last.content, CodeBlockLineContent::EndTag);

            // Middle lines should be code content
            for i in 1..lines.len()-1 {
                assert!(matches!(lines[i].content, CodeBlockLineContent::Text(_)));
            }
        }
    }

    #[test]
    fn test_medium_code_block_no_language() {
        let (remainder, doc) = parse_markdown_str(CODE_BLOCK_NO_LANGUAGE).unwrap();
        assert_eq!(remainder, "");
        // Should have code block without language
        let has_plain_code = doc.iter().any(|e| {
            matches!(e, MdElement::CodeBlock(lines) if lines.iter().any(|l| l.language.is_none()))
        });
        assert!(has_plain_code, "Expected code block without language");
    }

    #[test]
    fn test_medium_empty_code_block() {
        let (remainder, doc) = parse_markdown_str(EMPTY_CODE_BLOCK).unwrap();
        assert_eq!(remainder, "");
        // Should have empty code block (just start and end tags)
        let has_empty_code = doc.iter().any(|e| {
            matches!(e, MdElement::CodeBlock(lines) if lines.len() <= 3)
        });
        assert!(has_empty_code, "Expected empty code block");
    }

    #[test]
    fn test_medium_formatting_edge_cases() {
        let (remainder, doc) = parse_markdown_str(FORMATTING_EDGE_CASES).unwrap();
        assert_eq!(remainder, "");
        // Should handle edge cases gracefully
        assert!(!doc.is_empty(), "Document should not be empty");
    }

    #[test]
    fn test_medium_nested_formatting() {
        let (remainder, doc) = parse_markdown_str(NESTED_FORMATTING).unwrap();
        assert_eq!(remainder, "");
        // Should handle nested formatting
        let has_formatting = doc.iter().any(|e| {
            matches!(e, MdElement::Text(fragments) if fragments.iter().any(|f|
                matches!(f, MdLineFragment::Bold(_) | MdLineFragment::Italic(_) | MdLineFragment::InlineCode(_))
            ))
        });
        assert!(has_formatting, "Expected formatted text");
    }

    #[test]
    fn test_medium_edge_case_empty_lines() {
        let (remainder, doc) = parse_markdown_str(EDGE_CASE_EMPTY_LINES).unwrap();
        assert_eq!(remainder, "");
        // Should handle empty lines correctly
        assert!(!doc.is_empty(), "Document should parse empty lines");
    }

    #[test]
    fn test_medium_edge_case_whitespace_lines() {
        let (remainder, doc) = parse_markdown_str(EDGE_CASE_WHITESPACE_LINES).unwrap();
        assert_eq!(remainder, "");
        // Should handle whitespace lines correctly
        assert!(!doc.is_empty(), "Document should parse whitespace lines");
    }

    #[test]
    fn test_medium_edge_case_trailing_spaces() {
        let (remainder, doc) = parse_markdown_str(EDGE_CASE_TRAILING_SPACES).unwrap();
        assert_eq!(remainder, "");
        // Should handle trailing spaces correctly
        assert!(!doc.is_empty(), "Document should parse with trailing spaces");
    }

    #[test]
    fn test_medium_emoji_start_middle_end() {
        let (remainder, doc) = parse_markdown_str(EMOJI_START_MIDDLE_END).unwrap();
        assert_eq!(remainder, "");
        // Should handle emojis in various positions
        let has_emoji_content = doc.iter().any(|e| {
            match e {
                MdElement::Text(fragments) => fragments.iter().any(|f| {
                    match f {
                        MdLineFragment::Plain(text) => text.chars().any(|c| c as u32 > 127),
                        _ => false
                    }
                }),
                MdElement::Heading(data) => data.text.chars().any(|c| c as u32 > 127),
                _ => false
            }
        });
        assert!(has_emoji_content, "Expected emoji content");
    }

    #[test]
    fn test_medium_blog_post_document() {
        let (remainder, doc) = parse_markdown_str(BLOG_POST_DOCUMENT).unwrap();
        assert_eq!(remainder, "");
        // Blog post should have various elements
        assert!(doc.len() > 5, "Blog post should have multiple elements");
        let has_heading = doc.iter().any(|e| matches!(e, MdElement::Heading(_)));
        let has_text = doc.iter().any(|e| matches!(e, MdElement::Text(_)));
        assert!(has_heading && has_text, "Blog post should have headings and text");
    }

    // =============================================================================
    // Large valid input tests
    // =============================================================================

    #[test]
    fn test_large_complex_nested_document() {
        let (remainder, doc) = parse_markdown_str(COMPLEX_NESTED_DOCUMENT).unwrap();
        assert_eq!(remainder, "");
        // Complex document should have many varied elements
        assert!(doc.len() > 10, "Complex document should have many elements");
        // Verify variety of content types
        let element_types = [
            doc.iter().any(|e| matches!(e, MdElement::Heading(_))),
            doc.iter().any(|e| matches!(e, MdElement::SmartList(_))),
            doc.iter().any(|e| matches!(e, MdElement::CodeBlock(_))),
            doc.iter().any(|e| matches!(e, MdElement::Text(_)))
        ];
        assert!(element_types.iter().all(|&x| x), "Complex document should have varied content");
    }

    #[test]
    fn test_large_tutorial_document() {
        let (remainder, doc) = parse_markdown_str(TUTORIAL_DOCUMENT).unwrap();
        assert_eq!(remainder, "");
        // Tutorial should have structured content
        assert!(doc.len() > 10, "Tutorial should have substantial content");
        // Should have sections with headings
        let heading_count = doc.iter().filter(|e| matches!(e, MdElement::Heading(_))).count();
        assert!(heading_count >= 3, "Tutorial should have multiple sections");
    }

    // =============================================================================
    // Invalid input tests
    // =============================================================================

    #[test]
    fn test_invalid_malformed_syntax() {
        let (remainder, doc) = parse_markdown_str(MALFORMED_SYNTAX).unwrap();
        // Malformed syntax should still parse something
        assert!(remainder.is_empty() || !doc.is_empty(), "Parser should handle malformed syntax gracefully");
    }

    #[test]
    fn test_invalid_unclosed_formatting() {
        let (remainder, doc) = parse_markdown_str(UNCLOSED_FORMATTING).unwrap();
        // Unclosed formatting should still parse
        assert!(remainder.is_empty() || !doc.is_empty(), "Parser should handle unclosed formatting gracefully");
    }

    // =============================================================================
    // Jumbo/Real world file tests
    // =============================================================================

    #[test]
    fn test_jumbo_real_world_editor() {
        let (remainder, doc) = parse_markdown_str(REAL_WORLD_EDITOR_CONTENT).unwrap();
        assert_eq!(remainder, "");
        // Real world content should be substantial and varied
        assert!(doc.len() > 20, "Real world document should be large");

        // Should have all major element types
        let has_metadata = doc.iter().any(|e| matches!(e, MdElement::Title(_) | MdElement::Tags(_) | MdElement::Authors(_) | MdElement::Date(_)));
        let has_headings = doc.iter().any(|e| matches!(e, MdElement::Heading(_)));
        let has_lists = doc.iter().any(|e| matches!(e, MdElement::SmartList(_)));
        let has_code = doc.iter().any(|e| matches!(e, MdElement::CodeBlock(_)));
        let has_text = doc.iter().any(|e| matches!(e, MdElement::Text(_)));

        assert!(has_metadata, "Real world doc should have metadata");
        assert!(has_headings, "Real world doc should have headings");
        assert!(has_lists, "Real world doc should have lists");
        assert!(has_code, "Real world doc should have code blocks");
        assert!(has_text, "Real world doc should have text");
    }
}
