// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// rustdoc-fmt: skip

//! Lossless Concrete Syntax Tree (`CST`) parser and reassembler for Rust source files.
//!
//! Partitions Rust source code into [`FileChunk::Code`] and [`FileChunk::DocBlock`].
//! Inside each doc comment block, classifies lines into typed [`DocNode`]s (`Paragraph`,
//! `CodeFence`, `Table`, `ReferenceDefinitions`, `BlankLine`), each tracking its exact
//! 0-based [`RangeInclusive<CRow>`] span.
//!
//! [`RangeInclusive<CRow>`]: std::ops::RangeInclusive

mod doc_block_cst;
mod link_reference;
mod source_file_cst;
mod table_data;

pub use doc_block_cst::*;
pub use link_reference::*;
pub use source_file_cst::*;
pub use table_data::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cargo_rustdoc_fmt::FormatStatus;
    use r3bl_tui::c_row;

    #[test]
    fn test_lossless_roundtrip_pure_code() {
        let source = "fn main() {\n    println!(\"Hello\");\n}\n";
        let cst = SourceFileCst::parse(source);
        assert_eq!(cst.reconstruct(), source);
    }

    #[test]
    fn test_lossless_roundtrip_outer_doc() {
        let source = "/// Outer doc\n/// Second line\nfn foo() {}\n";
        let cst = SourceFileCst::parse(source);
        assert_eq!(cst.reconstruct(), source);
    }

    #[test]
    fn test_lossless_roundtrip_inner_doc() {
        let source = "//! Module doc\n//! Second line\n\nfn bar() {}\n";
        let cst = SourceFileCst::parse(source);
        assert_eq!(cst.reconstruct(), source);
    }

    #[test]
    fn test_lossless_roundtrip_crlf() {
        let source = "/// Doc with CRLF\r\n/// Second line\r\nfn crlf() {}\r\n";
        let cst = SourceFileCst::parse(source);
        assert_eq!(cst.reconstruct(), source);
    }

    #[test]
    fn test_lossless_roundtrip_no_trailing_newline() {
        let source = "/// Doc without newline\nfn eof() {}";
        let cst = SourceFileCst::parse(source);
        assert_eq!(cst.reconstruct(), source);
    }

    #[test]
    fn test_lossless_roundtrip_table_and_fences() {
        let source = "/// Documentation\n///\n/// ```rust\n/// let x = 1;\n/// ```\n///\n/// | Col A | Col B |\n/// |:------|------:|\n/// | 1     | 2     |\n///\n/// [label]: url\nfn test() {}\n";
        let cst = SourceFileCst::parse(source);
        assert_eq!(cst.reconstruct(), source);

        // Verify CST nodes in doc block.
        if let FileChunk::DocBlock(block) = &cst.chunks[0] {
            assert_eq!(block.nodes.len(), 7);
            assert!(matches!(block.nodes[0], DocNode::Paragraph { .. }));
            assert!(matches!(block.nodes[1], DocNode::BlankLine { .. }));
            assert!(matches!(block.nodes[2], DocNode::CodeFence { .. }));
            assert!(matches!(block.nodes[3], DocNode::BlankLine { .. }));
            assert!(matches!(block.nodes[4], DocNode::Table { .. }));
            assert!(matches!(block.nodes[5], DocNode::BlankLine { .. }));
            assert!(matches!(
                block.nodes[6],
                DocNode::ReferenceDefinitions { .. }
            ));
        } else {
            panic!("Expected DocBlock chunk");
        }
    }

    #[test]
    fn test_source_spans_accuracy() {
        let source = "fn line1() {}\n/// Line 2\n/// Line 3\nfn line4() {}\n";
        let cst = SourceFileCst::parse(source);

        assert_eq!(cst.chunks.len(), 3);
        assert_eq!(cst.chunks[0].span(), c_row(0)..=c_row(0));
        assert_eq!(cst.chunks[1].span(), c_row(1)..=c_row(2));
        assert_eq!(cst.chunks[2].span(), c_row(3)..=c_row(3));
    }

    #[test]
    fn test_split_reference_def_label_and_rest() {
        assert_eq!(
            split_reference_def_label_and_rest(
                "[`SmallString<[u8; 64]>`]: `smallstr::SmallString`"
            ),
            Some(("`SmallString<[u8; 64]>`", "`smallstr::SmallString`"))
        );
        assert_eq!(
            split_reference_def_label_and_rest("  [incomplete]:  "),
            Some(("incomplete", ""))
        );
        assert_eq!(split_reference_def_label_and_rest("[]: target"), None);
        assert_eq!(
            split_reference_def_label_and_rest("[missing colon] target"),
            None
        );
    }

    #[test]
    fn test_find_backtick_span() {
        assert_eq!(find_backtick_span("abc", 0), (0, None));
        assert_eq!(find_backtick_span("`code` rest", 0), (1, Some(6)));
        assert_eq!(find_backtick_span("`` `code` `` rest", 0), (2, Some(12)));
        assert_eq!(find_backtick_span("`unclosed", 0), (1, None));
    }

    #[test]
    fn test_empty_source_and_adjacent_mismatched_doc_blocks() {
        let empty_cst = SourceFileCst::parse("");
        assert!(empty_cst.chunks.is_empty());
        assert_eq!(empty_cst.reconstruct(), "");

        // Adjacent `//!` and `///` lines must split into two separate DocBlock chunks.
        let mixed =
            "//! Inner doc\n/// Outer doc\n    /// Indented outer doc\nfn item() {}\n";
        let cst = SourceFileCst::parse(mixed);
        assert_eq!(cst.chunks.len(), 4);
        assert_eq!(cst.reconstruct(), mixed);

        // Fallback branch of extract_comment_content when line does not start with
        // marker.
        assert_eq!(
            extract_comment_content("fn plain_code() {}", "///"),
            "fn plain_code() {}"
        );
    }

    #[test]
    fn test_titled_reference_def_and_prose_colon_rejection() {
        let raw_lines = vec!["/// [label]: https://example.com".to_string()];
        assert_eq!(try_parse_reference_def(&raw_lines, 5, "///"), None);

        let source = "/// See [example].\n///\n/// [example]: https://example.com \"Example Title\"\n/// [not a ref]: this has unquoted spaces so it is prose\n";
        let mut cst = SourceFileCst::parse(source);
        assert_eq!(cst.reconstruct(), source);

        if let FileChunk::DocBlock(ref mut block) = cst.chunks[0] {
            assert!(matches!(
                &block.nodes[2],
                DocNode::ReferenceDefinitions { definitions, .. }
                    if definitions[0].title.as_deref() == Some("Example Title")
                        && definitions[0].render()
                            == "[example]: https://example.com \"Example Title\""
            ));
            assert!(matches!(&block.nodes[3], DocNode::Paragraph { .. }));

            // Mark the ReferenceDefinitions node Modified to exercise modified
            // reconstruction of titled reference definitions.
            if let DocNode::ReferenceDefinitions { status, .. } = &mut block.nodes[2] {
                *status = FormatStatus::Modified;
            }
        } else {
            panic!("Expected DocBlock");
        }
        assert_eq!(cst.reconstruct(), source);
    }

    #[test]
    fn test_table_alignments_and_invalid_separators() {
        assert_eq!(is_separator_row(""), SeparatorRowStatus::Invalid);
        assert_eq!(is_separator_row("| --- |  |"), SeparatorRowStatus::Invalid);
        assert_eq!(
            is_separator_row("| ::: | --- |"),
            SeparatorRowStatus::Invalid
        );
        assert_eq!(
            is_separator_row("| -a- | --- |"),
            SeparatorRowStatus::Invalid
        );
        assert_eq!(parse_table_row("no_pipes"), vec!["no_pipes".to_string()]);
        assert!(
            parse_table_lines(&["| only_one_line |".to_string()])
                .headers
                .is_empty()
        );
        assert!(
            format_table_data(&TableData {
                headers: Vec::new(),
                alignments: Vec::new(),
                separator_widths: Vec::new(),
                rows: Vec::new(),
            })
            .is_empty()
        );

        // Non-table pipe lines (second line lacks pipes or has mismatched column count).
        let non_table_1 = vec!["/// | A | B |".to_string(), "/// plain line".to_string()];
        assert_eq!(
            is_table_start(&non_table_1, 0, "///"),
            TableStartStatus::NotTable
        );
        let non_table_2 = vec!["/// | A | B |".to_string(), "/// | --- |".to_string()];
        assert_eq!(
            is_table_start(&non_table_2, 0, "///"),
            TableStartStatus::NotTable
        );

        let lines = vec![
            "| Left | Center | Right | None |".to_string(),
            "|:---|:---:|---:|---|".to_string(),
            "| a | b | c | d |".to_string(),
        ];
        let table = parse_table_lines(&lines);
        assert_eq!(
            table.alignments,
            vec![
                ColumnAlignment::Left,
                ColumnAlignment::Center,
                ColumnAlignment::Right,
                ColumnAlignment::None,
            ]
        );
        let formatted = format_table_data(&table);
        assert_eq!(
            formatted,
            vec![
                "| Left | Center | Right | None |",
                "| :--- | :----: | ----: | ---- |",
                "| a    | b      |     c | d    |",
            ]
        );
    }
}
