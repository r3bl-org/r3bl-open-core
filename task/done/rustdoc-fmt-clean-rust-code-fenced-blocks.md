# Plan: Add Fence Normalizer to `cargo rustdoc-fmt`

## Context

Rustdoc defaults to Rust for unlabeled code fences, making ` ```rust ` redundant. The
codebase has ~160 occurrences across ~66 source files. We need:

1. A new `cargo rustdoc-fmt` CST transformation to strip redundant `rust` from fence
   openings
2. Updated test fixtures to match the new behavior
3. A codebase-wide run of the formatter to apply the fix

## Transformation Rules

- ` ```rust ` -> ` ``` ` (standalone)
- ` ```rust,<attr> ` -> ` ```<attr> ` (e.g., `rust,ignore` -> `ignore`)
- Non-rust fences unchanged (` ```text `, ` ```python `, bare ` ``` `)

## Steps

### 1. Create `fence_normalizer.rs`

**New file**: `build-infra/src/cargo_rustdoc_fmt/fence_normalizer.rs`

- [x] Add `// rustdoc-fmt: skip` at the top of the file (matching all sibling modules in
      `cargo_rustdoc_fmt/`).
- [x] CST node mutator:
      `pub fn normalize_code_fence_node(node: &mut DocNode) -> FormatStatus`
    - Matches `DocNode::CodeFence { language, lines, status, .. }`.
    - Checks if the opening fence line (`lines[0]`) has a redundant `rust` tag
      (` ```rust ` or ` ```rust,<attr> `).
    - If matched, updates `lines[0]` (preserving comment prefix/indentation) and
      `language`, sets `*status = FormatStatus::Modified`, and returns
      `FormatStatus::Modified`; otherwise returns `FormatStatus::Unchanged`.
- [x] Pure text helper (for direct unit testing):
      `#[must_use] pub fn normalize_fences(text: &str) -> String`
    - Uses `LazyLock<Regex>` with `^```rust(,.+)?$` (anchored, no false positives on e.g.
      ` ```rustic `).
- [x] Unit tests covering: standalone ` ```rust `, comma-separated attrs
      (` ```rust,ignore `, ` ```rust,no_run `), non-rust fences, empty input, no false
      positive on `rustic`, and `DocNode::CodeFence` CST mutation returning
      `FormatStatus::Modified` / `FormatStatus::Unchanged`.

### 2. Register module in `mod.rs`

**File**: `build-infra/src/cargo_rustdoc_fmt/mod.rs`

- [x] Add `pub mod fence_normalizer;`
- [x] Add `pub use fence_normalizer::*;` (in re-export block)
- [x] Update `build-infra/AGENTS.md` project structure tree to list `fence_normalizer.rs`.

### 3. Update `DocNode::CodeFence` and `FormatOptions` in `types.rs` and `cst.rs`

**Files**: `build-infra/src/cargo_rustdoc_fmt/types.rs` and
`build-infra/src/cargo_rustdoc_fmt/cst.rs`

- [x] In `types.rs`:
    - Add `status: FormatStatus` to `DocNode::CodeFence` and return `*status` for
      `Self::CodeFence { status, .. }` in `DocNode::status(&self)`.
    - Add `pub normalize_fences: bool` (defaults to `true`) to `FormatOptions`.
    - Update `Default` impl and `test_format_options_default` assertion.
- [x] In `cst.rs`:
    - Initialize `status: FormatStatus::Unchanged` when constructing `DocNode::CodeFence`
      in `DocBlockCst::parse`.

### 4. Add `--fences-only` CLI flag

**File**: `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs`

- [x] New `CLIArg` field: `pub fences_only: bool` with `#[arg(long)]`
    - Keep `--fences-only` compatible with `--lines-force` (just like `--tables-only` and
      `--links-only`; do NOT add `"fences_only"` to `lines_force`'s
      `conflicts_with_all = ["workspace", "terms_only", "skip_folder_matches"]`).
- [x] Mutual exclusion in `to_format_options()` (for both normal and `--lines-force`
      modes):
    ```text
    format_tables:    !links_only && !terms_only && !fences_only
    convert_links:    !tables_only && !terms_only && !fences_only
    link_terms:       !tables_only && !links_only && !fences_only (always false when lines_force is Some)
    normalize_fences: !tables_only && !links_only && !terms_only
    ```
