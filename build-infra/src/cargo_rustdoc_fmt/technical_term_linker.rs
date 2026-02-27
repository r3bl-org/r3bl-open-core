// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// rustdoc-fmt: skip

//! Upgrades known terms in rustdoc blocks to backticked+linked form.
//!
//! For each known term in the registry, this module:
//! 1. Finds plain-text or backticked-only occurrences
//! 2. Upgrades ALL occurrences to `` [`Term`] `` form
//! 3. Adds or corrects link target definitions at the bottom of the block

use crate::cargo_rustdoc_fmt::technical_term_dictionary::TechnicalTermDictionary;
use std::collections::BTreeSet;

/// Returns the linked form of a term for use in doc comment text.
///
/// - Plain term `"CSI"` → `` [`CSI`] ``
/// - Compound term `` "[`CSI` spec]" `` → `[`[`CSI`]` spec]`
///
/// [`CSI` spec]: https://en.wikipedia.org/wiki/ANSI_escape_code#CSI
/// [`CSI`]: crate::CsiSequence
fn linked_form(term: &str) -> String {
    if term.contains('`') {
        format!("[{term}]")
    } else {
        format!("[`{term}`]")
    }
}

/// Returns a reference definition line for a term.
///
/// - Plain term `"CSI"` → `` [`CSI`]: target ``
/// - Compound term `` "[`CSI` spec]" `` → `[`[`CSI`]` spec]: target`
///
/// [`CSI` spec]: https://en.wikipedia.org/wiki/ANSI_escape_code#CSI
/// [`CSI`]: crate::CsiSequence
fn ref_def_line(term: &str, target: &str) -> String {
    if term.contains('`') {
        format!("[{term}]: {target}")
    } else {
        format!("[`{term}`]: {target}")
    }
}

/// Upgrades known terms in a doc block to backticked+linked form.
///
/// Links ALL occurrences of each known term (not just the first).
/// Returns the modified text, or the original if no changes were needed.
///
/// Handles two kinds of terms:
/// - **Plain terms** (e.g., `"CSI"`): upgrades backticked or plain text to `` [`CSI`] ``
///   linked form
/// - **Compound terms** with backticks (e.g., `` [`CSI` spec] ``): wraps with brackets to
///   produce linked compound form
#[must_use]
pub fn link_known_terms(text: &str, registry: &TechnicalTermDictionary) -> String {
    if text.is_empty() || registry.is_empty() {
        return text.to_string();
    }

    // Split text into content lines and existing reference definitions.
    let (content, existing_refs) = split_off_ref_defs(text);

    // Build a map of existing ref def terms to (target, original_line).
    // Preserving the original line is important for multi-line ref defs that
    // rustfmt split across two lines - we must keep the multi-line format
    // for idempotency (otherwise single-line → rustfmt splits → single-line
    // on next run → infinite cycle).
    let mut ref_def_map: std::collections::HashMap<String, (String, String)> =
        std::collections::HashMap::new();
    for line in &existing_refs {
        if let Some((term, target)) = parse_ref_def(line) {
            ref_def_map.insert(term, (target, line.clone()));
        }
    }

    // Track which terms we linked (need ref defs at the bottom).
    let mut linked_terms: BTreeSet<String> = BTreeSet::new();

    // Collect existing linked terms from content (already in linked form).
    let terms = registry.terms_longest_first();
    for (term, _entry) in &terms {
        if content.contains(&linked_form(term)) {
            linked_terms.insert(term.to_string());
        }
    }

    // Process content line by line, tracking code fence, TOC block, and
    // multi-line inline code span state.
    let mut result_lines: Vec<String> = Vec::new();
    let mut inside_code_fence = false;
    let mut inside_toc_block = false;
    let mut pending_backtick_close: usize = 0;
    for line in content.lines() {
        // Track code fence boundaries.
        if line.trim_start().starts_with("```") {
            inside_code_fence = !inside_code_fence;
            result_lines.push(line.to_string());
            continue;
        }

        // Skip content inside code fences.
        if inside_code_fence {
            result_lines.push(line.to_string());
            continue;
        }

        // Track TOC block boundaries.
        if line.contains("<!-- TOC") && !line.contains("<!-- /TOC") {
            inside_toc_block = true;
            result_lines.push(line.to_string());
            continue;
        }
        if line.contains("<!-- /TOC") {
            inside_toc_block = false;
            result_lines.push(line.to_string());
            continue;
        }

        // Skip content inside TOC blocks.
        if inside_toc_block {
            result_lines.push(line.to_string());
            continue;
        }

        // Handle continuation of multi-line inline code span.
        if pending_backtick_close > 0 {
            if has_closing_backtick_sequence(line, pending_backtick_close) {
                pending_backtick_close = 0;
            }
            result_lines.push(line.to_string());
            continue;
        }

        // Check if this line opens a multi-line inline code span.
        if let Some(tick_count) = find_unclosed_backtick_opening(line) {
            pending_backtick_close = tick_count;
            result_lines.push(line.to_string());
            continue;
        }

        let mut modified_line = line.to_string();

        // Process terms longest-first to handle overlapping terms.
        for (term, _entry) in &terms {
            modified_line = upgrade_term_in_line(&modified_line, term);
            if modified_line.contains(&linked_form(term)) {
                linked_terms.insert(term.to_string());
            }
        }

        result_lines.push(modified_line);
    }

    // Build the final output.
    let mut output = result_lines.join("\n");

    // Collect ref defs that need to be at the bottom.
    let mut ref_defs: Vec<String> = Vec::new();

    for term in &linked_terms {
        let Some(entry) = registry.get(term) else {
            continue;
        };

        let canonical_target = &entry.target;

        // Check if there's already a ref def we should keep.
        if let Some((existing_target, original_line)) = ref_def_map.get(term)
            && (existing_target == canonical_target || existing_target.contains("::"))
        {
            // Keep existing ref def verbatim when the target matches canonical
            // OR is an intra-doc link. Preserves multi-line format from rustfmt.
            ref_defs.push(original_line.clone());
            continue;
        }

        // Add or replace with canonical target.
        ref_defs.push(ref_def_line(term, canonical_target));
    }

    // Also keep any existing ref defs for terms NOT in the registry
    // (e.g., intra-doc links to crate types that the user manually added).
    for line in &existing_refs {
        if let Some((term, _)) = parse_ref_def(line)
            && !linked_terms.contains(&term)
        {
            ref_defs.push(line.clone());
        }
    }

    // Sort ref defs alphabetically (case-insensitive).
    ref_defs.sort_by_key(|s| s.to_lowercase());
    ref_defs.dedup();

    // Append ref defs if any.
    if !ref_defs.is_empty() {
        // Ensure blank line before ref defs.
        if !output.ends_with("\n\n") && !output.ends_with('\n') {
            output.push_str("\n\n");
        } else if output.ends_with('\n') && !output.ends_with("\n\n") {
            output.push('\n');
        }

        output.push_str(&ref_defs.join("\n"));
    }

    output
}

