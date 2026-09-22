// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// rustdoc-fmt: skip

//! Format Markdown tables in rustdoc comments.

use crate::cargo_rustdoc_fmt::{DocNode, FormatStatus,
                               constants::{CONTENT_SPACE_CHAR, INNER_DOC_MARKER,
                                           OUTER_DOC_MARKER},
                               format_table_data};

/// Formats a [`DocNode::Table`] using [`format_table_data`].
///
/// Sets `status = FormatStatus::Modified` on the node if any formatted row differs from
/// original raw lines. Returns [`FormatStatus::Modified`] if the node was modified, or
/// [`FormatStatus::Unchanged`] otherwise.
pub fn format_table_node(node: &mut DocNode) -> FormatStatus {
    if let DocNode::Table {
        table,
        content_indent,
        raw_lines,
        status,
        ..
    } = node
    {
        let formatted_rows = format_table_data(table);
        if formatted_rows.is_empty() {
            return FormatStatus::Unchanged;
        }

        let is_different = formatted_rows.len() != raw_lines.len()
            || formatted_rows
                .iter()
                .zip(raw_lines.iter())
                .any(|(fmt_row, raw_line)| {
                    let trimmed_raw = raw_line.trim_start();
                    let content_after_marker = if let Some(after) =
                        trimmed_raw.strip_prefix(OUTER_DOC_MARKER)
                    {
                        after
                    } else if let Some(after) = trimmed_raw.strip_prefix(INNER_DOC_MARKER)
                    {
                        after
                    } else {
                        trimmed_raw
                    };
                    let expected_content =
                        format!("{CONTENT_SPACE_CHAR}{content_indent}{fmt_row}");
                    content_after_marker != expected_content
                });

        if is_different {
            *status = FormatStatus::Modified;
            return FormatStatus::Modified;
        }
    }
    FormatStatus::Unchanged
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cargo_rustdoc_fmt::{FileChunk, SourceFileCst, constants::LF_LINE_ENDING,
                                   extract_comment_content};

    /// Test helper: wraps raw Markdown lines in `/// ` doc comments, runs
    /// [`format_table_node`] on every node, and strips the `/// ` prefix from the
    /// reconstructed output.
    fn format_tables(text: &str) -> String {
        let raw_doc: String = text
            .lines()
            .map(|line| {
                if line.is_empty() {
                    OUTER_DOC_MARKER.to_string()
                } else {
                    format!("{OUTER_DOC_MARKER}{CONTENT_SPACE_CHAR}{line}")
                }
            })
            .collect::<Vec<_>>()
            .join(LF_LINE_ENDING);
        let mut cst = SourceFileCst::parse(&raw_doc);
        for chunk in &mut cst.chunks {
            if let FileChunk::DocBlock(block) = chunk {
                for node in &mut block.nodes {
                    format_table_node(node);
                }
            }
        }
        cst.reconstruct()
            .lines()
            .map(|line| extract_comment_content(line, OUTER_DOC_MARKER))
            .collect::<Vec<_>>()
            .join(LF_LINE_ENDING)
    }

    #[test]
    fn test_simple_and_unaligned_table_formatting() {
        let input = "| Short | Very Long Text |\n|---|---|\n| 1 | 2 |";
        let output = format_tables(input);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(
            lines,
            vec![
                "| Short | Very Long Text |",
                "| ----- | -------------- |",
                "| 1     | 2              |",
            ]
        );
    }

    #[test]
    fn test_non_table_and_code_fence_pipe_content_unchanged() {
        let input = r"Some text before
```text
+---------------------+
|         ↑           |
|      within vp      |
+---------------------+
```
| A | B |
|---|---|
| 1 | 2 |
Some text after";
        let output = format_tables(input);
        assert!(output.contains("Some text before"));
        assert!(output.contains("|         ↑           |"));
        assert!(output.contains("|      within vp      |"));
        assert!(output.contains("| A   | B   |\n| --- | --- |\n| 1   | 2   |"));
        assert!(output.contains("Some text after"));
    }

    #[test]
    fn test_mixed_alignment_preserved() {
        let input =
            "| Left | Center | Right | None |\n|:---|:---:|---:|---|\n| A | B | C | D |";
        let output = format_tables(input);
        let lines: Vec<&str> = output.lines().collect();
        let separator = lines[1];
        let cells: Vec<&str> = separator
            .trim()
            .trim_start_matches('|')
            .trim_end_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        assert_eq!(cells.len(), 4);
        assert!(cells[0].starts_with(':') && !cells[0].ends_with(':'));
        assert!(cells[1].starts_with(':') && cells[1].ends_with(':'));
        assert!(!cells[2].starts_with(':') && cells[2].ends_with(':'));
        assert!(!cells[3].contains(':'));
    }

    #[test]
    fn test_indented_tables_preserve_spaces_and_tabs() {
        // Space-indented and tab-indented tables preserve their leading content_indent.
        let input = "| A | B |\n|---|---|\n| 1 | 2 |\n\n    | C | D |\n    |---|---|\n    | 3 | 4 |\n\n\t| E | F |\n\t|---|---|\n\t| 5 | 6 |";
        let output = format_tables(input);
        let lines: Vec<&str> = output.lines().collect();
        assert!(lines[0].starts_with("| A   | B   |"));
        assert!(lines[4].starts_with("    | C   | D   |"));
        assert!(lines[8].starts_with("\t| E   | F   |"));
    }

    #[test]
    fn test_format_table_node_modified_and_idempotent() {
        let source = "/// Header\n/// | Col A | Col B |\n/// |---|---|\n/// | 1 | 2 |\nfn test() {}\n";
        let mut cst = SourceFileCst::parse(source);
        let FileChunk::DocBlock(ref mut block) = cst.chunks[0] else {
            unreachable!();
        };
        assert_eq!(
            format_table_node(&mut block.nodes[0]),
            FormatStatus::Unchanged
        );
        assert_eq!(
            format_table_node(&mut block.nodes[1]),
            FormatStatus::Modified
        );

        let reconstructed = cst.reconstruct();
        assert!(reconstructed.contains("| Col A | Col B |"));
        assert!(reconstructed.contains("| ----- | ----- |"));

        // Second pass on already-formatted source returns Unchanged.
        let mut cst2 = SourceFileCst::parse(&reconstructed);
        let FileChunk::DocBlock(ref mut block2) = cst2.chunks[0] else {
            unreachable!();
        };
        assert_eq!(
            format_table_node(&mut block2.nodes[1]),
            FormatStatus::Unchanged
        );
    }
}
