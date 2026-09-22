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
3. **First-Class Type-Safe Spans (`CRow`, `CHeight`, `CCol`, `CWidth`)**: Every `DocNode`
   carries a `SourceSpan` (`start_line: CRow`, `end_line: CRow`,
   `line_count() -> CHeight`) in 0-based Canvas document coordinates (`r3bl_tui`),
   `LineRange` converts 1-based CLI line numbers into 0-based `CRow` at the `FromStr`
   parsing boundary ("Parse, don't validate"), and `TableData` stores column widths as
   `CWidth`.
4. **Unified Range Formatting (`--lines-force`)**:
    - Focuses rustdoc-fmt changes (tables and inline links) strictly to the specified
      lines in doc comments.
    - For inline links inside the range, converts them to reference-style `[text]` and
      appends/aggregates reference definitions to the enclosing doc block's bottom.
    - **Overrides/bypasses `// rustdoc-fmt: skip`**: Because the user explicitly supplied
      a line range targeting this file, our binary honors the explicit command and
      bypasses its own file-level skip directive.
    - **Decoupled from `#![rustfmt::skip]`**: `cargo-rustdoc-fmt` only checks
      `// rustdoc-fmt: skip` for skipping doc comment formatting, while `cargo fmt`
      natively handles `#![rustfmt::skip]` without redundant filtering.
5. **Elimination of Sentinel Placeholders & Legacy Pipeline**: Because code fences,
   tables, and paragraphs are distinct CST types, `ContentProtector`, `extractor.rs`, and
   `RustdocBlock` are completely removed.
6. **Byte-for-Byte Preservation & Domain Enums**: Unmodified nodes emit their original
   `raw_lines` directly during reassembly, using `LineEndings` (`Windows` / `Unix`) and
   `TrailingNewline` (`Present` / `Absent`) instead of raw strings or booleans, and
   `FormatStatus` (`Modified` / `Unchanged`) on `DocNode` (`status: FormatStatus`) and CST
   formatting mutators (without any `.is_*() -> bool` helpers).

## Implementation Plan

### [x] Phase 1: CST Data Structures & Parsing Engine

- [x] Define `SourceSpan`, `LineRange`, `LineRangeError`, `LineEndings`,
      `TrailingNewline`, `FormatStatus`, `FileChunk`, `DocBlockCst`, `DocNode` (with
      `status: FormatStatus` and `pub fn status(&self) -> FormatStatus`), `TableData`,
      `ColumnAlignment`, and `LinkReference` in
      `build-infra/src/cargo_rustdoc_fmt/types.rs`.
- [x] Implement `FromStr` for `LineRange` with comprehensive delimiter and boundary
      validation (`start:end`, `start-end`, `start..end`, `start..=end`, `single_line`).
- [x] Add `--lines-force` CLI argument (with `--lines` alias) and single-file target
      validation to `CLIArg` in `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs`.
- [x] Implement file partitioner `SourceFileCst::parse(source: &str) -> SourceFileCst` in
      `build-infra/src/cargo_rustdoc_fmt/cst.rs` to split files into `Code` and `DocBlock`
      chunks with line spans, `LineEndings`, and `TrailingNewline` tracking.
- [x] Implement `DocBlockCst::parse` in `cst.rs` to classify doc comment lines into
      `Paragraph`, `CodeFence`, `Table`, `ReferenceDefinitions` (supporting nested
      brackets and multi-line wrapped definitions), and `BlankLine`.
- [x] Implement lossless reassembly `SourceFileCst::reconstruct(&self) -> String` that
      emits `raw_lines` for unmodified nodes (`FormatStatus::Unchanged`) and reformatted
      lines for modified nodes (`FormatStatus::Modified`).
- [x] Unit tests for parsing, span assignment, and lossless round-trip reassembly on
      unmodified files (asserting exact 100% byte-for-byte preservation).

### [x] Phase 2: Table Formatter & Range Formatting (`--lines-force`)

- [x] Port `table_formatter.rs` to parse table rows and alignments into `TableData` with
      GFM separator row syntax validation.
- [x] Implement `format_table_node(table: &mut DocNode) -> FormatStatus` to compute
      display widths via `unicode_width` and format aligned columns.
- [x] Integrate `--lines-force` range overlap filtering into CST traversal: only nodes
      whose spans overlap `line_range` are transformed.
