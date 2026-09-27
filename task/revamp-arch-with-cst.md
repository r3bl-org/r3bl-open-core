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
4. **Unified Range Formatting (`--lines-force`)**:
    - Focuses rustdoc-fmt changes (tables and inline links) strictly to the specified
      lines in doc comments.
    - For inline links inside the range, converts them to reference-style `[text]` and
      appends/aggregates reference definitions to the enclosing doc block's bottom.
    - **Overrides/bypasses `// rustdoc-fmt: skip`**: Because the user explicitly supplied
      a line range targeting this file, our binary honors the explicit command and
      bypasses its own file-level skip directive.
    - **Respects `#![rustfmt::skip]`**: If `#![rustfmt::skip]` is present, `cargo fmt` is
      suppressed to prevent clobbering hand-aligned code. If absent (and not
      `--skip-cargo-fmt`), `cargo fmt` is run on the modified file.
5. **Elimination of Sentinel Placeholders**: Because code fences, tables, and paragraphs
   are distinct CST types, `ContentProtector` and its Unicode sentinel strings
   (`\u{25C4}BTCK...`) are completely removed.
6. **Byte-for-Byte Preservation**: Unmodified nodes emit their original `raw_lines`
   directly during reassembly.

## Implementation Plan

### [x] Phase 1: CST Data Structures & Parsing Engine

- [x] Define `SourceSpan`, `LineRange`, `LineRangeError`, `FileChunk`, `DocBlockCst`,
      `DocNode`, `TableData`, `ColumnAlignment`, and `LinkReference` in
      `build-infra/src/cargo_rustdoc_fmt/types.rs`.
- [x] Implement `FromStr` for `LineRange` with comprehensive delimiter and boundary
      validation (`start:end`, `start-end`, `start..end`, `start..=end`, `single_line`).
- [x] Add `--lines-force` CLI argument (with `--lines` alias) and single-file target
      validation to `CLIArg` in `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs`.
- [x] Implement file partitioner `SourceFileCst::parse(source: &str) -> SourceFileCst` in
      `build-infra/src/cargo_rustdoc_fmt/cst.rs` to split files into `Code` and `DocBlock`
      chunks with line spans, newline detection (`\r\n` vs `\n`), and trailing newline
      tracking.
- [x] Implement `DocBlockCst::parse` in `cst.rs` to classify doc comment lines into
      `Paragraph`, `CodeFence`, `Table`, `ReferenceDefinitions`, and `BlankLine`.
- [x] Implement lossless reassembly `SourceFileCst::reconstruct(&self) -> String` that
      emits `raw_lines` for unmodified nodes and reformatted lines for modified nodes.
- [x] Unit tests for parsing, span assignment, and lossless round-trip reassembly on
      unmodified files (asserting exact 100% byte-for-byte preservation).

### [x] Phase 2: Table Formatter & Range Formatting (`--lines-force`)

- [x] Port `table_formatter.rs` to parse table rows and alignments into `TableData` with
      GFM separator row syntax validation.
- [x] Implement `format_table_node(table: &mut DocNode)` to compute display widths via
      `unicode_width` and format aligned columns.
- [x] Integrate `--lines-force` range overlap filtering into CST traversal: only nodes
      whose spans overlap `line_range` are transformed.
- [x] Bypass `// rustdoc-fmt: skip` when `--lines-force` is active.
- [x] Respect `#![rustfmt::skip]`: suppress `cargo fmt` when `#![rustfmt::skip]` is
      present, but allow `cargo fmt` to run when absent and not `--skip-cargo-fmt`.
- [x] Unit and integration tests for table formatting: full-file mode, range-targeted
      mode, multi-table files, unicode/emoji cell widths, and non-overlapping ranges.

### [x] Phase 3: Link Converter & Reference Aggregation

- [x] Port `link_converter.rs` to operate exclusively on `DocNode::Paragraph` nodes,
      converting inline `[text](url)` links to reference style `[text]` while ignoring
      backtick spans.
- [x] Aggregate and sort extracted links into `DocNode::ReferenceDefinitions` at the
      bottom of the `DocBlockCst`, supporting range filtering via `--lines-force`.
- [x] Delete `build-infra/src/cargo_rustdoc_fmt/content_protector.rs` and remove all
      Unicode sentinel placeholder mechanics.
- [x] Unit and integration tests for link conversion, reference aggregation, and code
      fence isolation.

### [x] Phase 4: Technical Term Linker & Full Pipeline Integration

- [x] Port `technical_term_linker.rs` to visit `DocNode::Paragraph` nodes using the
      `TechnicalTermDictionary`.
- [x] Unify `FileProcessor::process_file` to execute the complete CST pipeline across
      tables, links, and term linking.
- [x] Run and update comprehensive validation tests in
      `build-infra/src/cargo_rustdoc_fmt/validation_tests/complete_file_tests.rs`.

### [ ] Phase 5: Changelog, Verification & Tool Installation

- [x] **Update `CHANGELOG.md` (Accumulate for Upcoming Unreleased `v0.0.6`)**:
    - Under `## r3bl-build-infra` -> `### v0.0.6 (2026-09-18)` -> `**Added:**`, update:
        - `--lines-force <START>:<END>` argument allowing surgical range formatting of
          tables and inline links in doc comments.
        - Automatic `// rustdoc-fmt: skip` bypass when `--lines-force` is supplied.
        - Strict respecting of `#![rustfmt::skip]` (suppresses `cargo fmt` if present,
          runs `cargo fmt` if absent).
        - Strict single-file target validation (`paths.len() == 1`, `.rs` extension, no
          `--workspace`).
- [x] Run `./check.fish --check`.
- [x] Run `./check.fish --clippy`.
- [x] Run `./check.fish --fmt`.
- [x] Run `./check.fish --quick-doc`.
- [x] Run `./check.fish --test`.
- [x] Verify manual test:
      `cargo run -p r3bl-build-infra --bin cargo-rustdoc-fmt -- --lines-force 2404:2410 tui/src/lib.rs`.
- [x] Update installed binary: `cargo install --path build-infra --force`.
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
