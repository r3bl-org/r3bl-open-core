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

- [x] [fix linux perf problem](done/fix-yield_now-slowdown-on-linux.md)
- [ ] [binaries-self-upgrade-support.md](binaries-self-upgrade-support.md)
- [ ] [build-infra-add-more-terms-to-seed-jsonc.md](build-infra-add-more-terms-to-seed-jsonc.md)

# [TODO] Unify rendering

- [ ] [unify the interactive entry points to alternate screen](task_refactor_interactive_apis_to_alternate_screen.md)
- [ ] [unify styling](task_unify_cli_and_styled_text.md)

# [TODO] Clean up tasks

- [ ] [upgrade-range-for-rust_1_96_0.md](upgrade-range-for-rust_1_96_0.md)
- [ ] [rustdocs - fix readability of esc codes](fix-esc-code-formatting.md)

# [TODO] Release Verification & Publication

- [ ] [Mirror docs](mirror-3-ext-sites-to-docs-specs.md)
- [ ] **Code Quality & Documentation**
    - [ ] Run `./check.fish --full` to verify Linux builds, tests, clippy, and rustdoc
          generation.
    - [ ] Update `CHANGELOG.md` to comprehensively reflect this massive & breaking release
          (e.g., PTY multiplexer, VT100 parser extraction, scrollback, timeout fixes).
- [ ] **Cross-Platform Manual Verification**
    - [ ] macOS: Run interactive PTY examples (e.g.,
          `cargo run --example pty_mux_example`) and verify mouse input, scrollback, and
          DA1 timeout fixes.
    - [ ] Windows: Boot Windows VM/environment, verify compilation, and test interactive
          TUI/PTY examples.
- [ ] **Publication Workflow (via `/release` skill)**
    - [ ] `r3bl_tui`
        - [ ] Bump version numbers to `0.8.0` in `Cargo.toml` (workspace and/or crates).
        - [ ] Run `cargo publish --dry-run`.
        - [ ] Publish to crates.io.
        - [ ] Create and push git tag `v0.8.0`.
        - [ ] Draft and publish a GitHub Release using the updated changelog notes.
    - [ ] `r3bl-build-infra`
        - [ ] Bump version numbers to `???` in `Cargo.toml` (workspace and/or crates).
        - [ ] Run `cargo publish --dry-run`.
        - [ ] Publish to crates.io.
        - [ ] Create and push git tag `v???`.
        - [ ] Draft and publish a GitHub Release using the updated changelog notes.