- [x] Bypass `// rustdoc-fmt: skip` when `--lines-force` is active.
- [x] Decouple `#![rustfmt::skip]` from `cargo-rustdoc-fmt` (`find_rustfmt_skip_line` and
      `has_rustfmt_skip` removed; `cargo fmt` handles `#![rustfmt::skip]` natively).
- [x] Unit and integration tests for table formatting: full-file mode, range-targeted
      mode, multi-table files, unicode/emoji cell widths, and non-overlapping ranges.

### [x] Phase 3: Link Converter & Reference Aggregation

- [x] Port `link_converter.rs` to operate on `DocBlockCst`, converting inline
      `[text](url)` links to reference style `[text]` while ignoring backtick spans.
- [x] Aggregate and sort extracted links and scattered reference definitions into
      `DocNode::ReferenceDefinitions` at the bottom of the `DocBlockCst`, supporting range
      filtering via `--lines-force`.
- [x] Delete `build-infra/src/cargo_rustdoc_fmt/content_protector.rs` and remove all
      Unicode sentinel placeholder mechanics.
- [x] Unit and integration tests for link conversion, reference aggregation, and code
      fence isolation.

### [x] Phase 4: Technical Term Linker & Full Pipeline Integration

- [x] Port `technical_term_linker.rs` to visit `DocBlockCst` nodes using the
      `TechnicalTermDictionary` (`link_terms_in_doc_block(...) -> FormatStatus`).
- [x] Unify `FileProcessor::process_file` to execute the complete CST pipeline across
      tables, links, and term linking (both full-file and `--lines-force` modes).
- [x] Delete legacy `build-infra/src/cargo_rustdoc_fmt/extractor.rs` and `RustdocBlock`.
- [x] Run and update comprehensive validation tests in
      `build-infra/src/cargo_rustdoc_fmt/validation_tests/complete_file_tests.rs`.

### [x] Phase 5: Changelog, Verification & Tool Installation

- [x] **Update `CHANGELOG.md` (Accumulate for Upcoming Unreleased `v0.0.6`)**:
    - Under `## r3bl-build-infra` -> `### v0.0.6 (2026-09-18)` -> `**Added:**`, update:
        - `--lines-force <START>:<END>` argument allowing surgical range formatting of
          tables and inline links in doc comments.
        - Automatic `// rustdoc-fmt: skip` bypass when `--lines-force` is supplied.
        - Decoupled `#![rustfmt::skip]` from `cargo-rustdoc-fmt`.
        - Strict single-file target validation (`paths.len() == 1`, `.rs` extension, no
          `--workspace`).
- [x] Audit codebase for `// rustdoc-fmt: skip`: ensure all files in
      `build-infra/src/cargo_rustdoc_fmt/*.rs` (including `link_converter.rs`) have it,
      and verify zero occurrences in production crates (`tui/`, `cmdr/`,
      `rust-analyzer-mcp-server/`).
- [x] Run `./check.fish --check`.
- [x] Run `./check.fish --clippy`.
- [x] Run `./check.fish --fmt`.
- [x] Run `./check.fish --quick-doc`.
- [x] Run `./check.fish --test`.
- [x] Verify manual test:
      `cargo run -p r3bl-build-infra --bin cargo-rustdoc-fmt -- --lines-force 2404:2410 tui/src/lib.rs`.
- [x] Update installed binary: `cargo install --path build-infra --force`.

### [x] Phase 6: Type-Safe Canvas Coordinates, Domain Enums & Final Review

- [x] Add `RangeOverlapResult { Overlaps, Disjoint }` witness enum to
      `tui/src/core/coordinates/bounds_check/result_enums.rs` and add
      `overlaps(&self, other: &Self) -> RangeOverlapResult` and
      `to_len(&self) -> Self::LengthType` to `RangeBoundsExt` in
      `tui/src/core/coordinates/bounds_check/range/range_bounds_check.rs` (for both
      `RangeExclusive<I>` and `RangeInclusive<I>`), with unit tests.
- [x] Introduce 1-based `EditorLineNum` (`editor_line_num`) newtype in `types.rs` with
      `.to_c_row(self) -> CRow` and `.to_c_row_range(start, end) -> RangeInclusive<CRow>`.
