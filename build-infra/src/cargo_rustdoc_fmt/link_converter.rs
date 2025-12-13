// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Skip rustdoc formatting - this file contains examples of the formatter's output
#![cfg_attr(rustfmt, rustfmt_skip)]

//! Convert inline markdown links to reference-style links.

use pulldown_cmark::{Event, HeadingLevel, LinkType, Parser, Tag, TagEnd};
use std::{collections::HashMap, fmt::Write as _};

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

    // Extract existing reference definitions BEFORE parsing
    // (pulldown_cmark strips reference definitions, so we must preserve them)
    let (text_without_refs, existing_refs) = extract_reference_definitions(text);

    // Rebuild the markdown with reference-style links (if any inline links found)
    let mut result = if link_info.is_empty() {
        text_without_refs
    } else {
        rebuild_with_text_references(&text_without_refs)
    };

    // Collect all references: both newly converted and pre-existing
    let mut all_refs = Vec::new();

    // Add newly converted inline links as references
    for (link_text, url) in &link_info {
        all_refs.push((link_text.clone(), format!("[{link_text}]: {url}")));
    }

    // Add pre-existing references
    all_refs.extend(existing_refs);

    // If we have any references, aggregate and append them
    if !all_refs.is_empty() {
        // Sort references alphabetically by link name
        all_refs.sort_by(|(name_a, _), (name_b, _)| name_a.cmp(name_b));

        // Append references at bottom with blank line separator for visual clarity
        // This separates content (including reference usages) from reference definitions
        result = result.trim_end().to_string();
        result.push_str("\n\n");
        for (_, ref_line) in &all_refs {
            result.push_str(ref_line);
            result.push('\n');
        }
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
            Event::Start(Tag::Emphasis) | Event::End(TagEnd::Emphasis) => {
                result.push('*');
            }
            Event::Start(Tag::Strong) | Event::End(TagEnd::Strong) => {
                result.push_str("**");
            }
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
            Event::Start(Tag::Heading { level, .. }) => {
                // Ensure blank line before heading for Rust doc style
                if !result.is_empty() && !result.ends_with("\n\n") {
                    if !result.ends_with('\n') {
                        result.push('\n');
                    }
                    result.push('\n');
                }

                // Output correct number of # characters based on heading level
                let level_num = match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                };
                result.push_str(&"#".repeat(level_num));
                result.push(' ');
            }
            _ => {}
        }
    }

    result
}

/// Extract reference definitions from text and return (`text_without_refs`, `references`).
///
/// Scans for existing reference definitions (e.g., `[name]: target`), extracts them,
/// and returns both the text without those definitions and the list of references.
///
/// Returns: (text without reference definitions, `Vec<(link_name, full_reference_line)>`)
fn extract_reference_definitions(text: &str) -> (String, Vec<(String, String)>) {
    if text.is_empty() {
        return (String::new(), Vec::new());
    }

    let mut content_lines = Vec::new();
    let mut references: Vec<(String, String)> = Vec::new();

    // Process each line
    for line in text.lines() {
        // Check if this line is a reference definition: [name]: target
        if let Some(link_name) = parse_reference_definition(line) {
            references.push((link_name, line.to_string()));
        } else {
            content_lines.push(line);
        }
    }

    let content = content_lines.join("\n");
    (content, references)
}

/// Aggregate scattered reference-style link definitions to the bottom of the text.
///
/// Scans for existing reference definitions (e.g., `[name]: target`), extracts them,
/// sorts them alphabetically by link name, and moves them to the bottom with a blank
/// line separator for visual clarity.
///
/// # Example
///
/// Input:
/// ```text
/// Bla bla [link].
/// [link]: crate::Link
///
/// Bla bla [otherlink].
/// [otherlink]: crate::Otherlink
/// ```
///
/// Output:
/// ```text
/// Bla bla [link].
///
/// Bla bla [otherlink].
///
/// [link]: crate::Link
/// [otherlink]: crate::Otherlink
/// ```
#[must_use]
pub fn aggregate_existing_references(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }

    let mut content_lines = Vec::new();
    let mut references: Vec<(String, String)> = Vec::new(); // (link_name, full_line)

    // Process each line
    for line in text.lines() {
        // Check if this line is a reference definition: [name]: target
        if let Some(link_name) = parse_reference_definition(line) {
            references.push((link_name, line.to_string()));
        } else {
            content_lines.push(line);
        }
    }

    // If no references found, return original text
    if references.is_empty() {
        return text.to_string();
    }

    // Sort references alphabetically by link name (case-sensitive)
    references.sort_by(|(name_a, _), (name_b, _)| name_a.cmp(name_b));

    // Rebuild content
    let mut result = content_lines.join("\n");

    // Trim trailing whitespace/newlines
    result = result.trim_end().to_string();

    // Append references with blank line separator for visual clarity
    result.push_str("\n\n");
    for (_, ref_line) in &references {
        result.push_str(ref_line);
        result.push('\n');
    }

    // Remove trailing newline
    result = result.trim_end().to_string();

    result
}

