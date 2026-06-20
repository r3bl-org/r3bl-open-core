# Task: PR 462 Integration (LF scroll-up test fix)

## Overview

PR 462 (by Cecile Tonglet) updates `test_handle_line_feed_at_bottom` to actually verify
that content scrolls up when `handle_line_feed()` is called at the bottom row, instead of
just asserting that the cursor position stays clamped. This correctly tests the VT100
scroll-on-LF behavior.

## PR Handling & Attribution

We are implementing our own fixes based on Cecile's original intent and closing her PR
without merging it directly due to underlying architectural changes. We must absolutely
add a `Co-authored-by: Cecile Tonglet <cecile.tonglet@cecton.com>` trailer to all of the
commits we make for this task to ensure she gets proper attribution for the feature!

## Implementation plan

### Phase 1: Test Fix

- [x] Update test assertions in `test_handle_line_feed_at_bottom` to check for scrolled
      content.
- [x] Run `./check.fish --test` to confirm behavior.
- [x] Update `task/prepare-v0.8.0-meta-task.md` to check off this PR.
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_control_ops.rs`

### Phase 2: Documentation / Terminology Fix

- [x] Replace occurrences of `VT100` with `[`VT-100`]` in
      `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_dsr_ops.rs`.
- [x] Add the link reference definition for `[`VT-100`]` at the end of the module
      documentation in `vt_100_impl_dsr_ops.rs`.
- [x] Add the link reference definition for `[`vt_100_pty_output_parser::ops::dsr_ops`]`
      in `vt_100_impl_dsr_ops.rs`.
- [x] Fix formatting of `ESC 0n` response documentation in `vt_100_impl_dsr_ops.rs`.
- [x] Run `./check.fish --check` to verify code compiles.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
  - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_dsr_ops.rs`
  - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_sgr_ops.rs`

### Phase 3: Shorten `nz` Import Path using Barrel Exports

- [x] Flatly re-export `nz` under `vt_100_pty_output_conformance_tests` module.
- [x] Shorten all references to `nz` across the workspace.
- [x] Run `./check.fish --check` to verify code compiles.
- [x] Run `./check.fish --quick-doc` to verify documentation builds without warnings.
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/mod.rs`
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_scroll_ops.rs`
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_margin_ops.rs`
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_ansi_scroll_helper.rs`
  - [x] `tui/src/core/ansi/generator/dsr_sequence.rs`
  - [x] `tui/src/core/coordinates/vt_100_ansi_coords/term_col.rs`
  - [x] `tui/src/core/coordinates/vt_100_ansi_coords/term_row.rs`
  - [x] `tui/src/tui/terminal_lib_backends/direct_to_ansi/output/tests.rs`
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ansi_parser_public_api.rs`