- [x] Replace both `SourceSpan` and `LineRange` with `RangeInclusive<CRow>` across
      `build-infra/src/cargo_rustdoc_fmt/` and move `LineRangeError`, delimiter/prefix
      constants (`RANGE_DELIM_DOT_DOT_EQ`, `RANGE_DELIM_DOT_DOT`, `RANGE_DELIM_COLON`,
      `RANGE_DELIM_HYPHEN`, `RANGE_DELIMITERS`, `NEGATIVE_NUMBER_PREFIX`), and
      `parse_line_range` to `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs`
      (`value_parser = parse_line_range`).
- [x] Eliminate boolean blindness in `FormatOptions` and `ProcessingResult` in
      `types.rs`: - Replace the 6 `bool` fields in `FormatOptions` with domain enums
      (`PassPolicy`, `ExecutionMode`, `Verbosity`) and remove
      `#[allow(clippy::struct_excessive_bools)]`. - Replace `pub modified: bool` in
      `ProcessingResult` with `pub status: FormatStatus`.
- [x] Refactor `TableData` in `types.rs` and table width/column indexing in
      `build-infra/src/cargo_rustdoc_fmt/cst.rs` to use `CWidth`
      (`separator_widths: Vec<CWidth>`) and `CCol` / `CRow` with
      `ArrayBoundsCheck::overflows` instead of raw `usize`, eliminating `+ 1` / `- 1`
      index adjustments in `SourceFileCst::parse` and `DocBlockCst::parse`.
- [x] Update all call sites and unit/validation tests in `types.rs`, `cst.rs`,
      `cli_arg.rs`, `processor.rs`, `link_converter.rs`, `technical_term_linker.rs`,
      `cargo-rustdoc-fmt.rs`, `table_formatter.rs`, and
      `validation_tests/complete_file_tests.rs`.
- [x] Run `./check.fish --check`, `./check.fish --clippy`,
      `cargo test -p r3bl_tui --lib range`, `cargo test -p r3bl-build-infra --lib`, and
      `cargo install --path build-infra --force`.
- [x] **Mandatory manual review:** Verify all modified files and build state across the
      entire task:
    - [x] `tui/src/core/coordinates/bounds_check/result_enums.rs`
    - [x] `tui/src/core/coordinates/bounds_check/range/range_bounds_check.rs`
    - [x] `tui/src/core/coordinates/bounds_check/range/range_construct_ext.rs`
    - [x] `tui/src/core/coordinates/bounds_check/length_ops.rs`
    - [x] `tui/src/core/coordinates/bounds_check/mod.rs`
    - [x] `tui/src/core/coordinates/byte/byte_length.rs`
    - [x] `.harper-dictionary.txt`
    - [x] `build-infra/Cargo.toml`
    - [x] `build-infra/src/cargo_rustdoc_fmt/constants.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/types.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/cst/mod.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/cst/source_file_cst.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/cst/doc_block_cst.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/cst/table_data.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/cst/link_reference.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/formatters/mod.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/formatters/code_fence.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/formatters/table.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/formatters/link.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/formatters/technical_term.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/formatters/technical_term_dictionary.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/formatters/known_technical_term_link_dictionary.jsonc`
    - [x] `build-infra/src/cargo_rustdoc_fmt/processor.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/mod.rs`
    - [x] `build-infra/src/bin/cargo-rustdoc-fmt.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/validation_tests/complete_file_tests.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/validation_tests/mod.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output/sample_real_world.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/input/sample_seed_term_coverage.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output/sample_seed_term_coverage.rs`
    - [x] `build-infra/src/common/workspace_utils.rs`
    - [x] `build-infra/AGENTS.md`
    - [x] `build-infra/README.md`
    - [x] `build-infra/src/lib.rs`
    - [x] `CHANGELOG.md`
    - [x] `docs/release-notes/r3bl-build-infra/v0.0.6.md`
    - [x] `check_cargo.fish`
    - [x] `check.fish`
    - [x] `check_cli.fish`
    - [x] `AGENTS.md`
    - [x] `.agents/skills/design-philosophy/SKILL.md`
    - [x] `.agents/skills/design-philosophy/patterns.md`
    - [x] `tui/src/lib.rs`
    - [x] `build-infra/` (verify clean status and installed binary)
