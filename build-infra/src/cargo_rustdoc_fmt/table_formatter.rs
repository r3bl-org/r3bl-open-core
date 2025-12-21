// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Format markdown tables in rustdoc comments.

use regex::Regex;
use unicode_width::UnicodeWidthStr;

/// Format all markdown tables in the given text.
///
/// Aligns columns and normalizes table formatting while preserving content.
/// Code fence blocks (` ``` `) are preserved and not processed.
/// Content indentation (leading whitespace before `|`) is preserved.
///
/// # Panics
///
/// Panics if the internal regex pattern is invalid (should never happen with hardcoded
/// pattern).
#[must_use]
pub fn format_tables(text: &str) -> String {
    // Match table lines with optional leading whitespace.
    let table_regex = Regex::new(r"(?m)^[ \t]*\|.+\|[ \t]*$").unwrap();

    if !table_regex.is_match(text) {
        return text.to_string();
    }

    let lines: Vec<&str> = text.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;
    let mut in_code_fence = false;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        // Check for code fence boundaries (``` with optional language tag)
        if trimmed.starts_with("```") {
            in_code_fence = !in_code_fence;
            result.push(line.to_string());
            i += 1;
            continue;
        }

        // Skip table processing inside code fences
        if in_code_fence {
            result.push(line.to_string());
            i += 1;
            continue;
        }

        if let Some((table, content_indent)) = extract_table(&lines, i) {
            let formatted = format_single_table(&table, &content_indent);
            result.extend(formatted);
            i += table.len();
        } else {
            result.push(line.to_string());
            i += 1;
        }
    }

    result.join("\n")
}

/// Extract a table starting at the given index.
///
/// Returns the table lines (without content indentation) and the content indentation
/// string that should be re-applied after formatting.
fn extract_table(lines: &[&str], start: usize) -> Option<(Vec<String>, String)> {
    if start >= lines.len() {
        return None;
    }

    let first_line = lines[start];
    let trimmed = first_line.trim();
    if !trimmed.starts_with('|') || !trimmed.ends_with('|') {
        return None;
    }

    // Capture content indentation from first line (whitespace before the `|`).
    let content_indent = &first_line[..first_line.len() - first_line.trim_start().len()];

    let mut table_lines = Vec::new();
    let mut i = start;

    while i < lines.len() {
        let line = lines[i].trim();
        if line.starts_with('|') && line.ends_with('|') {
            table_lines.push(line.to_string());
            i += 1;
        } else {
            break;
        }
    }

    // Must have at least header and separator.
    if table_lines.len() >= 2 {
        Some((table_lines, content_indent.to_string()))
    } else {
        None
    }
}

/// Column alignment extracted from separator row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColumnAlignment {
    /// No alignment specified (default left in most renderers).
    None,
    /// Left alignment (`:---`).
    Left,
    /// Center alignment (`:---:`).
    Center,
    /// Right alignment (`---:`).
    Right,
}

/// Format a single table with proper column alignment.
///
/// The `content_indent` is prepended to each formatted line to preserve the original
/// indentation (e.g., for tables nested under numbered lists).
fn format_single_table(table: &[String], content_indent: &str) -> Vec<String> {
    if table.len() < 2 {
        // Return original lines with content indentation preserved.
        return table
            .iter()
            .map(|line| format!("{content_indent}{line}"))
            .collect();
    }

    // Parse table into cells.
    let rows: Vec<Vec<String>> = table.iter().map(|line| parse_table_row(line)).collect();

    if rows.is_empty() {
        return table
            .iter()
            .map(|line| format!("{content_indent}{line}"))
            .collect();
    }

    // Extract alignments from separator row (index 1).
    let alignments: Vec<ColumnAlignment> = if rows.len() >= 2 {
        rows[1].iter().map(|cell| parse_alignment(cell)).collect()
    } else {
        vec![]
    };

    // Calculate column widths.
    let col_count = rows[0].len();
    let mut col_widths = vec![0; col_count];

    for row in &rows {
        for (col_idx, cell) in row.iter().enumerate() {
            if col_idx < col_widths.len() {
                col_widths[col_idx] = col_widths[col_idx].max(cell.trim().width());
            }
        }
    }

    // Format each row, prepending content indentation.
    let mut formatted = Vec::new();
    for (row_idx, row) in rows.iter().enumerate() {
        let formatted_row = format_table_row(row, &col_widths, row_idx == 1, &alignments);
        formatted.push(format!("{content_indent}{formatted_row}"));
    }

    formatted
}

