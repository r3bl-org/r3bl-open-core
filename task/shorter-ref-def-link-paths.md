# Task: Shorten Intra-doc Link Paths using Barrel Exports

Audit and refactor intra-doc links in Rustdoc comments to use shorter, idiomatic paths made available by the project's barrel export pattern.

## Status
- [x] Audit complete
- [ ] Add missing re-export for `resilient_reactor_thread` in `tui/src/core/mod.rs`
- [ ] Refactor `tui/src/lib.rs` (lines 2500-2650)
- [ ] Refactor `tui/src/core/ansi/vt_100_pty_output_parser/operations/mod.rs`
- [ ] Refactor other identified modules in `core/`, `tui/`, and `readline_async/`
- [ ] Verify all links still resolve correctly (`cargo doc`)

## Background
The `r3bl-tui` crate uses a flattened API structure where most public types and modules are re-exported to the crate root. However, many doc comments still use long, physical file system paths (e.g., `crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::*`).

Per the `write-documentation` skill and project standards, these should be shortened to use `crate::` directly when possible, which reduces cognitive load and improves maintainability.

## Mapping of Shorter Paths

| Category | Long Physical Path (Example) | Shorter Idiomatic Path |
| :--- | :--- | :--- |
| **VT100 Impl** | `crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl` | `crate::vt_100_ansi_impl` |
| **VT100 Tests** | `crate::core::ansi::vt_100_pty_output_parser::vt_100_pty_output_conformance_tests` | `crate::vt_100_pty_output_conformance_tests` |
| **Coordinates** | `crate::core::coordinates::bounds_check` | `crate::bounds_check` |
| **Terminal IO** | `crate::core::ansi::terminal_raw_mode` | `crate::terminal_raw_mode` |
| **PTY** | `crate::core::pty::pty_engine::pty_pair::PtyPair` | `crate::PtyPair` |
| **Reactor** | `crate::core::resilient_reactor_thread::RRT` | `crate::RRT` (requires re-export) |
| **Editor** | `crate::tui::editor::editor_engine::EditorEngine` | `crate::EditorEngine` |
| **Gap Buffer** | `crate::tui::editor::zero_copy_gap_buffer::ZeroCopyGapBuffer` | `crate::ZeroCopyGapBuffer` |
| **MD Parser** | `crate::tui::md_parser::parse_markdown::parse_markdown` | `crate::parse_markdown` |
| **Syn Hi** | `crate::tui::syntax_highlighting::md_parser_syn_hi::try_parse_and_highlight` | `crate::try_parse_and_highlight` |

## Plan

### 1. Enable RRT Re-export
`RRT` and its related types are not currently re-exported in `tui/src/core/mod.rs`.
- Target: `tui/src/core/mod.rs`
- Add `pub use resilient_reactor_thread::*;` to the re-export block.

### 2. Refactor `tui/src/lib.rs`
This file contains many high-level documentation links that use very long paths. 
- Target: lines 2500-2650 (approx).
- Replace `crate::tui::terminal_lib_backends::*` with `crate::*`.
- Replace `crate::core::ansi::*` with `crate::*`.
- Replace `crate::tui::editor::*` with `crate::*`.
- Replace `crate::tui::md_parser::*` with `crate::*`.

### 3. Refactor `tui/src/core/ansi/vt_100_pty_output_parser/operations/mod.rs`
This was the original file identified.
- Replace the long `vt_100_impl_*` and `vt_100_test_*` links with their shorter equivalents.

### 4. Global Audit & Batch Refactor
Use `grep_search` to find other occurrences of:
- `crate::tui::terminal_lib_backends::`
- `crate::core::ansi::`
- `crate::core::coordinates::`
- `crate::core::pty::`
- `crate::tui::editor::`

Apply refactors to identified files.

### 5. Validation
- Run `./check.fish --doc` to ensure no links are broken.
- Verify the generated documentation still points to the correct items.

## Notes
- Only change paths in **documentation comments** (`///` or `//!`).
- Do not change `use` statements or code paths unless they are also unnecessarily long and can be shortened without affecting visibility.
- Keep `crate::` prefix as per `write-documentation` skill guidelines for intra-doc links.
