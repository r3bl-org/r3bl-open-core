// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Skip rustdoc formatting - this file contains examples of the formatter's output.
#![cfg_attr(rustfmt, rustfmt_skip)]

//! Convert inline markdown links to reference-style links.

use regex::Regex;
use std::sync::LazyLock;

/// Regex to match inline markdown links: `[text](url)`
///
/// Captures:
/// - Group 1: link text (can contain backticks for code)
/// - Group 2: URL
///
/// Does NOT match:
/// - Reference-style links `[text][ref]` or `[text]`
/// - Already existing reference definitions `[text]: url`
static INLINE_LINK_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    // Match [text](url) but not [![ (image links) or [text][ref]
    // The text can contain backticks etc. but NOT nested brackets.
    // URL cannot contain spaces or closing parens (simplified).
    // Excluding `[` from link text prevents pathological cross-line matches where
    // `[200~` in escape sequences chains through other `[` chars to reach a `]`
    // dozens of lines later, duplicating content.
    Regex::new(r"\[([^\[\]]+)\]\(([^)\s]+)\)").expect("Invalid inline link regex")
});

/// Convert inline markdown links to reference-style links.
///
/// This function preserves all original formatting (numbered lists, indentation,
/// etc.) by using regex replacement instead of parsing and rebuilding the markdown.
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

    // Extract existing reference definitions first
    let (text_without_refs, existing_refs) = extract_reference_definitions(text);

    // Collect inline links and their URLs
    let mut link_info: Vec<(String, String)> = Vec::new();
    let mut seen_urls: std::collections::HashSet<String> = std::collections::HashSet::new();

    for caps in INLINE_LINK_REGEX.captures_iter(&text_without_refs) {
        let link_text = caps.get(1).map_or("", |m| m.as_str()).to_string();
        let url = caps.get(2).map_or("", |m| m.as_str()).to_string();

        // Only add unique URLs (first link text wins for duplicates)
        if !seen_urls.contains(&url) {
            seen_urls.insert(url.clone());
            link_info.push((link_text, url));
        }
    }

    // Replace inline links with reference-style links in-place
    let result = INLINE_LINK_REGEX.replace_all(&text_without_refs, |caps: &regex::Captures| {
        let link_text = caps.get(1).map_or("", |m| m.as_str());
        format!("[{link_text}]")
    });

    let mut result = result.to_string();

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
        let input = r"Some content.

[ref1]: target1
[ref2]: target2

More content here.

[ref3]: target3";
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

    #[test]
    fn test_numbered_list_preserved() {
        // Numbered lists should NOT be converted to bullet lists
        let input = r"Steps:

1. First do [this](https://example.com/first)
2. Then do [that](https://example.com/second)
3. Finally [finish](https://example.com/third)";
        let output = convert_links(input);
        eprintln!("Input:\n{input}");
        eprintln!("\nOutput:\n{output}");

        // Numbered list markers should be preserved
        assert!(output.contains("1. First do [this]"), "Numbered list '1.' should be preserved");
        assert!(output.contains("2. Then do [that]"), "Numbered list '2.' should be preserved");
        assert!(output.contains("3. Finally [finish]"), "Numbered list '3.' should be preserved");

        // Should NOT have bullet markers
        assert!(!output.contains("- First"), "Should not convert to bullet list");
    }

    #[test]
    fn test_indentation_preserved() {
        // Multi-line list items with indentation should be preserved
        let input = r"- The framework handles all the mechanics: spawning
  threads, reusing running threads, wake signaling.
- The user implements [traits](https://example.com):
  complete implementations here.";
        let output = convert_links(input);
        eprintln!("Input:\n{input}");
        eprintln!("\nOutput:\n{output}");

        // Indentation should be preserved
        assert!(output.contains("  threads, reusing"), "Indentation should be preserved");
        assert!(output.contains("  complete implementations"), "Indentation should be preserved");
    }

    #[test]
    fn test_multiline_content_preserved() {
        // Content that spans multiple lines should be preserved exactly
        let input = r"Some text with a [link](https://example.com) that
continues on the next line without breaking.

1. **Item one** — description with a [link](https://example.com/one)
   that continues on the next line.

2. **Item two** — another description.";
        let output = convert_links(input);
        eprintln!("Input:\n{input}");
        eprintln!("\nOutput:\n{output}");

        // Line structure should be preserved
        assert!(output.contains("that\ncontinues"), "Newlines in text should be preserved");
        assert!(output.contains("1. **Item one**"), "Numbered list should be preserved");
        assert!(output.contains("2. **Item two**"), "Numbered list should be preserved");
        assert!(output.contains("   that continues"), "Indentation should be preserved");
    }

    #[test]
    fn test_no_cross_line_matching_with_bracket_escape_sequences() {
        // Regression test: `[200~` in escape sequence text must not match a `]`
        // many lines later, causing content duplication. The inline link regex
        // must not span newlines.
        let input = concat!(
            "- **How it works**: Terminal sends pasted text wrapped in escape\n",
            "  sequences (`ESC[200~` text `ESC[201~`)\n",
            "- **Characteristics**: Text arrives as a single chunk\n",
            "\n",
            "Note: Bracketed paste must be enabled via\n",
            "[`EnableBracketedPaste`](crate::PaintRenderOpImplCrossterm::raw_mode_enter) in raw\n",
            "mode.",
        );
        let output = convert_links(input);

        // The inline link should be converted to reference-style
        assert!(
            output.contains("[`EnableBracketedPaste`]"),
            "Inline link should be converted to reference-style"
        );
        assert!(
            output.contains("[`EnableBracketedPaste`]: crate::PaintRenderOpImplCrossterm::raw_mode_enter"),
            "Reference definition should be appended"
        );

        // The escape sequence line must NOT be duplicated or mangled
        assert!(
            output.contains("sequences (`ESC[200~` text `ESC[201~`)"),
            "Escape sequence line should remain intact"
        );

        // Content should appear exactly once (no duplication)
        assert_eq!(
            output.matches("Characteristics").count(),
            1,
            "Content must not be duplicated"
        );
    }
}
