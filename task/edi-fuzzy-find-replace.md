# Task: Fuzzy Find and Replace in edi

## Overview

Adds interactive **Find** (<kbd>Ctrl</kbd>+<kbd>F</kbd>) and **Search + Replace**
(<kbd>Ctrl</kbd>+<kbd>H</kbd>) with **fzf-style fuzzy matching** and exact substring
matching to `r3bl_tui` and `edi`.

Users can fuzzy-search lines and tokens across the open buffer with live ranked results
and matched-character highlights, preview matches directly in the editor viewport while
navigating results, replace individual or all matches atomically, or press
<kbd>Alt</kbd>+<kbd>Enter</kbd> to turn all matches into multiple carets (integrating with
`task/editor-multi-caret.md`).

```text
┌────────────────────────────────────────────────────────────────────┐
│ Find / Replace Overlay (Modal / Top Bar in ZOrder::Glass)          │
│ ┌────────────────────────────────────────────────────────────────┐ │
│ │ 🔍 Find [Fuzzy Alt+F]:  editor buf          [3 of 12 matches]  │ │
│ │ 🔁 Replace:             document_buffer     [Enter / Ctrl+Alt] │ │
│ ├────────────────────────────────────────────────────────────────┤ │
│ │ ► 042: let editor_buffer = EditorBuffer::new_empty(());  (98)  │ │
│ │   108: fn render_editor_buf(buf: &EditorBuffer)          (84)  │ │
│ │   219: // Update editor state buffer                     (71)  │ │
│ └────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────┬──────────────────────────────────┘
                                  │
   Live viewport pan + match span highlights in EditorEngine;
   Alt+Enter spawns CaretSet across all matches;
   ReplaceAll runs bottom-to-top in one EditorHistory snapshot
                                  │
                                  ▼
┌────────────────────────────────────────────────────────────────────┐
│ EditorBuffer (ZeroCopyGapBuffer + CaretSet + EditorHistory)        │
└────────────────────────────────────────────────────────────────────┘
```

## Implementation Plan

### Phase 1: Fuzzy Matcher and Search Model

Build a grapheme-safe fzf-style fuzzy and substring search engine operating directly over
`ZeroCopyGapBuffer` lines using `CRow` and `CCol` display coordinates.

- [ ] Evaluate adding `nucleo-matcher` (pure-Rust Smith-Waterman fzf v2 matcher with zero
      transitive bloat) to `tui/Cargo.toml` vs a self-contained Smith-Waterman scorer in
      `tui/src/core/fuzzy_search/`, ensuring `CCol` display-column indices map cleanly
      through `GapBufferLine` grapheme segments (`Seg`).
- [ ] Create `tui/src/tui/editor/search/` following module organization rules:
    - `tui/src/tui/editor/editor_engine/search/mod.rs` (or
      `tui/src/tui/editor/search/mod.rs`)
    - `search_types.rs`
    - `search_matcher.rs`
    - `search_session_struct.rs`
- [ ] Define domain types without raw `bool` flags:

    ```rust
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub enum SearchMode {
        #[default]
        Fuzzy,
        SubstringCaseInsensitive,
        SubstringCaseSensitive,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct SearchMatch {
        pub row: CRow,
        pub start_col: CCol,
        pub end_col: CCol,
        pub score: u32,
        pub matched_cols: InlineVec<CCol>,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum ReplaceOutcome {
        ReplacedCount(CLength),
        NoMatches,
    }

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub struct SearchSession {
        pub query: InlineString,
        pub replacement: InlineString,
        pub mode: SearchMode,
        pub matches: InlineVec<SearchMatch>,
        pub active_match_index: Option<CIndex>,
        pub saved_origin_caret: Option<CCaret>,
        pub saved_vp_origin: Option<CPos>,
    }
    ```

- [ ] Implement `SearchSession::recompute(&mut self, lines: &ZeroCopyGapBuffer)`:
    - In `SearchMode::Fuzzy`, score lines/spans using fzf v2 boundary bonuses (word start,
      camelCase, contiguous runs), populate `matched_cols` for character-level highlight,
      and bound `start_col..end_col` from the first to last matched grapheme column.
    - Sort fuzzy matches by descending `score` (ties broken by `(CRow, CCol)`) for the fzf
      results list, and provide document-order next/previous match navigation helpers
      (`select_next_match`, `select_prev_match`).
- [ ] Run `./check.fish --check` and unit tests verifying fuzzy ranking, multi-byte
      emoji/CJK `CCol` accuracy, and mode switching.
- [ ] **Mandatory manual review:** Verify search types and grapheme-accurate fuzzy
      matching.
    - [ ] `tui/Cargo.toml`
    - [ ] `tui/src/tui/editor/search/mod.rs`
    - [ ] `tui/src/tui/editor/search/search_types.rs`
    - [ ] `tui/src/tui/editor/search/search_matcher.rs`
    - [ ] `tui/src/tui/editor/search/search_session_struct.rs`

### Phase 2: In-Buffer Highlight and Replace

Render match highlights inside the editor viewport, implement atomic single/all
replacement, and wire multi-caret spawning from search matches.

- [ ] Store `pub search_session: Option<SearchSession>` on `EditorEngine`
      (`tui/src/tui/editor/editor_engine/engine_struct.rs`).
- [ ] Update `tui/src/tui/editor/editor_engine/engine_public_api.rs`:
    - Add `render_search_matches` between `render_content` and `render_selection` to
      highlight visible `SearchMatch` spans (`start_col..end_col`), accenting individual
      `matched_cols` in fuzzy mode and emphasizing `active_match_index` while panning
      `Viewport` to keep the active match visible.
