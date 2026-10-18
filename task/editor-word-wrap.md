# Task: Editor Soft Word Wrap

## Overview

Adds VS Code-style soft word wrap (`WordWrapMode::Off` and `WordWrapMode::On`) to the
`r3bl_tui` editor engine and exposes both a runtime toggle (<kbd>Alt</kbd>+<kbd>Z</kbd>)
and a `--word-wrap` CLI option in `edi`.

Soft word wrap breaks long logical lines visually at viewport width boundaries without
inserting `\n` bytes into the underlying `ZeroCopyGapBuffer`. Logical document coordinates
(`CRow`, `CCol`, `CCaret`, `SelectionLine`) remain unchanged, while rendering and vertical
cursor movement map through wrapped visual line spans.

**Sequencing Note**: Execute this task after
`task/rasterize-editor-component-rendering.md` so wrapped visual spans rasterize directly
into `OfsBuf` rows without legacy string-space `clip()` allocations.

```text
┌──────────────────────────────────────────────────────────────────┐
│ ZeroCopyGapBuffer (Logical Document Coordinates: CRow, CCol)     │
│   CRow(0): "The quick brown fox jumps over the lazy dog"         │
└───────────────────────────────┬──────────────────────────────────┘
                                │
                                ▼
┌──────────────────────────────────────────────────────────────────┐
│ WrapLayoutCache (Computed for VPWidth = 20, WordWrapMode::On)    │
│   VPRow(0) -> VisualRowSpan { row: 0, start: 0,  width: 16 }     │
│               "The quick brown "                                 │
│   VPRow(1) -> VisualRowSpan { row: 0, start: 16, width: 19 }     │
│               "fox jumps over the "                              │
│   VPRow(2) -> VisualRowSpan { row: 0, start: 35, width: 8  }     │
│               "lazy dog"                                         │
└───────────────────────────────┬──────────────────────────────────┘
                                │
                                ▼
┌──────────────────────────────────────────────────────────────────┐
│ Compositor / OfsBuf (2D Cell Grid + Reverse-Video Virtual Caret) │
└──────────────────────────────────────────────────────────────────┘
```

## Implementation Plan

### Phase 1: Word Wrap Config Types

Define domain configuration types without boolean flags and wire the toggle event into the
editor component.

- [ ] In `tui/src/tui/editor/editor_engine/engine_struct.rs`, define `WordWrapMode` and
      add it to `EditorEngineConfig`:

    ```rust
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub enum WordWrapMode {
        #[default]
        Off,
        On,
    }
    ```

    - Add `pub word_wrap_mode: WordWrapMode` to `EditorEngineConfig` (defaulting to
      `WordWrapMode::Off` when `multiline_mode == LineMode::SingleLine`).
    - Add `pub fn toggle(&mut self)` on `WordWrapMode` to flip between `Off` and `On`.

- [ ] In `tui/src/tui/editor/editor_component/editor_event.rs`:
    - Add `EditorEvent::ToggleWordWrap`.
    - Map <kbd>Alt</kbd>+<kbd>Z</kbd> (`KeyPress::WithModifiers` with
      `Key::Character('z')` and `alt_key_state: KeyState::Pressed`) in
      `EditorEvent::try_from` to `EditorEvent::ToggleWordWrap`.
    - Handle `EditorEvent::ToggleWordWrap` in `EditorEvent::apply_editor_event` (ignoring
      toggle when `engine.config_options.multiline_mode == LineMode::SingleLine`,
      resetting `vp_origin.col_index` to `c_col(0)` when switching to `WordWrapMode::On`,
      and invalidating render/wrap caches).
- [ ] Run `./check.fish --check` and targeted unit tests for `WordWrapMode` and
      `EditorEvent::ToggleWordWrap`.
- [ ] **Mandatory manual review:** Verify domain enum design and event mapping.
    - [ ] `tui/src/tui/editor/editor_engine/engine_struct.rs`
    - [ ] `tui/src/tui/editor/editor_component/editor_event.rs`

### Phase 2: Visual Line Wrap Layout

Build the grapheme-safe word-wrapping layout calculator and cache mapping logical
`(CRow, CCol)` positions to wrapped visual rows.

- [ ] Create `tui/src/tui/editor/editor_engine/word_wrap/` following private-module and
      barrel re-export rules:
    - `tui/src/tui/editor/editor_engine/word_wrap/mod.rs`
    - `tui/src/tui/editor/editor_engine/word_wrap/visual_row_span_struct.rs`
    - `tui/src/tui/editor/editor_engine/word_wrap/wrap_layout_cache_struct.rs`
    - `tui/src/tui/editor/editor_engine/word_wrap/word_wrap_algorithm.rs`
- [ ] Define `VisualRowSpan` and `WrapLayoutCache`:

    ```rust
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct VisualRowSpan {
        pub logical_row: CRow,
        pub start_col: CCol,
        pub display_width: CWidth,
    }

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub struct WrapLayoutCache {
        cached_vp_width: VPWidth,
        spans: InlineVec<VisualRowSpan>,
        row_to_first_span: InlineVec<CIndex>,
    }
    ```