/// Scans a line for an unclosed backtick opening sequence.
///
/// Matches backtick pairs as [`find_inline_code_spans`] does. If an opening sequence
/// has no matching close on the same line, returns `Some(tick_count)`.
fn find_unclosed_backtick_opening(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] != b'`' {
            i += 1;
            continue;
        }

        // Count opening backticks.
        let open_start = i;
        while i < len && bytes[i] == b'`' {
            i += 1;
        }
        let tick_count = i - open_start;

        // Search for matching closing backtick sequence (exactly tick_count).
        let mut found = false;
        let mut j = i;
        while j < len {
            if bytes[j] != b'`' {
                j += 1;
                continue;
            }

            let close_start = j;
            while j < len && bytes[j] == b'`' {
                j += 1;
            }

            if j - close_start == tick_count {
                // Matched - skip past and continue scanning.
                i = j;
                found = true;
                break;
            }
        }

        if !found {
            return Some(tick_count);
        }
    }

    None
}

/// Checks if a line contains a closing backtick sequence of exactly `tick_count`
/// backticks.
fn has_closing_backtick_sequence(line: &str, tick_count: usize) -> bool {
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] != b'`' {
            i += 1;
            continue;
        }

        let start = i;
        while i < len && bytes[i] == b'`' {
            i += 1;
        }

        if i - start == tick_count {
            return true;
        }
    }

    false
}

/// Finds all markdown link text byte ranges in a line.
///
/// Locates the text content inside `[text](url)` and `[text][ref]` patterns.
/// Returns ranges as `(start, end)` where `start` is the byte position of `[`
/// and `end` is one past `]` of the link text portion.
fn find_markdown_link_ranges(line: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] != b'[' {
            i += 1;
            continue;
        }

        // Skip image links (preceded by !).
        if i > 0 && bytes[i - 1] == b'!' {
            i += 1;
            continue;
        }

        let bracket_start = i;

        // Find matching closing bracket, accounting for nested brackets.
        let mut depth = 0;
        let mut close = None;
        let mut j = i;
        while j < len {
            match bytes[j] {
                b'[' => depth += 1,
                b']' => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(j);
                        break;
                    }
                }
                _ => {}
            }
            j += 1;
        }

        let Some(close_pos) = close else {
            i += 1;
            continue;
        };

        // Check what follows the closing bracket: (url) or [ref].
        let after = close_pos + 1;
        if after < len && (bytes[after] == b'(' || bytes[after] == b'[') {
            // This is a link - the text between bracket_start and close_pos+1
            // is the link text region.
            ranges.push((bracket_start, close_pos + 1));
        }

        i = close_pos + 1;
    }

    ranges
}