- [ ] Implement replacement and multi-caret conversion in
      `tui/src/tui/editor/editor_engine/content_mut.rs`:
    - `replace_active_search_match(EditorArgsMut<'_>) -> ReplaceOutcome`: replaces
      `start_col..end_col` of the active match, advances to the next match, and records an
      `EditorHistory` entry.
    - `replace_all_search_matches(EditorArgsMut<'_>) -> ReplaceOutcome`: sorts matches in
      reverse `(CRow, CCol)` order (skipping overlapping spans on the same line), applies
      all replacements inside a single `EditorBufferMutWithDrop` transaction with
      `ZeroCopyGapBuffer::rebuild_line_segments_batch`, and records a single atomic
      `EditorHistory` snapshot.
    - `spawn_carets_from_search_matches(EditorArgsMut<'_>)`: populates `CaretSet` (from
      `task/editor-multi-caret.md`) with one `CaretEntry` selecting `start_col..end_col`
      per match so the user can edit all matched sites interactively.
- [ ] Run `./check.fish --check` and unit tests covering `replace_active_search_match`,
      same-line multi-match `replace_all_search_matches`, single-step undo, and
      multi-caret spawning.
- [ ] **Mandatory manual review:** Verify match rendering, atomic replace, and undo
      safety.
    - [ ] `tui/src/tui/editor/editor_engine/engine_struct.rs`
    - [ ] `tui/src/tui/editor/editor_engine/engine_public_api.rs`
    - [ ] `tui/src/tui/editor/editor_engine/content_mut.rs`

### Phase 3: Find and Replace Component

Provide a reusable `ZOrder::Glass` interactive Find and Search+Replace overlay component
in `r3bl_tui` with live fzf results and keyboard navigation.

- [ ] Create `tui/src/tui/editor/search_component/` (or extend `tui/src/tui/dialog/` with
      `DialogEngineMode::ModalSearchReplace`):
    - `SearchOverlayMode { FindOnly, FindAndReplace }` and
      `FocusedSearchField { QueryInput, ReplacementInput }`.
    - Render single-line query input (plus replacement input in `FindAndReplace` mode),
      match counter (`M of N`), active `SearchMode` badge, and scrollable fzf results list
      highlighting `matched_cols` on each result line.
    - Keyboard controls:
        - <kbd>Up</kbd> / <kbd>Down</kbd> (or <kbd>Ctrl</kbd>+<kbd>N</kbd> /
          <kbd>Ctrl</kbd>+<kbd>P</kbd>): cycle matches and live-pan the underlying editor
          viewport.
        - <kbd>Tab</kbd> / <kbd>Shift</kbd>+<kbd>Tab</kbd>: toggle focus between Find and
          Replace inputs.
        - <kbd>Alt</kbd>+<kbd>F</kbd>: cycle `SearchMode` (`Fuzzy`,
          `SubstringCaseInsensitive`, `SubstringCaseSensitive`).
        - <kbd>Enter</kbd>: confirm jump to active match (in `FindOnly`) or replace active
          match and advance (in `FindAndReplace`).
        - <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>Enter</kbd>: `replace_all_search_matches`.
        - <kbd>Alt</kbd>+<kbd>Enter</kbd>: `spawn_carets_from_search_matches` and focus
          editor.
        - <kbd>Esc</kbd>: cancel overlay and restore `saved_origin_caret` /
          `saved_vp_origin` if unconfirmed.
- [ ] Run `./check.fish --check` and component-level event/render unit tests.
- [ ] **Mandatory manual review:** Verify search/replace overlay UX and focus transitions.
    - [ ] `tui/src/tui/editor/search_component/mod.rs`
    - [ ] `tui/src/tui/editor/mod.rs`

### Phase 4: Wire Shortcuts in edi

Integrate Find and Search+Replace into `edi` and verify end-to-end interactive workflows.

- [ ] Update `cmdr/src/edi/app_main.rs` and `cmdr/src/edi/state.rs`:
    - Add `AppSignal::OpenFind` (<kbd>Ctrl</kbd>+<kbd>F</kbd>) and
      `AppSignal::OpenFindAndReplace` (<kbd>Ctrl</kbd>+<kbd>H</kbd>).
    - Note terminal raw-mode nuance: verify <kbd>Ctrl</kbd>+<kbd>H</kbd> vs
      <kbd>Backspace</kbd> (`0x08` vs `0x7F` / Kitty keyboard protocol) across
      `DirectToAnsi` and Crossterm backends, providing <kbd>Ctrl</kbd>+<kbd>R</kbd> as an
      additional or fallback Replace shortcut on legacy terminals that alias
      <kbd>Ctrl</kbd>+<kbd>H</kbd> to `Backspace`.
    - Register the search/replace overlay component in `populate_component_registry` and
      render it in `ContainerSurfaceRender`.
    - Update `status_bar::render_status_bar` to include `Find: Ctrl+F` and
      `Replace: Ctrl+H`.
- [ ] Add PTY integration tests for fuzzy find jump, replace all + undo, and
      <kbd>Alt</kbd>+<kbd>Enter</kbd> multi-caret handoff in `edi`.
- [ ] Run `./check.fish --full` and `remove-crate-prefix` check across all modified files.
- [ ] **Mandatory manual review:** Verify `edi` bindings, terminal compatibility, and
      tests.
    - [ ] `cmdr/src/edi/app_main.rs`
    - [ ] `cmdr/src/edi/state.rs`
    - [ ] `task/edi-fuzzy-find-replace.md`
