# Task: PR 462 Integration (LF scroll-up test fix)

## Overview

PR 462 (by Cecile Tonglet) updates `test_handle_line_feed_at_bottom` to actually verify
that content scrolls up when `handle_line_feed()` is called at the bottom row, instead of
just asserting that the cursor position stays clamped. This correctly tests the VT100
scroll-on-LF behavior.

## Implementation plan

### Phase 1: Test Fix

- [ ] Update test assertions in `test_handle_line_feed_at_bottom` to check for scrolled
      content.
- [ ] Run `./check.fish --test` to confirm behavior.
- [ ] Update `task/prepare-v0.8.0-meta-task.md` to check off this PR.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
  - [ ] `tui/src/terminal_window/vt_100_impl_control_ops.rs` (or wherever the test
        resides)
