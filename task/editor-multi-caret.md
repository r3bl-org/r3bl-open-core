# Task: Editor Multiple Carets

## Overview

Extends `r3bl_tui` (`EditorBuffer`, `EditorEngine`, `EditorComponent`) and `edi` from a
single caret (`CCaret`) and single `SelectionContainer` to VS Code-style multiple carets
and selections.

`r3bl_tui` already paints carets as virtual reverse-video cells in `OfsBuf` rather than
relying on a single hardware terminal cursor, and `ZeroCopyGapBuffer` already exposes
`rebuild_line_segments_batch` for multi-line edits. Replacing the standalone `c_caret` and
`selection` fields on `EditorContent` with a canonical, invariant-preserving `CaretSet`
unlocks simultaneous multi-line editing, <kbd>Ctrl</kbd>+<kbd>D</kbd> next-occurrence
selection, and multi-match editing from Find/Replace.

```text
┌───────────────────────────────────────────────────────────────────┐
│ EditorContent                                                     │
│ ├─ lines: ZeroCopyGapBuffer                                       │
│ ├─ viewport: Viewport (pans to keep CaretSet::primary() in view)  │
│ └─ carets: CaretSet                                               │
│     ├─ primary_index: CIndex                                      │
│     └─ entries: InlineVec<CaretEntry> (sorted by CRow, CCol)      │
│         ├─ [0] CaretEntry { c_caret, selection }                  │
│         ├─ [1] CaretEntry { c_caret, selection }  <-- primary     │
│         └─ [2] CaretEntry { c_caret, selection }                  │
└─────────────────────────────────┬─────────────────────────────────┘
                                  │
       Edits applied bottom-to-top (index 2 -> 1 -> 0) inside one
       EditorBufferMutWithDrop scope, then normalize_and_merge()
                                  │
                                  ▼
┌───────────────────────────────────────────────────────────────────┐
│ Drop of EditorBufferMutWithDrop                                   │
│ 1. Clamp every CaretEntry to grapheme boundaries                  │
│ 2. Deduplicate colliding carets & merge overlapping selections    │
│ 3. Pan Viewport to primary caret & invalidate caches              │
└───────────────────────────────────────────────────────────────────┘
```

## Implementation Plan

### Phase 1: CaretSet Domain Model

Introduce `CaretEntry` and `CaretSet` so illegal states (zero carets, unsorted carets,
duplicate carets at identical positions, or overlapping selections) are unrepresentable
after validation.

- [ ] Create `tui/src/tui/editor/editor_buffer/caret_set/` following module organization
      rules:
    - `tui/src/tui/editor/editor_buffer/caret_set/mod.rs`
    - `tui/src/tui/editor/editor_buffer/caret_set/caret_entry_struct.rs`
    - `tui/src/tui/editor/editor_buffer/caret_set/caret_set_struct.rs`
    - `tui/src/tui/editor/editor_buffer/caret_set/caret_merge.rs`
- [ ] Define `CaretEntry`, `CaretAddOutcome`, and `CaretSet`:

    ```rust
    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub struct CaretEntry {
        pub c_caret: CCaret,
        pub selection: SelectionContainer,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum CaretAddOutcome {
        Added,
        MergedExisting,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct CaretSet {
        entries: InlineVec<CaretEntry>,
        primary_index: CIndex,
    }
    ```

    - Implement `Default` for `CaretSet` with a single default `CaretEntry` at
      `primary_index = c_idx(0)` (`len()` is always `>= 1`).
    - Implement `primary(&self) -> &CaretEntry`,
      `primary_mut(&mut self) -> &mut CaretEntry`, `as_slice(&self) -> &[CaretEntry]`,
      `len(&self) -> usize`, `is_empty(&self) -> bool`,
      `add_caret(&mut self, caret: CCaret) -> CaretAddOutcome`,
      `collapse_to_primary(&mut self)`, and
      `normalize_and_merge(&mut self, lines: &ZeroCopyGapBuffer)`.

- [ ] Migrate `EditorContent` (`tui/src/tui/editor/editor_buffer/buffer_struct.rs`) and
      `EditorBufferMut` (`tui/src/tui/editor/editor_engine/validate_buffer_mut.rs`) from
      standalone `c_caret` + `selection` fields to `carets: CaretSet`:
    - Keep `get_c_caret()` and `get_selection_container()` on `EditorBuffer` delegating to
      `carets.primary()` so existing single-caret callers and tests remain intact.
    - Add `get_caret_set(&self) -> &CaretSet` on `EditorBuffer`.
    - Update `perform_validation_checks_after_mutation` in `validate_buffer_mut.rs` to
      validate grapheme bounds on every `CaretEntry` and invoke `normalize_and_merge`.
- [ ] Run `./check.fish --check` and unit tests for `CaretSet` sorting, overlap merging,
      and `EditorHistory` snapshot round-trips.
- [ ] **Mandatory manual review:** Verify `CaretSet` invariants and `EditorBuffer`
      migration.
    - [ ] `tui/src/tui/editor/editor_buffer/caret_set/mod.rs`
    - [ ] `tui/src/tui/editor/editor_buffer/caret_set/caret_entry_struct.rs`
    - [ ] `tui/src/tui/editor/editor_buffer/caret_set/caret_set_struct.rs`
    - [ ] `tui/src/tui/editor/editor_buffer/caret_set/caret_merge.rs`
    - [ ] `tui/src/tui/editor/editor_buffer/buffer_struct.rs`
    - [ ] `tui/src/tui/editor/editor_buffer/mod.rs`
    - [ ] `tui/src/tui/editor/editor_engine/validate_buffer_mut.rs`

### Phase 2: Multi-Caret Movement and Shortcuts

Add keyboard events and cursor/selection movement across all active carets.

