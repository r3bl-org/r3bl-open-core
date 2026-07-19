# Task: Migrate Selection Architecture to `SelectionLine` & `Selection` (Anchor Model)

## Status

- [x] Phase 1: File Renames & Module Coordinator Updates
- [x] Phase 2: `SelectionLine` Implementation (`selection_per_line.rs`)
- [x] Phase 3: `Selection` Implementation (`selection_container.rs`)
- [x] Phase 4: Deterministic Selection Engine (`selection_state_machine.rs`)
- [x] Phase 5: Crate Integration & Verification
- [x] Phase 6: Selection API & Documentation Cleanup
- [x] Phase 7: Refactor `selection_support.rs` to `selection_state_machine.rs` with `AnchorState`
      & `SelectionRange` Enums
- [x] Phase 8: Modernize & Rename Selection Architecture to `Selection`, `SelectionLine`,
      and `selection_state_machine.rs`
- [x] Phase 9: Mandatory Manual Review

---

## Overview

Refactor the editor selection architecture:

1. **`SelectionLine`**
   ([`selection_per_line.rs`](file:///home/nazmul/github/roc/tui/src/tui/editor/editor_buffer/selection/selection_per_line.rs)):
    - Stores `row: CRow`, `start_col: CCol`, `end_col: CCol` directly (**0 `CCaret`
      fields, 0 `usize::MAX` hacks**).
    - Replaces `LineSelection` & `SelectionRange`.
2. **`Selection`**
   ([`selection_container.rs`](file:///home/nazmul/github/roc/tui/src/tui/editor/editor_buffer/selection/selection_container.rs)):
    - Stores `pub anchor_caret: Option<CCaret>` and `list: InlineVec<SelectionLine>`.
    - Replaces `MultiLineSelection` & `SelectionList`.
    - Removes `maybe_previous_direction` and stateful direction history logic.
3. **Deterministic Selection Engine**
   ([`selection_state_machine.rs`](file:///home/nazmul/github/roc/tui/src/tui/editor/editor_buffer/selection/selection_state_machine.rs)):
    - Derives single and multi-line selections deterministically from
      `(anchor_caret, active_caret)` using `AnchorState` and `SelectionRange` enums.

---

## Detailed Action Items

### Phase 1: File Renames (`git mv`) & Module Updates

- [x] Run
      `git mv tui/src/tui/editor/editor_buffer/selection/selection_range.rs tui/src/tui/editor/editor_buffer/selection/selection_per_line.rs`
- [x] Run
      `git mv tui/src/tui/editor/editor_buffer/selection/selection_list.rs tui/src/tui/editor/editor_buffer/selection/selection_container.rs`
- [x] Update `tui/src/tui/editor/editor_buffer/selection/mod.rs` module declarations and
      re-exports.
- [x] Verify `./check.fish --check` compiles.

### Phase 2: `SelectionLine` Implementation (`selection_per_line.rs`)

- [x] Implement `SelectionLine` struct with `row: CRow`, `start_col: CCol`,
      `end_col: CCol`.
- [x] Implement `From<(CRow, CCol, CCol)>`, `From<(CRow, Range<CCol>)>`, and
      `From<(CCaret, CCaret)>`.
- [x] Implement `clip_to_range_str` and `clip_left_to_vp_origin`.
- [x] Update all doc comments and ASCII diagrams preserving explanations.
- [x] Update unit tests in `selection_per_line.rs`.

### Phase 3: `Selection` Implementation (`selection_container.rs`)

- [x] Implement `Selection` struct with `pub anchor_caret: Option<CCaret>` and
      `list: InlineVec<SelectionLine>`.
- [x] Remove `maybe_previous_direction` and direction-change methods.
- [x] Add comprehensive rustdocs outlining the `anchor_caret` + `active_caret`
      deterministic selection algorithm (as used in VS Code and JetBrains IDEs) with ASCII
      diagrams.
- [x] Update all doc comments and ASCII diagrams preserving explanations.
- [x] Update unit tests in `selection_container.rs`.

### Phase 4: Deterministic Selection Engine (`selection_state_machine.rs`)

- [x] Update selection handlers to compute selections deterministically from
      `(anchor_caret, active_caret)`.
- [x] Add module-level and function-level rustdocs detailing the
      `(anchor_caret, active_caret)` algorithm.
- [x] Remove stateful direction-reversal logic.
- [x] Update unit tests in `selection_state_machine.rs`.

### Phase 5: Crate Integration & Verification

- [x] Update `EditorBuffer` (`buffer_struct.rs`), rendering (`engine_public_api.rs`),
      event handling (`content_mut.rs`), and test fixtures (`test_fixtures_editor.rs`).
- [x] Run `./check.fish --all` to verify typecheck, build, clippy, unit tests, doctests,
      doc build, and cross-platform compilation.
- [x] Run `check code quality` skill to verify code quality and documentation.

### Phase 6: Selection API & Documentation Cleanup

- [x] Consolidate entry point in `selection_state_machine.rs`: export
      `update_selection_from_anchor_and_active` and remove redundant `handle_selection_*`
      1-line wrappers.
- [x] Update unit tests in `selection_state_machine.rs` to call
      `update_selection_from_anchor_and_active` directly.
- [x] Simplify `select_mode.rs` by removing redundant row comparison branching and calling
      `update_selection_from_anchor_and_active` directly.
- [x] Update call sites in `caret_mut.rs` to call
      `update_selection_from_anchor_and_active`.
- [x] Fix rustdoc intra-doc link visibility in `selection/mod.rs`
      (`#[cfg(any(test, doc))] pub mod selection_state_machine;`).
- [x] Run `./check.fish --all` to verify build, tests, and documentation.

### Phase 7: Refactor `selection_support.rs` to `selection_state_machine.rs` with `AnchorState` & `SelectionRange` Enums

- [x] **File Renaming & Module Updates**:
    - Run
      `git mv tui/src/tui/editor/editor_buffer/selection/selection_support.rs tui/src/tui/editor/editor_buffer/selection/selection_state_machine.rs`.
    - Update `tui/src/tui/editor/editor_buffer/selection/mod.rs` module declaration
      (`selection_state_machine`) and re-exports.
    - Update rustdoc links referencing `selection_support` in `selection_container.rs`.
- [x] **Implement `AnchorState` Enum & State Machine**:
    - Define `AnchorState` enum representing anchor resolution strategies:
        - `AlreadySet(CCaret)`: Anchor is stored on `selection.anchor_caret`.
        - `FromNewSelection`: Anchor is `None` & `selection` is empty (new selection
          starting).
        - `FromExistingSelection { first: SelectionLine, last: SelectionLine }`: Anchor is
          `None` & `selection` is non-empty (infer anchor from selection boundaries).
    - Implement `AnchorState::from_buffer(buffer: &EditorBuffer) -> Self` state inspector.
    - Implement
      `resolve_and_update(&self, buffer: &mut EditorBuffer, prev: CCaret) -> CCaret`
      method to mutate buffer selection list when needed and return resolved anchor caret.
- [x] **Implement `SelectionRange` Enum & Range Categorization**:
    - Define `SelectionRange` enum representing categorized selection span:
        - `Empty`: `anchor == active`.
        - `SingleLine { row: CRow, start_col: CCol, end_col: CCol }`: Single line
          selection (`start_row == end_row`).
        - `MultiLine { start_caret: CCaret, end_caret: CCaret }`: Multi-line selection
          (`start_row < end_row`).
    - Implement `SelectionRange::from_carets(anchor: CCaret, active: CCaret) -> Self` for
      caret ordering (`anchor <= active` vs `active < anchor`).
    - Implement
      `compute_line_selections(&self, buffer: &EditorBuffer) -> InlineVec<SelectionLine>`
      method to compute single or multi-line `SelectionLine` entries based on line display
      widths.
- [x] **Refactor & Rename Entry Point Function**:
    - Rename `update_selection_from_anchor_and_active` to
      `update_selection_from_anchor_and_active_carets`.
    - Refactor body to cleanly compose `AnchorState` and `SelectionRange`:

        ```rust
        pub fn update_selection_from_anchor_and_active_carets(
            buffer: &mut EditorBuffer,
            prev: CCaret,
            curr: CCaret,
        ) {
            let anchor = AnchorState::from_buffer(buffer).resolve_and_update(buffer, prev);
            let active = curr;

            let range = SelectionRange::from_carets(anchor, active);
            let new_selections = range.compute_line_selections(buffer);

            buffer.mutate_selection(|sel_list| {
                sel_list.list = new_selections;
            });
        }
        ```

- [x] **Update External Callsites & Unit Tests**:
    - Update `tui/src/tui/editor/editor_engine/select_mode.rs` imports and function calls
      to `update_selection_from_anchor_and_active_carets`.
    - Update unit tests in `selection_state_machine.rs` to call
      `update_selection_from_anchor_and_active_carets`.
    - Add comprehensive unit tests for `AnchorState` methods (`from_buffer`,
      `resolve_and_update`) and `SelectionRange` methods (`from_carets`,
      `compute_line_selections`).
- [x] **Verification**:
    - Run `./check.fish --all` to verify typecheck, build, clippy, unit tests, doctests,
      doc build, and cross-platform compilation.

### Phase 8: Modernize & Rename Selection Architecture to `Selection`, `SelectionLine`, and `selection_state_machine.rs`

- [x] **Rename Types & Method Signatures**:
    - Rename `MultiLineSelection` $\rightarrow$ `Selection`.
    - Rename `LineSelection` $\rightarrow$ `SelectionLine`.
    - Rename `get_sel_list()` / `get_selection_list()` $\rightarrow$ `get_selection()`.
    - Rename field `EditorContent::sel_list` $\rightarrow$ `selection`.
- [x] **File Renames**:
    - Rename `multiline_selection.rs` $\rightarrow$ `selection_container.rs`.
    - Rename `line_selection.rs` $\rightarrow$ `selection_per_line.rs`.
    - Rename `selection_range.rs` $\rightarrow$ `selection_state_machine.rs`.
- [x] **Update Re-exports & Callsites**:
    - Update `selection/mod.rs` module attachments and re-exports.
    - Update all callsites across `tui` and `cmdr`.
- [x] **Verification**:
    - Run `./check.fish --check`, `./check.fish --clippy`, `./check.fish --fmt`,
      `./check.fish --quick-doc`, and `./check.fish --test`.

### Phase 9: Mandatory Manual Review Checklist

- [x] `task/migrate-selection-to-anchor-and-line-selection.md`
- [x] `tui/src/tui/editor/editor_buffer/selection/selection_line.rs`
- [x] `tui/src/tui/editor/editor_buffer/selection/selection_container.rs`
- [x] `tui/src/tui/editor/editor_buffer/selection/selection_state_machine.rs`
- [x] `tui/src/tui/editor/editor_buffer/selection/mod.rs`
- [x] `tui/src/tui/editor/editor_buffer/buffer_struct.rs`
- [x] `tui/src/tui/editor/editor_buffer/sizing.rs`
- [x] `tui/src/tui/editor/editor_buffer/clipboard/clipboard_support.rs`
- [x] `tui/src/tui/editor/editor_engine/validate_buffer_mut.rs`
- [x] `tui/src/tui/editor/editor_engine/content_mut.rs`
- [x] `tui/src/tui/editor/editor_engine/engine_internal_api.rs`
- [x] `tui/src/tui/editor/editor_engine/engine_public_api.rs`
- [x] `tui/src/tui/editor/editor_component/editor_event.rs`
- [x] `tui/src/core/stack_alloc_types/sizes.rs`
