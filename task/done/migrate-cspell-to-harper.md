# Task: Migrate CSpell Directives to Harper and Clean Up Codebase

## Overview

All 862 project-wide spelling exclusions have been consolidated into
`.harper-dictionary.txt` as the Single Source of Truth (SSOT). Because Harper operates at
the workspace level and only checks comments and Markdown (completely ignoring code
tokens), the legacy inline `cspell:words` and `<!-- cspell:words -->` comments across the
codebase are redundant.

This task systematically removes obsolete `cspell` directives across 170 files, converts
intentional mock/test blocks to `spellcheck:ignore`, removes legacy `"cSpell.words"`
configuration from `.vscode/settings.json`, and validates that the workspace remains clean
and error-free.

## Implementation plan

### Phase 1: Tooling, Workspace Configs, & Shell Scripts (7 files)

- [x] Remove legacy `"cSpell.words": [...]` block from `.vscode/settings.json`.
- [x] Remove inert `cspell` directives from `check.fish`.
- [x] Remove inert `cspell` directives from `run.fish`.
- [x] Remove inert `cspell` directives from `bootstrap.sh`.
- [x] Remove inert `cspell` directives from `check_orchestrators.fish`.
- [x] Remove inert `cspell` directives from `.cargo/config.toml`.
- [x] Remove inert `cspell` directives from `.vscode/rust.json.code-snippets`.
- [x] Verify build via `./check.fish --check`.
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `.vscode/settings.json`
    - [x] `check.fish`
    - [x] `run.fish`
    - [x] `bootstrap.sh`
    - [x] `check_orchestrators.fish`
    - [x] `.cargo/config.toml`
    - [x] `.vscode/rust.json.code-snippets`

### Phase 2: Documentation & Task Markdown Files (42 files)

- [x] Clean up `<!-- cspell:words ... -->` directives across architectural docs in `docs/`
      (`docs/ril.md`, `docs/memory_architecture.md`, `docs/pty_mux_architecture.md`,
      etc.).
- [x] Clean up `<!-- cspell:words ... -->` directives across task files in `task/`
      (`task/fast-stringify-write-to.md`, `task/modernize-layout-engine.md`, etc.).
- [x] Ensure any mock/test sequence blocks requiring skipping use
      `<!-- spellcheck:ignore -->`.
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `docs/` modified Markdown files
    - [x] `task/` modified Markdown files

### Phase 3: CLI & Infrastructure Crates (19 files)

- [x] Remove `// cspell:words ...` directives from `cmdr/` source files
      (`cmdr/src/bin/edi.rs`, `cmdr/src/bin/giti.rs`, `cmdr/src/bin/rc.rs`, etc.).
- [x] Remove `// cspell:words ...` directives from `build-infra/` source files.
- [x] Remove `// cspell:words ...` directives from `rust-analyzer-mcp-server/` source
      files.
- [x] Remove `// cspell:words ...` directives from `analytics_schema/` source files.
- [x] Verify build via `./check.fish --check`.
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `cmdr/` modified files
    - [x] `build-infra/` modified files
    - [x] `rust-analyzer-mcp-server/` modified files
    - [x] `analytics_schema/` modified files

### Phase 4: `tui` Crate — Core Submodules (72 files)

- [x] Batch 4A: Clean up PTY & OS engine files (`tui/src/core/pty/...`,
      `tui/src/core/terminal_io/...`).
- [x] Batch 4B: Clean up Resilient Reactor & event poller files
      (`tui/src/core/resilient_reactor_thread/...`,
      `tui/src/core/terminal_io/backpressure_stdout/...`).
- [x] Batch 4C: Clean up Coordinates, Canvas, & Common utility files
      (`tui/src/core/coordinates/...`, `tui/src/core/common/...`).
- [x] Verify build via `./check.fish --check`.
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `tui/src/core/` modified files

### Phase 5: `tui` Crate — UI Engine, Readline Async & Backends (30 files)

- [x] Remove `// cspell:words ...` directives from `tui/src/readline_async/...`.
- [x] Remove `// cspell:words ...` directives from `tui/src/tui/editor/...`.
- [x] Remove `// cspell:words ...` directives from `tui/src/tui/terminal_lib_backends/...`
      (`direct_to_ansi`, `ofs_buf`).
- [x] Remove `// cspell:words ...` directives from `tui/src/lib.rs` and
      `tui/src/tui/mod.rs`.
- [x] Verify build and docs via `./check.fish --check` and `./check.fish --quick-doc`.
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `tui/src/readline_async/` modified files
    - [x] `tui/src/tui/` modified files
    - [x] `tui/src/lib.rs`

### Phase 6: Final Verification & Audit

- [x] Run `git grep -i "cspell"` to confirm zero residual `cspell` directives exist across
      the repository.
- [x] Run `./check.fish --test` to ensure full test suite passes.
- [x] **Mandatory manual review:** Verify repository integrity and all modified files
      before final sign-off.
    - [x] `task/migrate-cspell-to-harper.md`

## Harper `spellcheck:ignore` Mechanics & Guidelines

During the migration and diagnostic verification across the codebase, the following
Harper behavior was documented:

1. **Supported Inline Directives:**
   Harper's `CommentMasker` recognizes the following substrings inside code comments:
   - `spellchecker:ignore` or `spellchecker: ignore`
   - `spellcheck:ignore` or `spellcheck: ignore`
   - `cspell:ignore` or `cspell: ignore`
   - `harper:ignore` or `harper: ignore`

2. **Entire Comment Block Masking:**
   When Harper finds any of the above directives, it masks out and ignores the **entire
   comment block** (e.g. all contiguous `//` or `///` lines). It does **not** parse
   individual word lists and does **not** act as a multi-line toggle (no `disable` / `enable`
   pairing).

3. **Rust Doc Comments Caution:**
   In Rust, contiguous `///` (or `//!`) lines attached to an item belong to a single AST
   comment node. Adding `/// spellcheck:ignore` above a reference link will mask the
   **entire doc comment**, disabling spellcheck for the actual documentation text.

4. **Best Practices for Clean Harper Diagnostics:**
   - **Reference definition links**: Keep on a single line (`[label]: destination`). Harper's
     Markdown parser identifies single-line links as `link_destination` tokens and ignores
     them. `rustfmt` does not wrap markdown reference links.
   - **Paths & Code Tokens in Prose**: Enclose file paths and code tokens in backticks
     (e.g., `` `/dev/tty` ``, `` `stdin` ``, `` `fd` ``).
   - **Technical Terms**: Add project-wide terms and acronyms to `.harper-dictionary.txt`
     (e.g., `os`, `pty`, `io`, `stdin`, `stdout`, `ESC`, `tty`).