/// Parse alignment from a separator cell.
///
/// - `:---` or `:--` → Left
/// - `:---:` or `:--:` → Center
/// - `---:` or `--:` → Right
/// - `---` or `--` → None
fn parse_alignment(cell: &str) -> ColumnAlignment {
    let trimmed = cell.trim();
    let starts_colon = trimmed.starts_with(':');
    let ends_colon = trimmed.ends_with(':');

    match (starts_colon, ends_colon) {
        (true, true) => ColumnAlignment::Center,
        (true, false) => ColumnAlignment::Left,
        (false, true) => ColumnAlignment::Right,
        (false, false) => ColumnAlignment::None,
    }
}

/// Parse a table row into cells.
fn parse_table_row(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    let without_pipes = &trimmed[1..trimmed.len() - 1]; // Remove leading and trailing |

    without_pipes
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect()
}

/// Format a table row with proper padding.
fn format_table_row(
    cells: &[String],
    col_widths: &[usize],
    is_separator: bool,
    alignments: &[ColumnAlignment],
) -> String {
    let formatted_cells: Vec<String> = cells
        .iter()
        .enumerate()
        .map(|(idx, cell)| {
            if idx < col_widths.len() {
                if is_separator {
                    // Separator row: generate dashes with alignment markers preserved.
                    let alignment = alignments
                        .get(idx)
                        .copied()
                        .unwrap_or(ColumnAlignment::None);
                    format_separator_cell(col_widths[idx], alignment)
                } else {
                    // Data row: pad with spaces (character-aware)
                    pad_string_by_chars(cell.trim(), col_widths[idx])
                }
            } else {
                cell.clone()
            }
        })
        .collect();

    format!("| {} |", formatted_cells.join(" | "))
}

/// Format a separator cell with alignment markers.
///
/// Generates dashes with the appropriate leading/trailing colons based on alignment.
fn format_separator_cell(width: usize, alignment: ColumnAlignment) -> String {
    match alignment {
        ColumnAlignment::None => "-".repeat(width),
        ColumnAlignment::Left => {
            // `:---` format - colon at start, dashes fill the rest.
            if width <= 1 {
                ":".to_string()
            } else {
                format!(":{}", "-".repeat(width - 1))
            }
        }
        ColumnAlignment::Right => {
            // `---:` format - dashes fill most, colon at end.
            if width <= 1 {
                ":".to_string()
            } else {
                format!("{}:", "-".repeat(width - 1))
            }
        }
        ColumnAlignment::Center => {
            // `:---:` format - colons at both ends.
            if width <= 2 {
                "::".to_string()
            } else {
                format!(":{}:", "-".repeat(width - 2))
            }
        }
    }
}