/// Parse a reference definition line and return the link name if valid.
///
/// Matches lines in the format: `[name]: target`
fn parse_reference_definition(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if !trimmed.starts_with('[') {
        return None;
    }

    // Find the closing bracket
    let close_bracket = trimmed.find(']')?;

    // Check if it's followed by a colon
    let after_bracket = &trimmed[close_bracket + 1..];
    if !after_bracket.trim_start().starts_with(':') {
        return None;
    }

    // Extract the link name
    let link_name = &trimmed[1..close_bracket];
    Some(link_name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heading_levels_preserved() {
        let input = "# Level 1\n\n## Level 2\n\n### Level 3\n\nSome content.";
        let output = convert_links(input);
        eprintln!("Input:\n{input}");
        eprintln!("\nOutput:\n{output}");

        assert!(
            output.contains("# Level 1"),
            "Level-1 heading should be preserved"
        );
        assert!(
            output.contains("## Level 2"),
            "Level-2 heading should be preserved"
        );
        assert!(
            output.contains("### Level 3"),
            "Level-3 heading should be preserved"
        );
    }

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
        let input =
            "See [docs](https://example.com) and [more docs](https://example.com).";
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

    #[test]
    fn test_aggregate_scattered_references() {
        let input = "Bla bla [link].\n[link]: crate::Link\n\nBla bla [otherlink].\n[otherlink]: crate::Otherlink";
        let output = aggregate_existing_references(input);
        eprintln!("Input:\n{input}");
        eprintln!("\nOutput:\n{output}");

        // Check that content is preserved
        assert!(output.contains("Bla bla [link]."));
        assert!(output.contains("Bla bla [otherlink]."));

        // Check that references are at the bottom
        let lines: Vec<&str> = output.lines().collect();
        assert!(lines.len() >= 2);
        assert!(lines[lines.len() - 2].contains("[link]: crate::Link"));
        assert!(lines[lines.len() - 1].contains("[otherlink]: crate::Otherlink"));
    }

    #[test]
    fn test_aggregate_alphabetical_sorting() {
        let input = "Text [zebra] and [alpha].\n[zebra]: crate::Z\n[alpha]: crate::A";
        let output = aggregate_existing_references(input);
        eprintln!("Input:\n{input}");
        eprintln!("\nOutput:\n{output}");

        // References should be sorted alphabetically
        let lines: Vec<&str> = output.lines().collect();
        let ref_section = lines
            .iter()
            .skip_while(|l| !l.contains("[alpha]:"))
            .collect::<Vec<_>>();
        assert!(ref_section.len() >= 2);
        assert!(ref_section[0].contains("[alpha]: crate::A"));
        assert!(ref_section[1].contains("[zebra]: crate::Z"));
    }

    #[test]
    fn test_aggregate_no_references() {
        let input = "Just some text without references.";
        let output = aggregate_existing_references(input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_aggregate_blank_line_before_references() {
        let input = "Content [link].\n[link]: target";
        let output = aggregate_existing_references(input);
        eprintln!("Output:\n{output:?}");

        // Should have blank line before references for visual separation
        assert!(output.contains("Content [link].\n\n[link]: target"));
    }

    #[test]
    fn test_parse_reference_definition_valid() {
        assert_eq!(
            parse_reference_definition("[example]: https://example.com"),
            Some("example".to_string())
        );
        assert_eq!(
            parse_reference_definition("[complex name]: target"),
            Some("complex name".to_string())
        );
        assert_eq!(
            parse_reference_definition("  [indented]: value  "),
            Some("indented".to_string())
        );
    }

    #[test]
    fn test_parse_reference_definition_invalid() {
        assert_eq!(parse_reference_definition("Not a reference"), None);
        assert_eq!(parse_reference_definition("[incomplete"), None);
        assert_eq!(parse_reference_definition("[missing colon] value"), None);
        assert_eq!(parse_reference_definition("regular text"), None);
    }

    #[test]
    fn test_aggregate_with_content_after_refs() {
        // Simulates the file where refs are in middle and content + more refs are after.
        let input = r#"Some content.

[ref1]: target1
[ref2]: target2

More content here.

[ref3]: target3"#;
        let output = aggregate_existing_references(input);

        // All refs should be at the bottom, sorted alphabetically.
        let lines: Vec<&str> = output.lines().collect();
        let last_three: Vec<&str> = lines.iter().rev().take(3).copied().collect();
        assert!(last_three.iter().all(|l| l.starts_with('[')));

        // Content should be before refs (with some blank lines from removed refs).
        assert!(output.contains("Some content."));
        assert!(output.contains("More content here."));
    }

    #[test]
    fn test_aggregate_with_code_fence_and_protector() {
        use crate::cargo_rustdoc_fmt::content_protector::ContentProtector;

        // Simulates actual file with code fence (e.g., rust,ignore language tag).
        let input = r#"Content before.

```rust,ignore
// Code here
fn example() {}
```

## References

- [mio issue #1377] - "Polling"

[ref1]: target1
[ref2]: target2

# Entry Point

More content.

[ref3]: target3"#;
        let original = input.to_string();

        // Simulate processor flow.
        let mut protector = ContentProtector::new();
        let protected = protector.protect(input);
        let converted = convert_links(&protected);
        let aggregated = aggregate_existing_references(&converted);
        let restored = protector.restore(&aggregated);

        // Restored should be different from original (refs moved to bottom).
        assert_ne!(
            restored, original,
            "Output should be different from input (refs should be aggregated)"
        );

        // Check that refs are at the bottom after the code fence is restored.
        let lines: Vec<&str> = restored.lines().collect();
        let last_three: Vec<&str> = lines.iter().rev().take(3).copied().collect();
        assert!(
            last_three.iter().all(|l| l.starts_with('[')),
            "Last 3 lines should be references"
        );
    }
}
