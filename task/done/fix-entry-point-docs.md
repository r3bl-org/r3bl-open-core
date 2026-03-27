# Task: Fix entry point docs in lib.rs and README.md [COMPLETE]

## Overview

The "Full TUI, Partial TUI, and async readline" section in `tui/src/lib.rs` has duplicate
subsections: three new API descriptions (numbered 1-3) collide with the four pre-existing
example-oriented subsections. These need to be merged into four clean subsections that
combine the API descriptions with the examples.

## Implementation plan

### Phase 1: Merge subsections in `tui/src/lib.rs`

Remove the three numbered subsections (`## 1. Full TUI: ...`, `## 2. Async Readline ...`,
`## 3. Terminal Multiplexer: ...`) and fold their content into the four pre-existing
subsections. The entry points table stays as the quick-reference after the `#` heading.
Restore the YouTube video links after the intro paragraph (before the entry points table):
- `[Build with Naz: TTY playlist](https://www.youtube.com/playlist?list=PLofhE49PEwmw3MKOU1Kn3xbP4FRQR4Mb3)`
- `[Build with Naz: async readline](https://www.youtube.com/playlist?list=PLofhE49PEwmwelPkhfiqdFQ9IXnmGdnSE)`

- [x]**"Full TUI for immersive apps"**: Keep old heading. Start with the new API bullet
  points (`App` trait, `FlexBox`, Component System), then the old text ("The bulk of this
  document...") with the `edi` example.
- [x]**"Partial TUI for simple choice"**: Keep old heading. Add the new description
  (`choose()` for interactive selection) before the old `giti` example. The old text
  already links `mod@readline_async::choose_api`.
- [x]**"Partial TUI for REPL"**: Keep as-is. It already describes `readline_async_api`,
  `SharedWriter`, spinners - the new content for entry point #2 is largely the same.
- [x]**"Terminal multiplexer"**: Keep as-is. Already has the `PTYMux` description +
  example link.
- [x]**"Power via composition"**: Keep old text.

### Phase 2: Update TOC in `tui/src/lib.rs`

- [x]Replace the current TOC entries for this section with:
  ```
  - [Full TUI, Partial TUI, and async readline](#...)
    - [Full TUI for immersive apps](#full-tui-for-immersive-apps)
    - [Partial TUI for simple choice](#partial-tui-for-simple-choice)
    - [Partial TUI for REPL](#partial-tui-for-repl)
    - [Terminal multiplexer](#terminal-multiplexer)
    - [Power via composition](#power-via-composition)
  ```

### Phase 3: Regenerate `tui/README.md` and update root `README.md`

- [x]Run `cargo readme > tui/README.md` to regenerate from `lib.rs`.
- [x]Update root `README.md` to match the same four-subsection structure (using plain
  markdown links instead of rustdoc syntax).