- [ ] Implement Unicode-aware wrapping over `GapBufferLine` grapheme segments (`Seg`) in
      `word_wrap_algorithm.rs`:
    - Break preferentially after ASCII/Unicode whitespace boundaries that fit within
      `VPWidth`.
    - Fall back to grapheme-boundary column breaks when a single unbroken token exceeds
      `VPWidth`.
    - Never split a 2-column wide grapheme (emoji or CJK glyph) across wrap boundaries; if
      only 1 column remains on the visual row, wrap the 2-column segment onto the next
      visual row.
    - Provide bidirectional coordinate helpers on `WrapLayoutCache`:
      `logical_to_visual_pos(CCaret) -> (CIndex, VPCol)` and
      `visual_to_logical_pos(CIndex, VPCol) -> CCaret`.
- [ ] Store `pub wrap_layout_cache: Option<WrapLayoutCache>` on `EditorEngine` and clear
      it on content mutation or viewport width change.
- [ ] Run `./check.fish --check` and unit tests covering ASCII word breaks, long unbroken
      URLs/tokens, empty lines, and 2-column emoji/CJK wrap boundaries.
- [ ] **Mandatory manual review:** Verify wrap layout math and coordinate conversions.
    - [ ] `tui/src/tui/editor/editor_engine/word_wrap/mod.rs`
    - [ ] `tui/src/tui/editor/editor_engine/word_wrap/visual_row_span_struct.rs`
    - [ ] `tui/src/tui/editor/editor_engine/word_wrap/wrap_layout_cache_struct.rs`
    - [ ] `tui/src/tui/editor/editor_engine/word_wrap/word_wrap_algorithm.rs`
    - [ ] `tui/src/tui/editor/editor_engine/engine_struct.rs`
    - [ ] `tui/src/tui/editor/editor_engine/mod.rs`

### Phase 3: Wrapped Rendering and Caret

Connect `WrapLayoutCache` to editor content rendering, selection highlighting, caret
painting, and vertical navigation.

- [ ] Update `tui/src/tui/editor/editor_engine/engine_public_api.rs` and
      `tui/src/tui/editor/editor_buffer/render_cache.rs`:
    - When `word_wrap_mode == WordWrapMode::On`, iterate visible `VisualRowSpan` entries
      starting from the visual scroll row up to `max_display_row_count` and rasterize each
      `[start_col, start_col + display_width)` slice into the target `OfsBuf` row.
    - Project `SelectionLine` column intersections onto each wrapped `VisualRowSpan` in
      `render_selection`.
    - Project `CCaret` through `logical_to_visual_pos` in `render_caret`.
- [ ] Update vertical navigation in `tui/src/tui/editor/editor_engine/caret_mut.rs`,
      `scroll_editor_content.rs`, and `validate_scroll_on_resize.rs`:
    - When `word_wrap_mode == WordWrapMode::On`, <kbd>Up</kbd> and <kbd>Down</kbd> move to
      the adjacent visual wrapped segment (within the same `CRow` or across adjacent
      `CRow` lines) at the corresponding `VPCol`.
    - <kbd>Home</kbd> and <kbd>End</kbd> move to the start/end of the current visual
      segment (pressing <kbd>Home</kbd> twice reaches `c_col(0)` of the logical line,
      matching VS Code).
    - <kbd>PageUp</kbd> and <kbd>PageDown</kbd> step by `VPHeight` visual rows.
- [ ] Run `./check.fish --check` and targeted editor engine rendering/navigation tests.
- [ ] **Mandatory manual review:** Verify wrapped rendering, selection, and caret motion.
    - [ ] `tui/src/tui/editor/editor_engine/engine_public_api.rs`
    - [ ] `tui/src/tui/editor/editor_buffer/render_cache.rs`
    - [ ] `tui/src/tui/editor/editor_engine/caret_mut.rs`
    - [ ] `tui/src/tui/editor/editor_engine/scroll_editor_content.rs`
    - [ ] `tui/src/tui/editor/editor_engine/validate_scroll_on_resize.rs`

### Phase 4: Integrate Word Wrap in edi

Expose word wrap configuration via CLI arguments and status bar UI in `edi`.

- [ ] In `cmdr/src/edi/clap_config.rs`, add `--word-wrap <on|off>` (`clap::ValueEnum`
      mapping to `WordWrapMode`, defaulting to `WordWrapMode::On` for Markdown editing in
      `edi`).
- [ ] In `cmdr/src/edi/launcher.rs` and `cmdr/src/edi/app_main.rs`:
    - Thread the resolved `WordWrapMode` into `EditorEngineConfig` when constructing
      `EditorComponent`.
    - Update `status_bar::render_status_bar` to show `Wrap: Alt+Z`.
- [ ] Add CLI schema tests in `cmdr/src/edi/clap_config.rs` and PTY integration tests
      verifying `--word-wrap` startup and <kbd>Alt</kbd>+<kbd>Z</kbd> runtime toggling.
- [ ] Run `./check.fish --full` and `remove-crate-prefix` check across all touched files.
- [ ] **Mandatory manual review:** Verify `edi` integration, docs, and full test suite.
    - [ ] `cmdr/src/edi/clap_config.rs`
    - [ ] `cmdr/src/edi/launcher.rs`
    - [ ] `cmdr/src/edi/app_main.rs`
    - [ ] `task/editor-word-wrap.md`