/// Finds all inline code span byte ranges in a line.
///
/// Follows the [`CommonMark`] spec: a code span opens with a sequence of N backticks and
/// closes at the next sequence of exactly N backticks. Returns ranges as `(start, end)`
/// where `start` is the position of the first opening backtick and `end` is one past the
/// last closing backtick.
///
/// [`CommonMark`]: https://spec.commonmark.org/0.30/#code-spans
fn find_inline_code_spans(line: &str) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] != b'`' {
            i += 1;
            continue;
        }

        // Count opening backticks.
        let open_start = i;
        while i < len && bytes[i] == b'`' {
            i += 1;
        }
        let tick_count = i - open_start;

        // Search for matching closing backtick sequence (exactly tick_count).
        let mut found = false;
        let mut j = i;
        while j < len {
            if bytes[j] != b'`' {
                j += 1;
                continue;
            }

            // Count this backtick sequence.
            let close_start = j;
            while j < len && bytes[j] == b'`' {
                j += 1;
            }

            if j - close_start == tick_count {
                spans.push((open_start, j));
                i = j;
                found = true;
                break;
            }
        }

        if !found {
            // No matching close. Not a code span; i is already past
            // the opening backticks.
        }
    }

    spans
}

/// Upgrades a single term occurrence in a line.
///
/// For **plain terms** (no backticks), handles three cases:
/// 1. Already linked: `` [`Term`] `` - leave as-is
/// 2. Backticked only: `` `Term` `` - upgrade to `` [`Term`] ``
/// 3. Plain text: `Term` - upgrade to `` [`Term`] ``
///
/// For **compound terms** (contain backticks, e.g., `` [`CSI` spec] ``):
/// 1. Already linked: `[`[`CSI`]` spec]` - leave as-is
/// 2. Not linked: `` [`CSI` spec] `` - upgrade to `[`[`CSI`]` spec]`
///
/// [`CSI` spec]: https://en.wikipedia.org/wiki/ANSI_escape_code#CSI
/// [`CSI`]: crate::CsiSequence
fn upgrade_term_in_line(line: &str, term: &str) -> String {
    // Don't process inside code fences (lines starting with ```).
    if line.trim_start().starts_with("```") {
        return line.to_string();
    }

    if term.contains('`') {
        return upgrade_compound_term_in_line(line, term);
    }

    let backtick_pattern = format!("`{term}`");
    let linked_pattern = format!("[`{term}`]");

    // Compute link text ranges for the original line to protect terms inside
    // existing markdown link text (e.g., [text with Term](url)).
    let link_ranges = find_markdown_link_ranges(line);

    // Step 1: Upgrade backticked-only `Term` -> [`Term`].
    // Find `Term` that is NOT preceded by [ (already linked) and NOT inside link text.
    let mut result = String::new();
    let mut search_start = 0;
    let bytes = line.as_bytes();
    while let Some(pos) = line[search_start..].find(&backtick_pattern) {
        let abs_pos = search_start + pos;
        let end_pos = abs_pos + backtick_pattern.len();

        let already_linked = abs_pos > 0 && bytes.get(abs_pos - 1) == Some(&b'[');

        // Check if inside a markdown link text range.
        let inside_link = link_ranges
            .iter()
            .any(|&(start, end)| abs_pos >= start && end_pos <= end);

        if already_linked || inside_link {
            result.push_str(&line[search_start..end_pos]);
        } else {
            result.push_str(&line[search_start..abs_pos]);
            result.push_str(&linked_pattern);
        }
        search_start = end_pos;
    }
    result.push_str(&line[search_start..]);

    // Step 2: Upgrade plain text Term -> [`Term`].
    // Find whole-word occurrences not inside inline code spans or link text.
    let code_spans = find_inline_code_spans(&result);
    let link_ranges_step2 = find_markdown_link_ranges(&result);
    let mut final_result = String::new();
    search_start = 0;
    let result_bytes = result.as_bytes();
    while let Some(pos) = result[search_start..].find(term) {
        let abs_pos = search_start + pos;
        let end_pos = abs_pos + term.len();

        // Whole-word check: not part of a longer identifier.
        let prev_is_word = abs_pos > 0
            && result_bytes
                .get(abs_pos - 1)
                .is_some_and(|&b| b.is_ascii_alphanumeric() || b == b'_');
        let next_is_word = result_bytes
            .get(end_pos)
            .is_some_and(|&b| b.is_ascii_alphanumeric() || b == b'_');

        // Code span check: term is inside an inline code span.
        let is_inside_code_span = code_spans
            .iter()
            .any(|&(span_start, span_end)| abs_pos > span_start && end_pos < span_end);

        // Link text check: term is inside markdown link text.
        let is_inside_link = link_ranges_step2
            .iter()
            .any(|&(start, end)| abs_pos >= start && end_pos <= end);

        let is_part_of_word = prev_is_word || next_is_word;

        if is_inside_code_span || is_part_of_word || is_inside_link {
            final_result.push_str(&result[search_start..end_pos]);
        } else {
            final_result.push_str(&result[search_start..abs_pos]);
            final_result.push_str(&linked_pattern);
        }
        search_start = end_pos;
    }
    final_result.push_str(&result[search_start..]);

    final_result
}