- [x] Update `test_cli_default` (`!cli.fences_only`) and add `test_cli_fences_only` test
- [x] Update `long_about` help text to mention fence normalization

### 5. Wire into `processor.rs` CST pipeline

**File**: `build-infra/src/cargo_rustdoc_fmt/processor.rs`

- [x] Add `fence_normalizer` to the top-level `use crate::cargo_rustdoc_fmt::{...}`
      import.
- [x] Inside `FileProcessor::process_file`'s `FileChunk::DocBlock(block)` loop, run
      `fence_normalizer::normalize_code_fence_node(node)` on nodes overlapping `range`
      (supporting both full-file and `--lines-force` modes):
    ```rust
    if self.options.normalize_fences {
        for node in &mut block.nodes {
            if range.is_none_or(|r| node.span().overlaps(r))
                && fence_normalizer::normalize_code_fence_node(node)
                    == FormatStatus::Modified
            {
                modified = true;
            }
        }
    }
    ```

### 6. Update 2 golden test expected outputs

These are intentional changes per `validation_tests/mod.rs` (or regenerate via
`regen_golden_output_snapshot_files_for_input_files`).

**Input files stay unchanged** (they represent the "before" state).

- [x] Update expected output fixtures:

| File (`validation_tests/test_data/complete_file/expected_output/`) | Line | Before               | After           |
| :----------------------------------------------------------------- | :--: | :------------------- | :-------------- |
| `sample_input_event.rs`                                            | 125  | `/// ```rust`        | `/// ``` `      |
| `sample_code_fence_comma.rs`                                       |  29  | `//! ```rust,ignore` | `//! ```ignore` |

### 7. Update test assertions and `FormatOptions` in `complete_file_tests.rs`

**File**: `build-infra/src/cargo_rustdoc_fmt/validation_tests/complete_file_tests.rs`

- [x] In `test_code_fence_comma_language_tag` (around line 659): Change
      `formatted.contains("```rust,ignore")` -> `formatted.contains("```ignore")`
- [x] Update assertion message and test doc comment
- [x] Add `normalize_fences: false` (or `..Default::default()`) to manually-constructed
      `FormatOptions` literals in `complete_file_tests.rs`

### 8. Add new test fixture pair for fence normalization

- [x] **Input**:
      `build-infra/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/input/sample_fence_normalizer.rs`
    - Synthetic file with ` ```rust `, ` ```rust,ignore `, ` ```rust,no_run `,
      ` ```text `, bare ` ``` `
- [x] **Expected output**:
      `build-infra/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output/sample_fence_normalizer.rs`
    - Same file with `rust` stripped from fence openings
- [x] **Test function** in `complete_file_tests.rs`:
      `test_fence_normalizer_strips_redundant_rust()`

### 9. Build, test, install

- [x] Run build, test, clippy, and install:

```bash
./check.fish --check
./check.fish --build
./check.fish --test
./check.fish --clippy
cargo install --path build-infra --force
```

### 10. Run formatter across codebase

`--workspace` processes all `.rs` files in the workspace (automatically skipping
`validation_tests/test_data/` via the default `--skip-folder-matches test_data` filter,
and skipping `build-infra/src/cargo_rustdoc_fmt/` via `// rustdoc-fmt: skip`).

- [x] Format workspace fences and verify idempotency (66 files modified):

```bash
cargo rustdoc-fmt --workspace --fences-only
cargo rustdoc-fmt --workspace --fences-only --check   # verify idempotency
```

### 11. Final verification

- [x] Run `./check.fish --full`:

```bash
./check.fish --full
```

### 12. Audit and verify largest rustdoc and code-fence target files

#### Workspace Rustdoc & Code-Fence Audit

`cargo rustdoc-fmt --workspace` discovers **897 `.rs` files** across all 5 workspace
crates (`tui`, `cmdr`, `build-infra`, `analytics_schema`, `rust-analyzer-mcp-server`),
excluding `validation_tests/test_data/`. After excluding the 13
`build-infra/src/cargo_rustdoc_fmt/` source files that carry `// rustdoc-fmt: skip`, there
are **884 active `.rs` files**:

| Category                                                         | File Count | Total Rustdoc Lines | Total Fence Delimiter Lines |
| :--------------------------------------------------------------- | ---------: | ------------------: | --------------------------: |
| Active `.rs` files in workspace (excluding `rustdoc-fmt: skip`)  |        884 |              64,258 |                       1,680 |
| Active `.rs` files containing rustdoc (`///` or `//!`)           |        782 |              64,258 |                       1,680 |
| **Target `.rs` files with rustdoc fenced code blocks (` ``` `)** |    **346** |          **47,192** |      **1,680 (840 blocks)** |
| `lib.rs` and `mod.rs` files with rustdoc                         |         95 |              14,108 |                         506 |
| **`lib.rs` and `mod.rs` files with rustdoc fenced code blocks**  |     **49** |          **13,115** |        **506 (253 blocks)** |
| Files modified by `cargo rustdoc-fmt --workspace --fences-only`  |         66 |              14,682 |            412 (206 blocks) |

#### All 49 `lib.rs` and `mod.rs` Files with Fenced Code Blocks (Sorted by Rustdoc Line Count)

|   # | File                                                                                                             | Rustdoc Lines | Fence Lines (` ``` `) | Modified by `--fences-only` |
| --: | :--------------------------------------------------------------------------------------------------------------- | ------------: | --------------------: | :-------------------------- |
|   1 | `tui/src/lib.rs`                                                                                                 |         3,564 |                   108 | Yes                         |
|   2 | `tui/src/core/coordinates/bounds_check/mod.rs`                                                                   |         1,264 |                    72 | Yes                         |
|   3 | `tui/src/core/resilient_reactor_thread/mod.rs`                                                                   |         1,084 |                    22 | No                          |
|   4 | `tui/src/readline_async/mod.rs`                                                                                  |           537 |                    14 | No                          |
|   5 | `tui/src/core/pty/mod.rs`                                                                                        |           484 |                     8 | No                          |
|   6 | `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/mod.rs`                                       |           483 |                    10 | No                          |
|   7 | `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_decoder/keyboard/mod.rs`                                   |           441 |                    12 | No                          |
|   8 | `rust-analyzer-mcp-server/src/lib.rs`                                                                            |           414 |                    14 | No                          |
|   9 | `tui/src/core/ansi/vt_100_terminal_input_parser/mod.rs`                                                          |           399 |                    10 | No                          |
|  10 | `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/mod.rs`                          |           343 |                    20 | No                          |
|  11 | `tui/src/tui/terminal_lib_backends/ofs_buf/mod.rs`                                                               |           316 |                    14 | Yes                         |
|  12 | `tui/src/tui/editor/zero_copy_gap_buffer/mod.rs`                                                                 |           273 |                     4 | Yes                         |
|  13 | `tui/src/core/graphemes/mod.rs`                                                                                  |           270 |                     6 | No                          |
|  14 | `build-infra/src/lib.rs`                                                                                         |           257 |                    34 | Yes                         |
|  15 | `tui/src/core/ansi/terminal_raw_mode/mod.rs`                                                                     |           247 |                    16 | No                          |
|  16 | `tui/src/tui/terminal_lib_backends/direct_to_ansi/mod.rs`                                                        |           241 |                     4 | No                          |
|  17 | `tui/src/core/ansi/mod.rs`                                                                                       |           239 |                    14 | No                          |
|  18 | `tui/src/tui/md_parser/mod.rs`                                                                                   |           226 |                     6 | No                          |
|  19 | `tui/src/core/coordinates/canvas/mod.rs`                                                                         |           226 |                     4 | No                          |
|  20 | `tui/src/core/coordinates/mod.rs`                                                                                |           223 |                     8 | Yes                         |
|  21 | `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/conformance_data/mod.rs`         |           212 |                    18 | No                          |
|  22 | `tui/src/tui/terminal_lib_backends/mod.rs`                                                                       |           184 |                     4 | No                          |
|  23 | `cmdr/src/lib.rs`                                                                                                |           175 |                     4 | No                          |
|  24 | `tui/src/core/ansi/vt_100_pty_output_parser/ops/mod.rs`                                                          |           168 |                     4 | No                          |
|  25 | `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_framer/mod.rs`                                             |           153 |                     2 | No                          |
|  26 | `tui/src/core/pty/pty_mux/mod.rs`                                                                                |           150 |                     2 | No                          |
|  27 | `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/mod.rs`                                             |           132 |                     6 | No                          |
|  28 | `analytics_schema/src/lib.rs`                                                                                    |           122 |                     2 | Yes                         |
|  29 | `tui/src/core/ansi/constants/mod.rs`                                                                             |           114 |                     6 | No                          |
|  30 | `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/mod.rs`                          |            97 |                     4 | No                          |
|  31 | `tui/src/core/test_fixtures/mod.rs`                                                                              |            89 |                     6 | No                          |
|  32 | `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mod.rs`                                                  |            87 |                     2 | No                          |
|  33 | `tui/src/tui/terminal_lib_backends/render_op/mod.rs`                                                             |            85 |                     2 | No                          |
|  34 | `tui/src/core/pty/pty_session/mod.rs`                                                                            |            81 |                     4 | No                          |
|  35 | `tui/src/core/coordinates/vt_100_ansi_coords/mod.rs`                                                             |            64 |                     2 | Yes                         |
|  36 | `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/test_sequence_generators/mod.rs` |            58 |                     2 | No                          |
|  37 | `tui/src/core/ansi/vt_100_terminal_input_parser/validation_tests/mod.rs`                                         |            57 |                     4 | No                          |
|  38 | `tui/src/core/ansi/terminal_raw_mode/validation_tests/mod.rs`                                                    |            51 |                     4 | No                          |
|  39 | `tui/src/readline_async/choose_impl/mod.rs`                                                                      |            51 |                     2 | No                          |
|  40 | `tui/src/core/ansi/terminal_raw_mode/raw_mode_integration_tests/mod.rs`                                          |            50 |                     4 | No                          |
|  41 | `tui/src/core/ansi/vt_100_pty_output_parser/mod.rs`                                                              |            50 |                     2 | No                          |
|  42 | `tui/src/tui/terminal_lib_backends/crossterm_backend/mod.rs`                                                     |            49 |                     2 | No                          |
|  43 | `tui/src/core/graphemes/word_boundaries/mod.rs`                                                                  |            47 |                     4 | Yes                         |
|  44 | `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/tests/mod.rs`                    |            42 |                     2 | No                          |
|  45 | `tui/src/core/terminal_io/mod.rs`                                                                                |            40 |                     2 | No                          |
|  46 | `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/mod.rs`                             |            38 |                     2 | No                          |
|  47 | `tui/src/tui/md_parser/conformance_test_data/mod.rs`                                                             |            37 |                     2 | No                          |
|  48 | `tui/src/core/coordinates/byte/mod.rs`                                                                           |            35 |                     2 | Yes                         |
|  49 | `tui/src/core/ansi/vt_100_terminal_input_parser/unit_tests/mod.rs`                                               |            18 |                     2 | No                          |

