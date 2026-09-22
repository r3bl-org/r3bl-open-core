# Task: Revamp `cargo-rustdoc-fmt` Architecture with Lossless CST

## Overview

Refactor `cargo-rustdoc-fmt` around a lossless Concrete Syntax Tree (CST) architecture.
The previous architecture joined entire doc comment blocks into monolithic strings and
applied global regex transformations with ad-hoc placeholder shields (`ContentProtector`).
Because nodes lacked source line spans, adding range formatting (`--lines`) was complex
and required an isolated bypass path.

This clean-break refactoring establishes a two-level CST:

1. **File Partitioning**: The source file is partitioned into `FileChunk::Code` and
   `FileChunk::DocBlock`. Non-doc code is preserved 100% byte-for-byte.
2. **DocComment CST**: Contiguous doc comment blocks (`///` or `//!`) are parsed into
   strongly-typed `DocNode`s (`Paragraph`, `CodeFence`, `Table`, `ReferenceDefinitions`,
   `BlankLine`).
3. **First-Class Spans**: Every `DocNode` carries a 1-indexed `SourceSpan`
   (`start_line..=end_line`).
4. **Unified Range Formatting (`--lines`)**: Range formatting becomes a simple predicate
   (`node.span.overlaps(&range)`). Unmatched nodes remain untouched, eliminating the need
   for separate bypass execution paths.
5. **Elimination of Sentinel Placeholders**: Because code fences, tables, and paragraphs
   are distinct CST types, `ContentProtector` and its Unicode sentinel strings
   (`\u{25C4}BTCK...`) are completely removed.
6. **Byte-for-Byte Preservation**: Unmodified nodes emit their original `raw_lines`
   directly during reassembly.

## Implementation Plan

### [ ] Phase 1: CST Data Structures & Parsing Engine

- [ ] Define `SourceSpan`, `LineRange`, `LineRangeError`, `FileChunk`, `DocBlockCst`,
      `DocNode`, `TableData`, `ColumnAlignment`, and `LinkReference` in
      `build-infra/src/cargo_rustdoc_fmt/types.rs`.
- [ ] Implement `FromStr` for `LineRange` with comprehensive delimiter and boundary
      validation (`start:end`, `start-end`, `start..end`, `start..=end`, `single_line`).
- [ ] Add `--lines` CLI argument and single-file target validation to `CLIArg` in
      `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs`.
- [ ] Implement file partitioner `SourceFileCst::parse(source: &str) -> SourceFileCst` in
      `build-infra/src/cargo_rustdoc_fmt/cst.rs` to split files into `Code` and `DocBlock`
      chunks with line spans, newline detection (`\r\n` vs `\n`), and trailing newline
      tracking.
- [ ] Implement `DocBlockCst::parse` in `cst.rs` to classify doc comment lines into
      `Paragraph`, `CodeFence`, `Table`, `ReferenceDefinitions`, and `BlankLine`.
- [ ] Implement lossless reassembly `SourceFileCst::reconstruct(&self) -> String` that
      emits `raw_lines` for unmodified nodes and reformatted lines for modified nodes.
- [ ] Unit tests for parsing, span assignment, and lossless round-trip reassembly on
      unmodified files (asserting exact 100% byte-for-byte preservation).

### [ ] Phase 2: Table Formatter & Range Formatting (`--lines`)

- [ ] Port `table_formatter.rs` to parse table rows and alignments into `TableData` with
      GFM separator row syntax validation.
- [ ] Implement `format_table_node(table: &mut DocNode)` to compute display widths via
      `unicode_width` and format aligned columns.
- [ ] Integrate `--lines` range overlap filtering into CST traversal: only nodes whose
      spans overlap `line_range` are transformed.
- [ ] Add `#![rustfmt::skip]` bypass in CST traversal when `--lines` is supplied while
      respecting `// rustdoc-fmt: skip`.
- [ ] Suppress whole-file `cargo fmt` execution when `--lines` is active.
- [ ] Unit and integration tests for table formatting: full-file mode, range-targeted
      mode, multi-table files, unicode/emoji cell widths, and non-overlapping ranges.

### [ ] Phase 3: Link Converter & Reference Aggregation

- [ ] Port `link_converter.rs` to operate exclusively on `DocNode::Paragraph` nodes,
      converting inline `[text](url)` links to reference style `[text]` while ignoring
      backtick spans.
- [ ] Aggregate and sort extracted links into `DocNode::ReferenceDefinitions` at the
      bottom of the `DocBlockCst`.
- [ ] Delete `build-infra/src/cargo_rustdoc_fmt/content_protector.rs` and remove all
      Unicode sentinel placeholder mechanics.
- [ ] Unit and integration tests for link conversion, reference aggregation, and code
      fence isolation.

### [ ] Phase 4: Technical Term Linker & Full Pipeline Integration

- [ ] Port `technical_term_linker.rs` to visit `DocNode::Paragraph` nodes using the
      `TechnicalTermDictionary`.
- [ ] Unify `FileProcessor::process_file` to execute the complete CST pipeline across
      tables, links, and term linking.
- [ ] Run and update comprehensive validation tests in
      `build-infra/src/cargo_rustdoc_fmt/validation_tests/complete_file_tests.rs`.

### [ ] Phase 5: Changelog, Verification & Tool Installation

- [ ] **Update `CHANGELOG.md` (Accumulate for Upcoming Unreleased `v0.0.6`)**:
    - Under `## r3bl-build-infra` -> `### v0.0.6 (2026-09-18)` -> `**Added:**`, add:
        - `--lines <START>:<END>` argument allowing surgical range formatting of markdown
          tables in doc comments.
        - Automatic `#![rustfmt::skip]` bypass when `--lines` is supplied.
        - Automatic whole-file `cargo fmt` skip to preserve untouched lines outside range.
        - Strict single-file target validation (`paths.len() == 1`, `.rs` extension, no
          `--workspace`).
- [ ] Run `./check.fish --check`.
- [ ] Run `./check.fish --clippy`.
- [ ] Run `./check.fish --fmt`.
- [ ] Run `./check.fish --quick-doc`.
- [ ] Run `./check.fish --test`.
- [ ] Verify manual test:
      `cargo run -p r3bl-build-infra --bin cargo-rustdoc-fmt -- --lines 2404:2410 tui/src/lib.rs`.
- [ ] Update installed binary: `cargo install --path build-infra --force`.
- [ ] **Mandatory manual review:** Verify all modified files and build state across the
      entire task:
    - [ ] `build-infra/src/cargo_rustdoc_fmt/types.rs`
    - [ ] `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs`
    - [ ] `build-infra/src/cargo_rustdoc_fmt/cst.rs`
    - [ ] `build-infra/src/cargo_rustdoc_fmt/table_formatter.rs`
    - [ ] `build-infra/src/cargo_rustdoc_fmt/link_converter.rs`
    - [ ] `build-infra/src/cargo_rustdoc_fmt/technical_term_linker.rs`
    - [ ] `build-infra/src/cargo_rustdoc_fmt/processor.rs`
    - [ ] `build-infra/src/cargo_rustdoc_fmt/mod.rs`
    - [ ] `build-infra/src/bin/cargo-rustdoc-fmt.rs`
    - [ ] `build-infra/src/cargo_rustdoc_fmt/validation_tests/complete_file_tests.rs`
    - [ ] `CHANGELOG.md`
    - [ ] `build-infra/` (verify clean status and installed binary)