/// Upgrades a compound term (one that contains backticks) in a line.
///
/// For a term like `` [`CSI` spec] ``, finds that exact text and wraps it
/// with brackets: `[`[`CSI`]` spec]`. Skips occurrences already inside brackets.
///
/// [`CSI` spec]: https://en.wikipedia.org/wiki/ANSI_escape_code#CSI
/// [`CSI`]: crate::CsiSequence
fn upgrade_compound_term_in_line(line: &str, term: &str) -> String {
    let linked_pattern = format!("[{term}]");

    let mut result = String::new();
    let mut search_start = 0;
    let bytes = line.as_bytes();

    while let Some(pos) = line[search_start..].find(term) {
        let abs_pos = search_start + pos;

        // Check if already inside brackets (already linked).
        let already_linked = abs_pos > 0 && bytes.get(abs_pos - 1) == Some(&b'[');

        if already_linked {
            result.push_str(&line[search_start..abs_pos + term.len()]);
        } else {
            result.push_str(&line[search_start..abs_pos]);
            result.push_str(&linked_pattern);
        }
        search_start = abs_pos + term.len();
    }
    result.push_str(&line[search_start..]);

    result
}

/// Splits text into content and trailing reference definitions.
///
/// Reference definitions are lines matching `` [`Term`]: target `` at the end
/// of the text (after a blank line separator). Handles both single-line and
/// multi-line ref defs (where rustfmt split a long line across two lines):
///
/// ```text
/// [`MioPollWorker::block_until_ready_then_dispatch_impl()`]:
///     crate::terminal_lib_backends::MioPollWorker::block_until_ready_then_dispatch_impl
/// ```
fn split_off_ref_defs(text: &str) -> (String, Vec<String>) {
    let lines: Vec<&str> = text.lines().collect();
    let mut ref_defs = Vec::new();
    let mut content_end = lines.len();

    // Walk backwards from the end to find ref def block.
    let mut i = lines.len();
    while i > 0 {
        i -= 1;
        let trimmed = lines[i].trim();
        if trimmed.is_empty() {
            // Blank line - could be separator before ref defs.
            if !ref_defs.is_empty() {
                content_end = i;
                break;
            }
            continue;
        }
        if is_ref_def(trimmed) {
            ref_defs.push(trimmed.to_string());
            content_end = i;
        } else if is_incomplete_ref_def(trimmed) {
            // Incomplete ref def header with no target (e.g., `[`Term`]:`)
            // The target should have been on a continuation line below, but
            // we may have already consumed it. Preserve it as-is.
            ref_defs.push(trimmed.to_string());
            content_end = i;
        } else if i > 0 && is_incomplete_ref_def(lines[i - 1].trim()) {
            // This line is a continuation of a multi-line ref def.
            // Combine the header line (i-1) with this continuation line (i).
            let combined = format!("{}\n{}", lines[i - 1].trim(), lines[i]);
            ref_defs.push(combined);
            content_end = i - 1;
            i -= 1; // Skip the header line too
        } else {
            // Non-ref-def, non-blank line - stop.
            break;
        }
    }

    ref_defs.reverse();

    let content = lines[..content_end].join("\n");
    (content, ref_defs)
}