#### Top 35 Non-`mod.rs`/`lib.rs` Files with Largest Rustdoc Sections & Code Fences

|   # | File                                                                                    | Rustdoc Lines | Fence Lines (` ``` `) | Modified by `--fences-only` |
| --: | :-------------------------------------------------------------------------------------- | ------------: | --------------------: | :-------------------------- |
|   1 | `tui/src/core/ansi/constants/input_sequences.rs`                                        |           939 |                    12 | No                          |
|   2 | `tui/src/core/pty/pty_engine/pty_pair.rs`                                               |           889 |                    22 | No                          |
|   3 | `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/input_device_public_api.rs`     |           754 |                    20 | No                          |
|   4 | `tui/src/core/resilient_reactor_thread/rrt.rs`                                          |           683 |                    14 | No                          |
|   5 | `tui/src/core/ansi/vt_100_pty_output_parser/performer.rs`                               |           613 |                    26 | No                          |
|   6 | `tui/src/core/common/scoped_mutex/scoped_mutex_public_api.rs`                           |           525 |                    20 | Yes                         |
|   7 | `tui/src/core/coordinates/bounds_check/length_ops.rs`                                   |           456 |                    26 | Yes                         |
|   8 | `tui/src/readline_async/readline_async_impl/readline_struct.rs`                         |           426 |                     2 | No                          |
|   9 | `tui/src/core/ansi/vt_100_pty_output_parser/protocols/vte_params_ext.rs`                |           413 |                     6 | Yes                         |
|  10 | `tui/src/tui/editor/editor_buffer/buffer_struct.rs`                                     |           403 |                    12 | No                          |
|  11 | `tui/src/core/resilient_reactor_thread/rrt_worker.rs`                                   |           386 |                     4 | No                          |
|  12 | `tui/src/core/common/flat_2d_array/core.rs`                                             |           378 |                     4 | Yes                         |
|  13 | `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_decoder/mouse.rs`                 |           378 |                     4 | No                          |
|  14 | `tui/src/core/ansi/generator/ansi_input_test_fixtures.rs`                               |           356 |                     4 | No                          |
|  15 | `tui/src/core/test_fixtures/pty_test_fixtures/pty_test_child/pty_test_child_impl.rs`    |           351 |                     4 | No                          |
|  16 | `tui/src/core/coordinates/bounds_check/range/range_convert_ext.rs`                      |           329 |                    22 | Yes                         |
|  17 | `tui/src/core/resilient_reactor_thread/rrt_monitor.rs`                                  |           320 |                     4 | No                          |
|  18 | `tui/src/core/color_wheel/color_wheel_impl.rs`                                          |           312 |                     2 | No                          |
|  19 | `tui/src/core/ansi/detect_color/detect_color_support.rs`                                |           308 |                     4 | Yes                         |
|  20 | `tui/src/core/graphemes/gc_string/owned/gc_string_owned.rs`                             |           305 |                    32 | No                          |
|  21 | `tui/src/core/coordinates/bounds_check/range/range_ext.rs`                              |           299 |                    12 | No                          |
|  22 | `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_decoder/utf8.rs`                  |           298 |                     8 | No                          |
|  23 | `tui/src/core/pty/pty_session/builder.rs`                                               |           294 |                     8 | No                          |
|  24 | `tui/src/tui/editor/editor_engine/content_mut.rs`                                       |           290 |                    22 | No                          |
|  25 | `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_line_metadata.rs`                         |           286 |                    14 | Yes                         |
|  26 | `tui/src/core/coordinates/canvas/canvas_projection_ext.rs`                              |           285 |                    10 | No                          |
|  27 | `tui/src/core/coordinates/bounds_check/range/range_bounds_check.rs`                     |           283 |                     8 | No                          |
|  28 | `tui/src/core/ansi/vt_100_pty_output_parser/protocols/csi_codes/sgr_color_sequences.rs` |           282 |                    22 | No                          |
|  29 | `tui/src/core/graphemes/gc_string/owned/gc_string_owned_non_editor_impl.rs`             |           276 |                    28 | No                          |
|  30 | `tui/src/core/test_fixtures/pty_test_fixtures/generate_pty_test.rs`                     |           274 |                     4 | No                          |
|  31 | `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_scroll_ops.rs` |           264 |                     8 | No                          |
|  32 | `tui/src/core/ansi/vt_100_pty_output_parser/ansi_parser_public_api.rs`                  |           259 |                     8 | Yes                         |
|  33 | `tui/src/core/coordinates/bounds_check/range/range_construct_ext.rs`                    |           253 |                     8 | Yes                         |
|  34 | `tui/src/core/ansi/constants/generic.rs`                                                |           253 |                     4 | Yes                         |
|  35 | `tui/src/core/coordinates/bounds_check/index_ops.rs`                                    |           253 |                    10 | Yes                         |