/// Pad a string to the specified display width (accounts for emoji and unicode).
fn pad_string_by_chars(s: &str, width: usize) -> String {
    let display_width = s.width();
    if display_width >= width {
        s.to_string()
    } else {
        let padding = " ".repeat(width - display_width);
        format!("{s}{padding}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_table_formatting() {
        let input = "| A | B |\n|---|---|\n| 1 | 2 |";
        let output = format_tables(input);
        // Check that table is formatted with proper alignment
        assert!(output.contains("| A"));
        assert!(output.contains("| B"));
        assert!(output.contains("| 1"));
        assert!(output.contains("| 2"));
    }

    #[test]
    fn test_unaligned_table() {
        let input = "| Short | Very Long Text |\n|---|---|\n| A | B |";
        let output = format_tables(input);
        // Check that columns are properly aligned
        assert!(output.contains("Very Long Text"));
        assert!(output.contains("Short"));
        // The "Short" column should be padded to match "Very Long Text" length
        let lines: Vec<&str> = output.lines().collect();
        assert!(lines.len() >= 3);
    }

    #[test]
    fn test_empty_text() {
        let output = format_tables("");
        assert_eq!(output, "");
    }

    #[test]
    fn test_non_table_text() {
        let input = "This is not a table\nJust regular text";
        let output = format_tables(input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_table_with_surrounding_text() {
        let input = "Some text\n| A | B |\n|---|---|\n| 1 | 2 |\nMore text";
        let output = format_tables(input);
        assert!(output.contains("Some text"));
        assert!(output.contains("More text"));
        // Check table content is preserved
        assert!(output.contains("| A"));
        assert!(output.contains("| B"));
    }

    #[test]
    fn test_code_fence_preserves_pipe_content() {
        // ASCII art with pipes inside code fence should NOT be formatted as a table
        let input = r"Some text
```text
+---------------------+
|         ↑           |
|      within vp      |
|         ↓           |
+---------------------+
```
More text";
        let output = format_tables(input);
        // Content inside code fence should be unchanged
        assert!(output.contains("|         ↑           |"));
        assert!(output.contains("|      within vp      |"));
        assert!(output.contains("|         ↓           |"));
    }

    #[test]
    fn test_code_fence_with_language_tag() {
        let input = r"```rust
| not | a | table |
```";
        let output = format_tables(input);
        // Should preserve exactly as-is (not format as table)
        assert_eq!(output, input);
    }

    #[test]
    fn test_table_outside_code_fence_still_formatted() {
        let input = r"```text
| preserved | content |
```
| A | B |
|---|---|
| 1 | 2 |";
        let output = format_tables(input);
        // Code fence content preserved exactly
        assert!(output.contains("| preserved | content |"));
        // Table outside is formatted (columns aligned with padding)
        assert!(output.contains("| A"));
        assert!(output.contains("| B"));
        assert!(output.contains("| 1"));
        assert!(output.contains("| 2"));
    }

    #[test]
    fn test_multiple_code_fences() {
        let input = r"```
| fence1 |
```
| A | B |
|---|---|
| 1 | 2 |
```
| fence2 |
```";
        let output = format_tables(input);
        // Both fence contents preserved
        assert!(output.contains("| fence1 |"));
        assert!(output.contains("| fence2 |"));
    }

    #[test]
    fn test_left_alignment_preserved() {
        let input = "| Column | Description |\n|:---|:---|\n| A | B |";
        let output = format_tables(input);
        let lines: Vec<&str> = output.lines().collect();
        // Separator row should preserve left alignment markers
        assert!(
            lines[1].contains(":---"),
            "Left alignment marker should be preserved: {}",
            lines[1]
        );
    }

    #[test]
    fn test_right_alignment_preserved() {
        let input = "| Column | Description |\n|---:|---:|\n| A | B |";
        let output = format_tables(input);
        let lines: Vec<&str> = output.lines().collect();
        // Separator row should preserve right alignment markers
        assert!(
            lines[1].contains("---:"),
            "Right alignment marker should be preserved: {}",
            lines[1]
        );
    }

    #[test]
    fn test_center_alignment_preserved() {
        let input = "| Column | Description |\n|:---:|:---:|\n| A | B |";
        let output = format_tables(input);
        let lines: Vec<&str> = output.lines().collect();
        // Separator row should have colons at both ends for each cell
        assert!(
            lines[1].contains(':') && lines[1].matches(':').count() >= 4,
            "Center alignment markers should be preserved: {}",
            lines[1]
        );
    }

    #[test]
    fn test_mixed_alignment_preserved() {
        let input =
            "| Left | Center | Right | None |\n|:---|:---:|---:|---|\n| A | B | C | D |";
        let output = format_tables(input);
        let lines: Vec<&str> = output.lines().collect();
        let separator = lines[1];
        // Check each column's alignment is preserved.
        let cells: Vec<&str> = separator
            .trim()
            .trim_start_matches('|')
            .trim_end_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        assert_eq!(cells.len(), 4);
        // Left: starts with :, no trailing :
        assert!(cells[0].starts_with(':') && !cells[0].ends_with(':'));
        // Center: starts and ends with :
        assert!(cells[1].starts_with(':') && cells[1].ends_with(':'));
        // Right: ends with :, no leading :
        assert!(!cells[2].starts_with(':') && cells[2].ends_with(':'));
        // None: no colons
        assert!(!cells[3].contains(':'));
    }

    // ==================== Content Indentation Tests ====================
    // Tables nested under numbered lists need their indentation preserved.

    #[test]
    fn test_indented_table_preserves_spaces() {
        // Table with 4-space indentation (common for nested under numbered lists).
        let input = "    | A | B |\n    |---|---|\n    | 1 | 2 |";
        let output = format_tables(input);
        let lines: Vec<&str> = output.lines().collect();
        // All lines should start with 4 spaces.
        for line in &lines {
            assert!(
                line.starts_with("    "),
                "Line should preserve 4-space indent: '{}'",
                line
            );
        }
    }

    #[test]
    fn test_indented_table_formats_correctly() {
        // Table with indentation and uneven columns.
        let input = "    | Short | Very Long Text |\n    |---|---|\n    | A | B |";
        let output = format_tables(input);
        let lines: Vec<&str> = output.lines().collect();
        // Check indentation preserved.
        assert!(lines[0].starts_with("    "));
        assert!(lines[1].starts_with("    "));
        assert!(lines[2].starts_with("    "));
        // Check content is still formatted (columns aligned).
        assert!(output.contains("Very Long Text"));
    }

    #[test]
    fn test_mixed_indentation_levels() {
        // Two tables with different indentation levels.
        let input = "| A | B |\n|---|---|\n| 1 | 2 |\n\n    | C | D |\n    |---|---|\n    | 3 | 4 |";
        let output = format_tables(input);
        let lines: Vec<&str> = output.lines().collect();
        // First table has no indentation.
        assert!(lines[0].starts_with("| A"));
        // Second table (after blank line) has 4-space indentation.
        assert!(
            lines[4].starts_with("    "),
            "Second table should have 4-space indent: '{}'",
            lines[4]
        );
    }

    #[test]
    fn test_tab_indentation_preserved() {
        // Table with tab indentation.
        let input = "\t| A | B |\n\t|---|---|\n\t| 1 | 2 |";
        let output = format_tables(input);
        let lines: Vec<&str> = output.lines().collect();
        // All lines should start with tab.
        for line in &lines {
            assert!(
                line.starts_with('\t'),
                "Line should preserve tab indent: '{}'",
                line
            );
        }
    }

    #[test]
    fn test_no_indentation_still_works() {
        // Table with no indentation (regression test).
        let input = "| A | B |\n|---|---|\n| 1 | 2 |";
        let output = format_tables(input);
        let lines: Vec<&str> = output.lines().collect();
        // Lines should start with pipe (no indentation).
        assert!(lines[0].starts_with('|'));
        assert!(lines[1].starts_with('|'));
        assert!(lines[2].starts_with('|'));
    }

    #[test]
    fn test_indented_table_with_surrounding_text() {
        // Table indented within rustdoc content.
        let input = "Some text before\n    | A | B |\n    |---|---|\n    | 1 | 2 |\nSome text after";
        let output = format_tables(input);
        // Surrounding text preserved.
        assert!(output.contains("Some text before"));
        assert!(output.contains("Some text after"));
        // Table lines have indentation.
        let lines: Vec<&str> = output.lines().collect();
        assert!(lines[1].starts_with("    "));
        assert!(lines[2].starts_with("    "));
        assert!(lines[3].starts_with("    "));
    }
}
