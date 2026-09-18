_Meta Task: Prepare v0.8.0 Release_

# [DONE] PTY MUX UI Freeze

- [x] [fix-pty-mux-debug-session.md](done/fix-pty-mux-debug-session.md)

# [DONE] Polling and Event Loop Fixes

- [x] https://github.com/r3bl-org/r3bl-open-core/pull/450
- [x] [fix-mio-poller-edge-triggered-polling.md](done/fix-mio-poller-edge-triggered-polling.md)
- [x] [Fix bug introduce by mio-poller-edge-triggered-polling](https://github.com/r3bl-org/r3bl-open-core/issues/453)

# [DONE] Terminal Parsing

- [x] [improve-immature-vt100-shim.md](done/improve-immature-vt100-shim.md)
- [x] [pr-448-fix.md](done/pr-448-fix.md) -
      https://github.com/r3bl-org/r3bl-open-core/pull/448
- [x] [issue-451-fix.md](done/issue-451-fix.md) -
      https://github.com/r3bl-org/r3bl-open-core/issues/451

# [DONE] RRT API

- [x] [pr-452-fix.md](done/pr-452-fix.md) -
      https://github.com/r3bl-org/r3bl-open-core/pull/452

# [DONE] Cursor display issues

- [x] [issue-461-fix.md](done/issue-461-fix.md) -
      https://github.com/r3bl-org/r3bl-open-core/issues/461

# [DONE] Core Types Refactor

- [x] [remove crossterm mental model pollution](done/remove-crossterm-mental-model-pollution.md)

# [WIP] Complete PRs from Cecile

- [x] [LF scroll-up test fix](done/pr-462-fix.md) -
      https://github.com/r3bl-org/r3bl-open-core/pull/462
- [x] [DA1 responses timeout fix](done/pr-455-fix.md) -
      https://github.com/r3bl-org/r3bl-open-core/pull/455
- [x] [VT100 pending wrap fix](done/pr-456-fix.md) -
      https://github.com/r3bl-org/r3bl-open-core/pull/456
- [x] [add mouse event forwarding](done/pr-458-fix.md) -
      https://github.com/r3bl-org/r3bl-open-core/pull/458
- [x] [DECCKM Cursor Key Mode tracking & state refactor](https://github.com/r3bl-org/r3bl-open-core/pull/470)
- [x] [add scrollback buffer for PTY](done/pr-459-fix.md) -
      https://github.com/r3bl-org/r3bl-open-core/pull/459
- [x] OfsBufVT100 Canvas and Viewport Refactor
    - [x] [OfsBuf backing store change to Flat2DArray](done/ofsbuf_flat2darray_backing_store.md)
    - [x] [Script for YT Video on Flat2DArray and SIMD](done/yt_script_flat2darray_plan.md)
    - [x] Canvas and Viewport Refactor
        - [x] [OfsBuf Growable, Canvas and Viewport, pan normal mode apps](done/ofsbuf_trait_growable_impl.md)
        - [x] [clean up units](done/refactor-units.md)
        - [x] [clean up Canvas & Viewport API](done/cleanup_viewport.md)
        - [x] [clean up coordinate types](done/rename-buffer-coords.md)
        - [x] [update editor](done/modernize-editor-using-new-units.md)
            - [x] [use viewport bounds](done/viewport_bounds_check.md)
            - [x] [use method overloading](done/use_method_overloading.md)
            - [x] [update editor to use viewport](done/migrate-scroll-offset-to-vp-origin.md)
            - [x] [use anchor and line for selection model](done/migrate-selection-to-anchor-and-line-selection.md)
            - [x] [modernize buffer_struct.rs](done/buffer-struct-modernize.md)
            - [x] [unify viewport coords constructors](done/unify-viewport-coords-constructors.md)
            - [x] [fix calling macro in macro](done/fix-future-incompat-warnings-2.md)
            - [x] [fix star-history outage](done/star-history-replace.md)
            - [x] fix rust-analyzer mcp server:
                - [x] [investigate and remove broken rust mcp server](done/fix-rust-analyzer-mcp-server.md)
                - [x] [rewrite native rust-analyzer-mcp-server in build-infra](done/create-build-infra-rust-analyzer-mcp-server.md)
                - [x] [publish new rust-analyzer mcp server repo & crate](done/publish-new-mcp-server-crate.md)
                - [x] [write TWiR article - when not to use tokio](done/write-twir-article-for-no-tokio-stdio-mcp-server.md)
    - [ ] [Enable mouse in editor](editor-mouse-enable.md)
    - [ ] [Update Layout Engine](modernize-layout-engine.md)
    - [ ] [rasterize editor component rendering](rasterize-editor-component-rendering.md)
- [x] [fix linux perf problem](done/fix-yield_now-slowdown-on-linux.md)
- [ ] **NEEDS RESEARCH & PLANNING** https://github.com/r3bl-org/r3bl-open-core/pull/473
- [ ] [invert control and decouple UI in pty_mux](pty-mux-invert-control.md)
- [ ] **NEEDS RESEARCH & PLANNING** https://github.com/r3bl-org/r3bl-open-core/pull/468
- [ ] **NEEDS RESEARCH & PLANNING** https://github.com/r3bl-org/r3bl-open-core/pull/466
- [ ] **NEEDS RESEARCH & PLANNING** https://github.com/r3bl-org/r3bl-open-core/pull/467
- [ ] **NEEDS RESEARCH & PLANNING** https://github.com/r3bl-org/r3bl-open-core/pull/464
- [ ] **NEEDS RESEARCH & PLANNING** https://github.com/r3bl-org/r3bl-open-core/pull/465
- [ ] **NEEDS RESEARCH & PLANNING** https://github.com/r3bl-org/r3bl-open-core/pull/469
- [ ] **NEEDS RESEARCH & PLANNING** https://github.com/r3bl-org/r3bl-open-core/pull/476
- [ ] **NEEDS RESEARCH & PLANNING** https://github.com/r3bl-org/r3bl-open-core/pull/479
- [ ] **NEEDS RESEARCH & PLANNING** https://github.com/r3bl-org/r3bl-open-core/pull/480
- [ ] [wire up bracketed paste in pty_mux](pty-mux-bracketed-paste.md)
      https://github.com/r3bl-org/r3bl-open-core/pull/471
- [ ] [fix fish shell issues in pty_mux module](task/fix-fish-in-pty-mux.md)

# [TODO] build-infra tasks

- [x] [add-env-source.md](done/add-env-source.md)
- [x] [fix-win-conpty-eof.md](done/fix-win-conpty-eof.md)
- [x] [add-academic-research-on-type-safety-at-scale.md](done/add-academic-research-on-type-safety-at-scale.md)
- [x] [clean-command-result-handling.md](done/clean-command-result-handling.md)
- [x] [fix-windows-tests-tui-term-api-and-mcp.md](done/fix-windows-tests-tui-term-api-and-mcp.md)
- [x] [make-pty-session-sync.md](done/make-pty-session-sync.md)
- [x] [support-worktree-in-check-script.md](support-worktree-in-check-script.md)
- [ ] [fix-shift-home-lockup.md](fix-shift-home-lockup.md)
- [ ] [make-0.8.0-release.md](make-0.8.0-release.md)
- [ ] [build-infra-spawny.md](build-infra-spawny.md)
- [ ] [binaries-self-upgrade-support.md](binaries-self-upgrade-support.md)
- [ ] TODO - dl article on eliminate off by one errors (for which we already have a video)
- [ ] [dl-article-type-safety-at-scale.md](dl-article-type-safety-at-scale.md)
- [ ] [build-infra-add-more-terms-to-seed-jsonc.md](build-infra-add-more-terms-to-seed-jsonc.md)

# [TODO] Unify rendering

- [ ] [unify the interactive entry points to alternate screen](task_refactor_interactive_apis_to_alternate_screen.md)
- [ ] [unify styling](task_unify_cli_and_styled_text.md)

# [TODO] Clean up tasks

- [ ] [upgrade-range-for-rust_1_96_0.md](upgrade-range-for-rust_1_96_0.md)
- [ ] [rustdocs - fix readability of esc codes](fix-esc-code-formatting.md)

# [TODO] Release Verification & Publication

Detailed execution plan is maintained in [make-0.8.0-release.md](make-0.8.0-release.md).

## Parallel Workstreams

```text
┌────────────────────────────────────────────────────────┐
│ Track A (Parallel - Available Now on main):            │
│ 1. Create docs/release-notes/<crate>/ structure        │
│ 2. Backfill historical 19 releases via gh release      │
│ 3. Pre-draft v0.8.0 release notes & migration guides   │
│ 4. Pre-draft CHANGELOG.md entries & update TOC         │
└──────────────────────────┬─────────────────────────────┘
                           │
┌──────────────────────────▼─────────────────────────────┐
│ Track B (Sequential - After worktree merge):           │
│ 1. Complete & merge fix-shift-home-lockup to main      │
│ 2. Update tui/src/lib.rs (SSOT) & generate README.md   │
│ 3. Update root README.md & all workspace Cargo.tomls   │
│ 4. Execute sequential multi-crate release DAG:         │
│    r3bl_tui -> r3bl-build-infra -> cmdr -> mcp-server  │
└────────────────────────────────────────────────────────┘
```

### Track A: Pre-Release Documentation, Release Notes & Changelogs (In Parallel Now)

- [ ] [Mirror docs](mirror-3-ext-sites-to-docs-specs.md)
- [ ] **Create Release Notes Directory Structure**:
    - `docs/release-notes/r3bl_tui/`
    - `docs/release-notes/r3bl-cmdr/`
    - `docs/release-notes/r3bl-build-infra/`
    - `docs/release-notes/r3bl-rust-analyzer-mcp-server/` (starts at `v1.1.5.md`)
    - `docs/release-notes/archived/`
- [ ] **Backfill Historical 19 GitHub Releases**:
    - Extract release bodies via
      `gh release view <tag> --json body --jq .body > docs/release-notes/<crate>/<version>.md`.
    - Commit explicitly:
      `git add docs/release-notes/ && git commit -m "docs: backfill historical release notes"`.
- [ ] **Pre-Draft Standalone Release Notes & Migration Guides**:
    - [ ] `docs/release-notes/r3bl_tui/v0.8.0.md` (Discoverability intro, Migration Guide,
          benchmarks, SIMD/Flat2DArray, FUNARCH type safety, link to `CHANGELOG.md`).
    - [ ] `docs/release-notes/r3bl-build-infra/v0.0.6.md` (`cargo-rustdoc-fmt` technical
          term linking).
    - [ ] `docs/release-notes/r3bl-cmdr/v0.0.27.md` (`env-source`, `edi`, `giti`).
    - [ ] `docs/release-notes/r3bl-rust-analyzer-mcp-server/v1.1.5.md` (Stdlib thread MCP
          server, devlife article link).
    - Commit explicitly:
      `git add docs/release-notes/ && git commit -m "docs: pre-draft v0.8.0 standalone release notes"`.
- [ ] **Pre-Draft `CHANGELOG.md` Entries**:
    - [ ] Add `v0.8.0` for `r3bl_tui`, `v0.0.6` for `r3bl-build-infra`, `v0.0.27` for
          `r3bl-cmdr`, `v1.1.5` for `r3bl-rust-analyzer-mcp-server`.
    - [ ] Update TOC using `mktoc`.
    - Commit explicitly:
      `git add CHANGELOG.md && git commit -m "docs: pre-draft v0.8.0 changelog entries"`.
- [ ] **Mandatory manual review:** Verify `docs/release-notes/` and `CHANGELOG.md` files
      are complete and correctly formatted.

### Track B: Release Execution (Post-Merge of `fix-shift-home-lockup`)

- [ ] **Merge Worktree & Documentation SSOT Sync**:
    - [ ] Merge `../roc-fix-shift-home-lockup` into `main`.
    - [ ] Update `tui/src/lib.rs` (SSOT under `//! # Why R3BL?`) with Type Safety and
          Systems Performance sections.
    - [ ] Generate `tui/README.md`: `cd tui && cargo readme > README.md && cd ..`.
    - [ ] Update root `README.md` and run `mktoc`.
    - [ ] Update all `Cargo.toml` files (`tui` to `0.8.0`, others with
          `r3bl_tui = "0.8.0"`).
- [ ] **Cross-Platform Verification & Quality Checks**:
    - [ ] Run `./check.fish --full` (Linux builds, tests, clippy, docs).
    - [ ] macOS: Run interactive PTY examples (`cargo run --example pty_mux_example`).
    - [ ] Windows: Verify compilation and interactive TUI/PTY examples.
- [ ] **Sequential Multi-Crate Release (Detailed in
      [make-0.8.0-release.md](make-0.8.0-release.md))**:
    - [ ] **Step 1: Release `r3bl_tui` v0.8.0 (Core Library)**
        - Perform dry run: `cd tui && cargo publish --dry-run --allow-dirty --no-verify`.
        - User permission checkpoint.
        - Commit & Tag (explicit):
          `git add tui/ README.md Cargo.lock Cargo.toml build-infra/Cargo.toml cmdr/Cargo.toml rust-analyzer-mcp-server/Cargo.toml docs/release-guide.md && git commit -m "v0.8.0-tui" && git tag -a v0.8.0-tui -m "v0.8.0-tui"`.
        - Publish to crates.io: `cd tui && cargo publish --no-verify --allow-dirty`.
        - Verify live on crates.io: `cargo search r3bl_tui`.
        - Push commit/tag and create GitHub release via
          `--notes-file docs/release-notes/r3bl_tui/v0.8.0.md`.
    - [ ] **Step 2: Release `r3bl-build-infra` v0.0.6 (Tooling Crate)**
        - Generate README from SSOT: `cd build-infra && cargo readme > README.md`.
        - Perform dry run:
          `cd build-infra && cargo publish --dry-run --allow-dirty --no-verify`.
        - User permission checkpoint.
        - Commit & Tag (explicit):
          `git add build-infra/ && git commit -m "v0.0.6-build-infra" && git tag -a v0.0.6-build-infra -m "v0.0.6-build-infra"`.
        - Publish to crates.io:
          `cd build-infra && cargo publish --no-verify --allow-dirty`.
        - Push commit/tag, create GitHub release via `--notes-file`, and re-install binary
          (`cargo install --path build-infra --force`).
    - [ ] **Step 3: Release `r3bl-cmdr` v0.0.27 (CLI & Apps Crate)**
        - Generate README from SSOT: `cd cmdr && cargo readme > README.md`.
        - Perform dry run: `cd cmdr && cargo publish --dry-run --allow-dirty --no-verify`.
        - User permission checkpoint.
        - Commit & Tag (explicit):
          `git add cmdr/ && git commit -m "v0.0.27-cmdr" && git tag -a v0.0.27-cmdr -m "v0.0.27-cmdr"`.
        - Publish to crates.io: `cd cmdr && cargo publish --no-verify --allow-dirty`.
        - Push commit/tag, create GitHub release via `--notes-file`, and re-install binary
          (`cargo install --path cmdr --force`).
    - [ ] **Step 4: Release `r3bl-rust-analyzer-mcp-server` v1.1.5 (MCP Server)**
        - _Do NOT run cargo readme_ (README.md is hand-crafted SSOT for crates.io).
        - Perform dry run:
          `cd rust-analyzer-mcp-server && cargo publish --dry-run --allow-dirty --no-verify`.
        - User permission checkpoint.
        - Commit & Tag (explicit):
          `git add rust-analyzer-mcp-server/ && git commit -m "v1.1.5-rust-analyzer-mcp-server" && git tag -a v1.1.5-rust-analyzer-mcp-server -m "v1.1.5-rust-analyzer-mcp-server"`.
        - Publish to crates.io:
          `cd rust-analyzer-mcp-server && cargo publish --no-verify --allow-dirty`.
        - Push commit/tag, create GitHub release via `--notes-file`, and re-install binary
          (`cargo install --path rust-analyzer-mcp-server --force`).
- [ ] **Post-Release Housekeeping & Visibility**:
    - [ ] Share release announcements across developer communities (Reddit `r/rust`,
          Hacker News Show HN, and LinkedIn) for `r3bl_tui`, `r3bl-cmdr`,
          `r3bl-build-infra`, and `r3bl-rust-analyzer-mcp-server`.
    - [ ] Mark meta-tasks and release tasks complete.
- [ ] **Mandatory manual review:** Verify all release steps across crates.io, GitHub
      releases, and local binaries.

# Future tasks

- [rust-dojo-and-r3bl-runner.md](rust-dojo-and-r3bl-runner.md)
- [agent-runner.md](agent-runner.md)
- [new-call-chain-ext.md](../../r3bl-vscode-extensions/task/new-call-chain-ext.md)

<!-- cspell:words windowstests -->
