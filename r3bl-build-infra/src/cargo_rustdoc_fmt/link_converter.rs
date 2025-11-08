// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Convert inline markdown links to reference-style links.

use pulldown_cmark::{Event, LinkType, Parser, Tag, TagEnd};
use std::collections::HashMap;
use std::fmt::Write as _;

/// Convert inline markdown links to reference-style links.
///
/// # Example
///
/// Input: `See [docs](https://example.com) here.`
/// Output: `See [docs] here.\n\n[docs]: https://example.com`
#[must_use] 
pub fn convert_links(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }

    let parser = Parser::new(text);
    let mut link_info: Vec<(String, String)> = Vec::new(); // (link_text, url)
    let mut url_to_text: HashMap<String, String> = HashMap::new();

    // First pass: collect link text and URLs
    let mut in_link = false;
    let mut current_text = String::new();
    let mut current_url = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                ..
            }) => {
                if link_type == LinkType::Inline {
                    in_link = true;
                    current_url = dest_url.to_string();
                    current_text.clear();
                }
            }
            Event::End(TagEnd::Link) => {
                if in_link {
                    // Store this link info
                    if !url_to_text.contains_key(&current_url) {
                        url_to_text.insert(current_url.clone(), current_text.clone());
                        link_info.push((current_text.clone(), current_url.clone()));
                    }
                    in_link = false;
                }
            }
            Event::Text(text_content) => {
                if in_link {
                    current_text.push_str(&text_content);
                }
            }
            Event::Code(code) => {
                if in_link {
                    current_text.push('`');
                    current_text.push_str(&code);
                    current_text.push('`');
                }
            }
            _ => {}
        }
    }

    // If no inline links found, return original text
    if link_info.is_empty() {
        return text.to_string();
    }

    // Rebuild the markdown with reference-style links
    let mut result = rebuild_with_text_references(text);

    // Append reference definitions
    if !link_info.is_empty() {
        // Ensure result ends with exactly one newline before adding references
        let result_trimmed = result.trim_end();
        result = result_trimmed.to_string();
        result.push_str("\n\n");
        for (link_text, url) in &link_info {
            let _ = writeln!(result, "[{link_text}]: {url}");
        }
        // Remove trailing newline
        result = result.trim_end().to_string();
    }

    result
}

/// Rebuild markdown text with reference-style links using link text as reference ID.
#[allow(clippy::too_many_lines)]
fn rebuild_with_text_references(text: &str) -> String {
    let parser = Parser::new(text);
    let mut result = String::new();
    let mut in_link = false;
    let mut link_text = String::new();
    let mut current_url = String::new();
    let mut need_newline = false;

    for event in parser {
        match event {
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                ..
            }) => {
                if link_type == LinkType::Inline {
                    in_link = true;
                    current_url = dest_url.to_string();
                    link_text.clear();
                } else {
                    // Keep reference links as-is
                    result.push('[');
                }
            }
            Event::End(TagEnd::Link) => {
                if in_link {
                    // Convert to reference style using link text as reference ID
                    let _ = write!(result, "[{link_text}]");
                    in_link = false;
                    link_text.clear();
                    current_url.clear();
                } else {
                    result.push(']');
                }
            }
            Event::Text(text_content) => {
                if in_link {
                    link_text.push_str(&text_content);
                } else {
                    result.push_str(&text_content);
                }
                need_newline = false;
            }
            Event::Code(code) => {
                if in_link {
                    link_text.push('`');
                    link_text.push_str(&code);
                    link_text.push('`');
                } else {
                    result.push('`');
                    result.push_str(&code);
                    result.push('`');
                }
                need_newline = false;
            }
            Event::SoftBreak => result.push('\n'),
            Event::HardBreak => result.push_str("  \n"),
            Event::Start(Tag::Emphasis) | Event::End(TagEnd::Emphasis) => result.push('*'),
            Event::Start(Tag::Strong) | Event::End(TagEnd::Strong) => result.push_str("**"),
            Event::Start(Tag::List(_)) => {
                if !result.is_empty() && !result.ends_with('\n') {
                    result.push('\n');
                }
                need_newline = false;
            }
            Event::End(TagEnd::List(_)) => {
                if !result.is_empty() && !result.ends_with('\n') {
                    result.push('\n');
                }
                need_newline = true;
            }
            Event::Start(Tag::Item) => {
                if !result.is_empty() && !result.ends_with('\n') {
                    result.push('\n');
                }
                result.push_str("- ");
            }
            Event::End(TagEnd::Item) => {
                need_newline = false;
            }
            Event::Start(Tag::Paragraph) => {
                if !result.is_empty() && need_newline && !result.ends_with("\n\n") {
                    if !result.ends_with('\n') {
                        result.push('\n');
                    }
                    result.push('\n');
                }
                need_newline = false;
            }
            Event::End(TagEnd::Paragraph | TagEnd::Heading(..)) => {
                need_newline = true;
            }
            Event::Start(Tag::Heading { .. }) => {
                if !result.is_empty() && !result.ends_with('\n') {
                    result.push('\n');
                }
                result.push_str("# ");
            }
            _ => {}
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_link_conversion() {
        let input = "See [docs](https://example.com) here.";
        let output = convert_links(input);
        eprintln!("Input: {input:?}");
        eprintln!("Output: {output:?}");
        assert!(output.contains("[docs]"));
        assert!(output.contains("[docs]: https://example.com"));
        assert!(!output.contains("[docs]["));
    }

    #[test]
    fn test_multiple_links() {
        let input = "See [docs](https://example.com) and [Rust](https://rust-lang.org).";
        let output = convert_links(input);
        assert!(output.contains("[docs]"));
        assert!(output.contains("[Rust]"));
        assert!(output.contains("[docs]: https://example.com"));
        assert!(output.contains("[Rust]: https://rust-lang.org"));
    }

    #[test]
    fn test_duplicate_urls() {
        let input = "See [docs](https://example.com) and [more docs](https://example.com).";
        let output = convert_links(input);
        // First link text becomes the reference
        assert!(output.contains("[docs]"));
        assert!(output.contains("[more docs]"));
        // Should only have one reference definition (using first link text)
        assert_eq!(output.matches("[docs]: https://example.com").count(), 1);
        assert_eq!(output.matches("https://example.com").count(), 1);
    }

    #[test]
    fn test_empty_text() {
        let output = convert_links("");
        assert_eq!(output, "");
    }

    #[test]
    fn test_no_links() {
        let input = "This text has no links.";
        let output = convert_links(input);
        assert_eq!(output, input);
    }
}