#### Current Full `cargo rustdoc-fmt` Status Across All 884 Active Files

When running the full `cargo rustdoc-fmt` pipeline (all 4 passes enabled: `format_tables`,
`convert_links`, `link_terms`, and `normalize_fences`) via `--check`:

1. **`--tables-only --workspace --check`**: `0 modified, 0 errors` (100% clean).
2. **`--links-only --workspace --check`**: `0 modified, 0 errors` (100% clean).
3. **`--fences-only --workspace --check`**: `0 modified, 0 errors` (100% clean after the
   66-file pass).
4. **Full `cargo rustdoc-fmt --workspace --check`** (including `--terms-only`):
    - **All 49 `lib.rs` and `mod.rs` files with code fences** and **all top rustdoc files
      (`> 50` rustdoc lines)** produce **0 modifications** (already 100% formatted and
      idempotent).
    - **53 smaller files** (`<= 50` rustdoc lines each; 14 of which contain code fences)
      across `tui/`, `cmdr/`, and `rust-analyzer-mcp-server/` were not formatted in the
      earlier seed-term commit (`dd849d974`) because that run used default git-changed
      discovery rather than `--workspace`.

#### Verification Steps

- [x] Run full `cargo rustdoc-fmt --workspace --verbose` across all 884 active `.rs` files
      (covering all 346 code-fence files, all 95 `lib.rs`/`mod.rs` files, and the 53
      remaining unformatted term files):

