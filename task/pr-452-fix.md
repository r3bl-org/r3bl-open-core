# Task: PR 452 Integration & Fixes

## Overview

We are integrating the core fixes from PR
[#452](https://github.com/r3bl-org/r3bl-open-core/pull/452) (`@cecton`) related to
preserving the parent origin when computing the next box insertion position in the layout
surface.

## Execution Workflow

We will process each of the action items iteratively using the following loop:

1. **Implementation:** Write the specific code changes for the current heading.
2. **Local Testing:** Run `./check.fish --check` and, where applicable, test
   functionality.
3. **Mandatory Manual Review:** You (the user) will manually review the specifically
   touched files before the heading is marked as checked `[x]`.

_(Once all headings are successfully implemented and checked off, we will proceed to final
verification and cleanup.)_

### Core Fixes from PR #452

#### [x] Fix `update_insertion_pos_for_next_box`

Fix the issue where directional-mask multiplication zeroed the preserved axis when the
parent container origin is non-zero.

- _Context:_ `update_insertion_pos_for_next_box` used a directional-mask multiplication
  (`new_pos * (width(0) + height(1))`) to discard the axis that shouldn't advance. When
  the parent container origin is non-zero, this zeroed the preserved axis instead of
  keeping it. The bug manifested as sibling boxes in a `Horizontal` container placed at a
  non-zero row all having their row snapped to 0 after the first sibling — breaking
  padding symmetry on the second (and subsequent) panes.
- _The Fix:_ Replace the multiplication with explicit `Pos` field selection. `row_index`
  and `col_index` are distinct newtypes, so the assignment is type-checked by the
  compiler. For vertical layout, keep the column; for horizontal layout, keep the row.
- _File Touched:_ `tui/src/tui/layout/surface.rs`

#### [x] Fix Outdated Rustdocs in `update_insertion_pos_for_next_box`

Update outdated rustdoc and inline comments, replacing `box_cursor_pos` with intra-doc
links to prevent future bit rot.

- _Context:_ The rustdoc and inline comments incorrectly refer to an outdated
  `box_cursor_pos` field. Furthermore, we want to adhere to the codebase standard of
  linkifying symbols to prevent staleness.
- _The Fix:_ Update comments to refer to `[`FlexBox::insertion_pos_for_next_box`]` using
  proper intra-doc linking.
- _File Touched:_ `tui/src/tui/layout/surface.rs`

#### [x] Add Regression Tests

Add regression tests to cover layout positioning with offset origins.

- _Context:_ We need regression tests to ensure that sibling boxes placed inside
  horizontal and vertical containers with non-zero origins preserve the proper offset (row
  or col respectively) from their parent.
- _The Fix:_ Add two regression tests: `test_surface_horizontal_non_zero_origin` and
  `test_surface_vertical_non_zero_origin`.
- _File Touched:_ `tui/src/tui/layout/test_surface_offset_origin.rs`

#### [x] Register Test Module

Ensure the new regression test module is included in the build.

- _Context:_ The new test file needs to be registered so that `cargo test` executes it.
- _The Fix:_ Add `mod test_surface_offset_origin;` to the module declarations.
- _File Touched:_ `tui/src/tui/layout/mod.rs`

### Final Verification & Cleanup

- [x] Verify full test suite coverage using `./check.fish --full`.
- [x] **Mandatory manual review:** Verify every file modified in this task for correct
      implementation and ensure no regressions.
  - [x] `tui/src/tui/layout/surface.rs`
  - [x] `tui/src/tui/layout/mod.rs`
  - [x] `task/prepare-v0.8.0-meta-task.md`
- [x] Update `task/prepare-v0.8.0-meta-task.md` to check off PR #452.
- [x] Run interactive rebase (`git rebase -i main`) on the PR branch (if applicable), or
      commit changes locally with the correct PR authorship (`@cecton`).
- [x] Merge the changes into `main`.