/// Checks if a line is an incomplete ref def header (no target after `]:`).
///
/// Matches lines like `` [`Term`]: `` or `` [Term]: `` where the target URL
/// is expected on the next (continuation) line due to rustfmt line-length limits.
fn is_incomplete_ref_def(line: &str) -> bool {
    if !line.starts_with('[') {
        return false;
    }
    if let Some(pos) = line.find("]:") {
        let after = &line[pos + 2..];
        after.trim().is_empty()
    } else {
        false
    }
}

/// Checks if a line is a reference-style link definition.
///
/// Matches patterns like:
/// - `` [`CSI`]: https://example.com ``
/// - `[CSI]: https://example.com`
///
/// [`CSI`]: crate::CsiSequence
fn is_ref_def(line: &str) -> bool { line.starts_with('[') && line.contains("]: ") }

/// Parses a reference definition line into (term, target).
fn parse_ref_def(line: &str) -> Option<(String, String)> {
    let line = line.trim();

    // Try backticked form: [`Term`]: target
    if line.starts_with("[`") {
        if let Some(end) = line.find("`]: ") {
            let term = line[2..end].to_string();
            let target = line[end + 4..].trim().to_string();
            return Some((term, target));
        }
        // Multi-line ref def: [`Term`]:\n    target
        if let Some(end) = line.find("`]:") {
            let term = line[2..end].to_string();
            let target = line[end + 3..].trim().to_string();
            if target.is_empty() {
                return Some((term, String::new()));
            }
            return Some((term, target));
        }
    }

    // Try plain form: [Term]: target
    if line.starts_with('[')
        && let Some(end) = line.find("]: ")
    {
        let term = line[1..end].to_string();
        let target = line[end + 3..].trim().to_string();
        return Some((term, target));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_registry() -> TechnicalTermDictionary {
        TechnicalTermDictionary::from_seed(None).unwrap()
    }

    #[test]
    fn test_plain_text_upgraded_to_linked() {
        let registry = test_registry();
        let input = "Parses CSI sequences from the input.";
        let result = link_known_terms(input, &registry);
        assert!(result.contains("[`CSI`]"), "Expected [`CSI`] in: {result}");
        assert!(
            result.contains("[`CSI`]: crate::CsiSequence"),
            "Expected ref def in: {result}"
        );
    }

    #[test]
    fn test_backticked_upgraded_to_linked() {
        let registry = test_registry();
        let input = "Parses `CSI` sequences from the input.";
        let result = link_known_terms(input, &registry);
        assert!(result.contains("[`CSI`]"), "Expected [`CSI`] in: {result}");
        assert!(
            !result.contains("``CSI``"),
            "Should not double-backtick: {result}"
        );
    }

    #[test]
    fn test_already_linked_preserved() {
        let registry = test_registry();
        let input = "Parses [`CSI`] sequences.\n\n[`CSI`]: crate::CsiSequence";
        let result = link_known_terms(input, &registry);
        // Should have exactly one [`CSI`] in content and one ref def.
        let content_links =
            result.matches("[`CSI`]").count() - result.matches("[`CSI`]: ").count();
        assert_eq!(content_links, 1, "Expected 1 content link in: {result}");
    }

    #[test]
    fn test_wrong_target_corrected() {
        let registry = test_registry();
        let input = "Parses [`CSI`] sequences.\n\n[`CSI`]: https://wrong-url.com";
        let result = link_known_terms(input, &registry);
        assert!(
            result.contains("[`CSI`]: crate::CsiSequence"),
            "Expected corrected target in: {result}"
        );
        assert!(
            !result.contains("wrong-url"),
            "Should not contain wrong URL: {result}"
        );
    }

    #[test]
    fn test_multiple_occurrences_all_linked() {
        let registry = test_registry();
        let input = "The CSI sequence and another CSI usage.";
        let result = link_known_terms(input, &registry);
        let content_links =
            result.matches("[`CSI`]").count() - result.matches("[`CSI`]: ").count();
        assert_eq!(content_links, 2, "Expected 2 content links in: {result}");
    }

    #[test]
    fn test_heading_also_linked() {
        let registry = test_registry();
        let input = "# CSI Sequences\n\nParsing CSI here.";
        let result = link_known_terms(input, &registry);
        // Headings should also be linked.
        assert!(
            result.starts_with("# [`CSI`]"),
            "Heading should be linked: {result}"
        );
        assert!(
            result.contains("Parsing [`CSI`]"),
            "Body should be linked: {result}"
        );
    }

    #[test]
    fn test_tier2_external_link() {
        let registry = test_registry();
        let input = "Uses ANSI escape codes.";
        let result = link_known_terms(input, &registry);
        assert!(
            result.contains("[`ANSI`]"),
            "Expected [`ANSI`] in: {result}"
        );
        assert!(
            result.contains("[`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code"),
            "Expected external URL ref def in: {result}"
        );
    }

    #[test]
    fn test_overlapping_terms_longest_first() {
        let registry = test_registry();
        let input = "See the `VT-100` spec for details.";
        let result = link_known_terms(input, &registry);
        assert!(
            result.contains("[`VT-100` spec]"),
            "Expected longest match in: {result}"
        );
    }

    #[test]
    fn test_no_known_terms_unchanged() {
        let registry = test_registry();
        let input = "This has no known terms at all.";
        let result = link_known_terms(input, &registry);
        assert_eq!(result, input);
    }

    #[test]
    fn test_empty_input() {
        let registry = test_registry();
        let result = link_known_terms("", &registry);
        assert_eq!(result, "");
    }

    #[test]
    fn test_code_fence_not_modified() {
        let registry = test_registry();
        let input = "```\nCSI sequence inside code\n```";
        let result = link_known_terms(input, &registry);
        // The code fence lines themselves should not be modified.
        assert!(
            result.contains("```\nCSI sequence inside code\n```"),
            "Code fence should be preserved: {result}"
        );
    }

    #[test]
    fn test_inline_code_span_not_modified() {
        let registry = test_registry();
        let input = "The `CSI [ 38 ; 5 ; n m` sequence sets color.";
        let result = link_known_terms(input, &registry);
        // The CSI inside the longer inline code span should NOT be linked.
        assert!(
            result.contains("`CSI [ 38 ; 5 ; n m`"),
            "Inline code span should be preserved: {result}"
        );
    }

    #[test]
    fn test_ref_defs_sorted_alphabetically() {
        let registry = test_registry();
        let input = "Uses SGR and CSI and ANSI.";
        let result = link_known_terms(input, &registry);
        let ref_def_section: String = result
            .lines()
            .skip_while(|l| !l.starts_with('['))
            .collect::<Vec<_>>()
            .join("\n");
        let lines: Vec<&str> = ref_def_section.lines().collect();
        // Should be sorted: ANSI, CSI, SGR.
        assert!(
            lines.len() >= 3,
            "Expected at least 3 ref defs: {ref_def_section}"
        );
        assert!(
            lines[0].contains("ANSI"),
            "First should be ANSI: {ref_def_section}"
        );
        assert!(
            lines[1].contains("CSI"),
            "Second should be CSI: {ref_def_section}"
        );
        assert!(
            lines[2].contains("SGR"),
            "Third should be SGR: {ref_def_section}"
        );
    }

    #[test]
    fn test_split_off_ref_defs() {
        let text = "Some content.\n\n[`CSI`]: https://example.com\n[`SGR`]: https://example2.com";
        let (content, refs) = split_off_ref_defs(text);
        assert_eq!(content, "Some content.");
        assert_eq!(refs.len(), 2);
        assert!(refs[0].contains("CSI"));
        assert!(refs[1].contains("SGR"));
    }

    #[test]
    fn test_parse_ref_def_backticked() {
        let (term, target) = parse_ref_def("[`CSI`]: https://example.com").unwrap();
        assert_eq!(term, "CSI");
        assert_eq!(target, "https://example.com");
    }

    #[test]
    fn test_parse_ref_def_plain() {
        let (term, target) = parse_ref_def("[CSI]: https://example.com").unwrap();
        assert_eq!(term, "CSI");
        assert_eq!(target, "https://example.com");
    }

    #[test]
    fn test_find_inline_code_spans_single() {
        let spans = find_inline_code_spans("The `foo bar` end");
        assert_eq!(spans, vec![(4, 13)]);
    }

    #[test]
    fn test_find_inline_code_spans_multiple() {
        let spans = find_inline_code_spans("`a` and `b`");
        assert_eq!(spans, vec![(0, 3), (8, 11)]);
    }

    #[test]
    fn test_find_inline_code_spans_double_backtick() {
        let spans = find_inline_code_spans("The ``foo `bar` baz`` end");
        assert_eq!(spans, vec![(4, 21)]);
    }

    #[test]
    fn test_find_inline_code_spans_none() {
        let spans = find_inline_code_spans("plain text with no backticks");
        assert!(spans.is_empty());
    }

    #[test]
    fn test_term_deep_inside_code_span_not_modified() {
        let registry = test_registry();
        let input = "Run with: `cargo test -p CSI --lib foo -- --nocapture`";
        let result = link_known_terms(input, &registry);
        assert!(
            result.contains("`cargo test -p CSI --lib foo -- --nocapture`"),
            "Term deep inside code span should be preserved: {result}"
        );
        assert!(
            !result.contains("[`CSI`]"),
            "Should not linkify term inside code span: {result}"
        );
    }

    #[test]
    fn test_term_inside_markdown_link_text_not_modified() {
        let registry = test_registry();
        let input = "See [Linux TTY and CSI codes](https://example.com) for details.";
        let result = link_known_terms(input, &registry);
        assert!(
            result.contains("[Linux TTY and CSI codes](https://example.com)"),
            "Term inside link text should not be modified: {result}"
        );
    }

    #[test]
    fn test_term_inside_ref_link_text_not_modified() {
        let registry = test_registry();
        let input = "See [CSI overview][ref] for details.";
        let result = link_known_terms(input, &registry);
        assert!(
            result.contains("[CSI overview][ref]"),
            "Term inside reference link text should not be modified: {result}"
        );
    }

    #[test]
    fn test_toc_block_not_modified() {
        let registry = test_registry();
        let input = concat!(
            "<!-- TOC -->\n",
            "\n",
            "- [CSI Sequences](#csi)\n",
            "- [SGR Codes](#sgr)\n",
            "\n",
            "<!-- /TOC -->\n",
            "\n",
            "Uses CSI sequences.",
        );
        let result = link_known_terms(input, &registry);

        // Lines inside the TOC block should not be modified.
        assert!(
            result.contains("- [CSI Sequences](#csi)"),
            "TOC content should not be modified: {result}"
        );
        assert!(
            result.contains("- [SGR Codes](#sgr)"),
            "TOC content should not be modified: {result}"
        );

        // Content outside the TOC should be linked.
        assert!(
            result.contains("Uses [`CSI`]"),
            "Content outside TOC should be linked: {result}"
        );
    }

    #[test]
    fn test_find_markdown_link_ranges() {
        let ranges = find_markdown_link_ranges("See [CSI info](url) here.");
        assert_eq!(ranges, vec![(4, 14)]);
    }

    #[test]
    fn test_find_markdown_link_ranges_ref_style() {
        let ranges = find_markdown_link_ranges("See [CSI info][ref] here.");
        assert_eq!(ranges, vec![(4, 14)]);
    }

    #[test]
    fn test_find_markdown_link_ranges_none() {
        let ranges = find_markdown_link_ranges("Plain text without links.");
        assert!(ranges.is_empty());
    }

    #[test]
    fn test_existing_intradoc_link_preserved() {
        let registry = test_registry();
        // PTY is in the registry with an external URL, but the file has its own
        // intra-doc link target. The file-local target should be preserved.
        let input = "Spawns a [`PTY`] process.\n\n[`PTY`]: crate::core::pty";
        let result = link_known_terms(input, &registry);
        assert!(
            result.contains("[`PTY`]: crate::core::pty"),
            "Intra-doc link target should be preserved: {result}"
        );
        assert!(
            !result.contains("wikipedia"),
            "Should not replace with external URL: {result}"
        );
    }

    #[test]
    fn test_existing_external_link_replaced_by_canonical() {
        let registry = test_registry();
        // CSI has a wrong external URL that should be replaced by the canonical target.
        let input = "Parses [`CSI`] sequences.\n\n[`CSI`]: https://wrong-url.com";
        let result = link_known_terms(input, &registry);
        assert!(
            result.contains("[`CSI`]: crate::CsiSequence"),
            "Wrong external URL should be replaced by canonical: {result}"
        );
        assert!(
            !result.contains("wrong-url"),
            "Should not contain wrong URL: {result}"
        );
    }

    #[test]
    fn test_multiline_code_span_not_modified() {
        let registry = test_registry();
        // A backtick span that opens on one line and closes on the next.
        let input = "Run with: `cargo test -p r3bl_tui --lib CSI --\n--nocapture`";
        let result = link_known_terms(input, &registry);
        assert!(
            !result.contains("[`CSI`]"),
            "Term inside multi-line code span should not be linked: {result}"
        );
        assert_eq!(result, input, "Multi-line code span should be unchanged");
    }

    #[test]
    fn test_multiline_code_span_three_lines() {
        let registry = test_registry();
        // A backtick span that opens on line 1 and closes on line 3.
        let input = "Run: `cargo test\n-p CSI\n--nocapture`";
        let result = link_known_terms(input, &registry);
        assert!(
            !result.contains("[`CSI`]"),
            "Term on middle line of multi-line code span should not be linked: {result}"
        );
        assert_eq!(result, input, "Multi-line code span should be unchanged");
    }

    #[test]
    fn test_multiline_code_span_subsequent_linking_works() {
        let registry = test_registry();
        // After the multi-line span closes, terms on subsequent lines should be linked.
        let input = "Run: `cargo test\n--nocapture`\nUses CSI sequences.";
        let result = link_known_terms(input, &registry);
        assert!(
            result.contains("Uses [`CSI`]"),
            "Term after multi-line code span should be linked: {result}"
        );
    }

    #[test]
    fn test_multiline_code_span_double_backtick() {
        let registry = test_registry();
        // A double-backtick span crossing lines.
        let input = "Run: ``cargo test CSI\n--nocapture``";
        let result = link_known_terms(input, &registry);
        assert!(
            !result.contains("[`CSI`]"),
            "Term inside double-backtick multi-line span should not be linked: {result}"
        );
        assert_eq!(
            result, input,
            "Double-backtick multi-line span should be unchanged"
        );
    }

    #[test]
    fn test_split_off_ref_defs_multiline() {
        let text = "Some content.\n\n[`Short`]: target\n[`LongName`]:\n    crate::long::path::Target";
        let (content, refs) = split_off_ref_defs(text);
        assert_eq!(content, "Some content.");
        assert_eq!(refs.len(), 2, "Expected 2 ref defs: {refs:?}");
        assert!(refs[0].contains("Short"), "First ref def: {}", refs[0]);
        assert!(
            refs[1].contains("LongName") && refs[1].contains("crate::long::path::Target"),
            "Multi-line ref def should be combined: {}",
            refs[1]
        );
    }

    #[test]
    fn test_split_off_ref_defs_mixed_single_and_multiline() {
        // Simulates rrt.rs pattern: single-line and multi-line ref defs interleaved.
        let text = concat!(
            "Content here.\n\n",
            "[`RAII`]: https://en.wikipedia.org/wiki/RAII\n",
            "[`RRTWaker::wake()`]:\n",
            "    super::RRTWaker::wake\n",
            "[`RRTWorker`]: super::RRTWorker\n",
            "[`block_until_ready()`]:\n",
            "    super::RRTWorker::block_until_ready\n",
            "[`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html",
        );
        let (content, refs) = split_off_ref_defs(text);
        assert_eq!(content.trim(), "Content here.");
        assert_eq!(refs.len(), 5, "Expected 5 ref defs: {refs:?}");
        // Verify multi-line ref defs are correctly combined.
        assert!(
            refs[1].contains("RRTWaker::wake()")
                && refs[1].contains("super::RRTWaker::wake")
        );
        assert!(
            refs[3].contains("block_until_ready()")
                && refs[3].contains("super::RRTWorker::block_until_ready")
        );
    }

    #[test]
    fn test_is_incomplete_ref_def() {
        assert!(is_incomplete_ref_def("[`Term`]:"));
        assert!(is_incomplete_ref_def("[`Long::Name()`]:"));
        assert!(is_incomplete_ref_def("[Term]:"));
        assert!(!is_incomplete_ref_def("[`Term`]: target"));
        assert!(!is_incomplete_ref_def("[`Term`]: https://example.com"));
        assert!(!is_incomplete_ref_def("not a ref"));
        assert!(!is_incomplete_ref_def("plain text"));
    }

    #[test]
    fn test_multiline_ref_defs_preserved_on_second_run() {
        let registry = test_registry();
        // Input with a multi-line ref def (as rustfmt would format it).
        let input =
            concat!("Uses [`CSI`] sequences.\n\n", "[`CSI`]: crate::CsiSequence",);
        let first_run = link_known_terms(input, &registry);
        let second_run = link_known_terms(&first_run, &registry);
        assert_eq!(
            first_run, second_run,
            "Term linker should be idempotent:\nFirst:  {first_run}\nSecond: {second_run}"
        );
    }

    #[test]
    fn test_multiline_ref_def_not_linkified_inside_url() {
        let registry = test_registry();
        // The term "epoll" appears inside a URL target. It should NOT be
        // linkified because ref defs are split off before term processing.
        let input = "Uses [`epoll`] for I/O.\n\n[`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html";
        let result = link_known_terms(input, &registry);
        assert!(
            result.contains("epoll.7.html"),
            "URL should not be corrupted: {result}"
        );
        assert!(
            !result.contains("[`epoll`].7.html"),
            "Term inside URL should not be linkified: {result}"
        );
    }
}