```bash
cargo rustdoc-fmt --workspace --verbose
```

- [x] Verify idempotency across the entire workspace (confirm a second full pass produces
      `0 modified, 0 errors`):

```bash
cargo rustdoc-fmt --workspace --check --verbose
```

- [x] Audit `git diff` across all newly modified files to verify:
    - Code fence bodies and opening tags (` ``` `, ` ```ignore `, ` ```no_run `,
      ` ```text `) are 100% intact.
    - No terms inside code fences or inline code spans were linkified.
    - All reference definitions are properly sorted at the bottom of each doc block (and
      fixed pre-existing split reference link in
      `tui/src/core/terminal_io/backend_compat_tests/pty_non_blocking_stdout_no_panic_test.rs`).
- [x] Run full documentation, doctest, and test verification (`./check.fish --quick-doc`,
      `./check.fish --doc`, and `./check.fish --test`) to verify all doctests in the 346
      code-fence files compile and pass and all intra-doc links resolve without warnings.
- [x] **Mandatory manual review:** Verify every file modified in this task for correct
      implementation and ensure no regressions.
    - [x] `build-infra/AGENTS.md`
    - [x] `build-infra/src/cargo_rustdoc_fmt/fence_normalizer.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/mod.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/types.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/cst.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/processor.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/validation_tests/complete_file_tests.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output/sample_input_event.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output/sample_code_fence_comma.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/input/sample_fence_normalizer.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output/sample_fence_normalizer.rs`
    - [x] 66 workspace `.rs` files modified by `--fences-only`
          (`analytics_schema/src/lib.rs`, `build-infra/src/lib.rs`, and 64 files in
          `tui/src/`)
    - [x] 53 workspace `.rs` files modified by full `cargo rustdoc-fmt --workspace`
          (`cmdr/`, `rust-analyzer-mcp-server/`, `tui/examples/`, and `tui/src/`)

## Files Modified (summary)

| File                                                                                                                    | Change                                                               |
| :---------------------------------------------------------------------------------------------------------------------- | :------------------------------------------------------------------- |
| `build-infra/src/cargo_rustdoc_fmt/fence_normalizer.rs`                                                                 | **New**: CST + text transformation module                            |
| `build-infra/src/cargo_rustdoc_fmt/mod.rs`                                                                              | Add module + re-export                                               |
| `build-infra/AGENTS.md`                                                                                                 | Add `fence_normalizer.rs` to project structure tree                  |
| `build-infra/src/cargo_rustdoc_fmt/types.rs`                                                                            | Add `status` to `CodeFence`, `normalize_fences` to `FormatOptions`   |
| `build-infra/src/cargo_rustdoc_fmt/cst.rs`                                                                              | Initialize `status: FormatStatus::Unchanged` on `DocNode::CodeFence` |
| `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs`                                                                          | Add `--fences-only` flag                                             |
| `build-infra/src/cargo_rustdoc_fmt/processor.rs`                                                                        | Wire `normalize_code_fence_node` into CST pipeline                   |
| `build-infra/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output/sample_input_event.rs`      | Update golden output                                                 |
| `build-infra/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output/sample_code_fence_comma.rs` | Update golden output                                                 |
| `build-infra/src/cargo_rustdoc_fmt/validation_tests/complete_file_tests.rs`                                             | Update assertions, add new test                                      |
| `build-infra/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/input/sample_fence_normalizer.rs`           | **New**: test fixture                                                |
| `build-infra/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output/sample_fence_normalizer.rs` | **New**: test fixture                                                |
| 66 source files across `analytics_schema/src/`, `build-infra/src/`, and `tui/src/`                                      | Mechanical: ` ```rust ` -> ` ``` `                                   |
| 53 source files across `cmdr/`, `rust-analyzer-mcp-server/`, and `tui/`                                                 | Full `cargo rustdoc-fmt --workspace` pass                            |