- [ ] In `tui/src/tui/editor/editor_component/editor_event.rs`, define `MultiCaretAction`
      and add `EditorEvent::MultiCaret(MultiCaretAction)`:

    ```rust
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum MultiCaretAction {
        AddCaretAbove,
        AddCaretBelow,
        AddNextOccurrence,
        SelectAllOccurrences,
        AddOrToggleCaretAt(CPos),
        CollapseToPrimary,
    }
    ```

    - Bind <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>Up</kbd> to `AddCaretAbove`.
    - Bind <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>Down</kbd> to `AddCaretBelow`.
    - Bind <kbd>Ctrl</kbd>+<kbd>D</kbd> to `AddNextOccurrence` (if primary selection is
      empty, select word under primary caret; otherwise locate next exact match in
      `ZeroCopyGapBuffer` and add a `CaretEntry` selecting it).
    - Bind <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>L</kbd> to `SelectAllOccurrences`.
    - Update <kbd>Esc</kbd> handling so when `carets.len() > 1` or any selection is
      non-empty, it collapses secondary carets to primary and clears selections.

- [ ] Update `tui/src/tui/editor/editor_engine/caret_mut.rs` and `select_mode.rs` so
      directional movement (<kbd>Left</kbd>, <kbd>Right</kbd>, <kbd>Up</kbd>,
      <kbd>Down</kbd>, <kbd>Home</kbd>, <kbd>End</kbd>, <kbd>PageUp</kbd>,
      <kbd>PageDown</kbd>) and `SelectionAction` apply to every `CaretEntry` in
      `CaretSet`, panning `Viewport` to `carets.primary()` and merging any carets that
      converge onto the same `(CRow, CCol)`.
- [ ] Run `./check.fish --check` and unit tests for multi-caret movement, convergence
      deduplication, <kbd>Ctrl</kbd>+<kbd>D</kbd>, and <kbd>Esc</kbd> collapse.
- [ ] **Mandatory manual review:** Verify multi-caret navigation and shortcut bindings.
    - [ ] `tui/src/tui/editor/editor_component/editor_event.rs`
    - [ ] `tui/src/tui/editor/editor_engine/caret_mut.rs`
    - [ ] `tui/src/tui/editor/editor_engine/select_mode.rs`

### Phase 3: Batch Multi-Caret Edits

Execute insertions, deletions, newlines, and clipboard operations deterministically across
multiple carets within a single undoable transaction.

- [ ] Refactor mutation routines in `tui/src/tui/editor/editor_engine/content_mut.rs`:
    - Iterate `CaretSet` in reverse `(CRow, CCol)` order inside a single
      `EditorBufferMutWithDrop` scope for character insertion, string insertion, newline
      splitting, `Delete`, `Backspace`, and `delete_selected`.
    - When multiple carets sit on the same `CRow`, adjust column offsets (`CCol`) and
      `SelectionLine` ranges of higher-column carets on that row as earlier-column or
      reverse-order edits complete.
    - Batch segment rebuilds on touched rows via
      `ZeroCopyGapBuffer::rebuild_line_segments_batch`.
- [ ] Update clipboard handling in
      `tui/src/tui/editor/editor_buffer/clipboard/clipboard_service_impl.rs` and
      `editor_event.rs`:
    - **Copy / Cut**: Concatenate selected text from each `CaretEntry` in top-to-bottom
      order joined by `\n`.
    - **Paste**: If clipboard contains `N` lines matching `carets.len() == N` (with
      `N > 1`), insert 1 corresponding line per caret; otherwise insert the full clipboard
      text at every caret.
- [ ] Verify `EditorHistory` records each multi-caret edit as a single atomic undo/redo
      step restoring all carets and selections.
- [ ] Run `./check.fish --check` and unit tests covering same-row multi-caret edits,
      multi-line newline insertion, multi-caret copy/paste, and undo/redo.
- [ ] **Mandatory manual review:** Verify reverse-order edit math and clipboard behavior.
    - [ ] `tui/src/tui/editor/editor_engine/content_mut.rs`
    - [ ] `tui/src/tui/editor/editor_buffer/clipboard/clipboard_service_impl.rs`
    - [ ] `tui/src/tui/editor/editor_component/editor_event.rs`

### Phase 4: Rendering and edi Integration

Render all active carets and selections in the viewport and surface multi-caret status in
`edi`.

- [ ] Update `render_selection` and `render_caret` in
      `tui/src/tui/editor/editor_engine/engine_public_api.rs`:
    - Render selection spans across all `CaretEntry` items in `CaretSet`.
    - Paint all visible carets in `OfsBuf`, distinguishing the primary caret (reverse
      video) from secondary carets (distinct accent background/underline style) so the
      user always knows which caret drives scrolling and <kbd>Esc</kbd> collapse.
- [ ] Update `cmdr/src/edi/app_main.rs`:
    - Display an active caret badge (e.g., `Carets: N`) in `status_bar::render_status_bar`
      whenever `carets.len() > 1`.
    - Verify zero shortcut conflicts with `edi` app-level bindings
      (<kbd>Ctrl</kbd>+<kbd>S</kbd>, <kbd>Ctrl</kbd>+<kbd>K</kbd>,
      <kbd>Ctrl</kbd>+<kbd>Q</kbd>).
- [ ] Add integration tests in `tui/src/tui/editor/` and `cmdr/src/edi/`.
- [ ] Run `./check.fish --full` and `remove-crate-prefix` check across all modified files.
- [ ] **Mandatory manual review:** Verify multi-caret rendering, `edi` UX, and full
      checks.
    - [ ] `tui/src/tui/editor/editor_engine/engine_public_api.rs`
    - [ ] `cmdr/src/edi/app_main.rs`
    - [ ] `task/editor-multi-caret.md`
