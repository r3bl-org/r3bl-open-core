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
- [x] [DA1 Responses timeout fix](done/pr-455-fix.md) -
      https://github.com/r3bl-org/r3bl-open-core/pull/455
- [x] [VT100 Pending Wrap fix](done/pr-456-fix.md) -
      https://github.com/r3bl-org/r3bl-open-core/pull/456
- [x] [Mouse Event Forwarding](done/pr-458-fix.md) -
      https://github.com/r3bl-org/r3bl-open-core/pull/458
- [x] [DECCKM Cursor Key Mode tracking & state refactor](https://github.com/r3bl-org/r3bl-open-core/pull/470)
- [x] [Scrollback Buffer for PTY](done/pr-459-fix.md) -
      https://github.com/r3bl-org/r3bl-open-core/pull/459
- [ ] OfsBufVT100 Canvas and Viewport Refactor
    - [x] [OfsBuf backing store change to Flat2DArray](done/ofsbuf_flat2darray_backing_store.md)
    - [x] [Script for YT Video on Flat2DArray and SIMD](done/yt_script_flat2darray_plan.md)
    - [ ] [OfsBuf Growable, Canvas and Viewport](ofsbuf_trait_growable_impl.md)
- [ ] **NEEDS RESEARCH & PLANNING** https://github.com/r3bl-org/r3bl-open-core/pull/472
- [ ] [invert control and decouple UI in pty_mux](pty-mux-invert-control.md)
- [ ] **NEEDS RESEARCH & PLANNING** https://github.com/r3bl-org/r3bl-open-core/pull/468
- [ ] **NEEDS RESEARCH & PLANNING** https://github.com/r3bl-org/r3bl-open-core/pull/466
- [ ] **NEEDS RESEARCH & PLANNING** https://github.com/r3bl-org/r3bl-open-core/pull/467
- [ ] **NEEDS RESEARCH & PLANNING** https://github.com/r3bl-org/r3bl-open-core/pull/464
- [ ] **NEEDS RESEARCH & PLANNING** https://github.com/r3bl-org/r3bl-open-core/pull/465
- [ ] **NEEDS RESEARCH & PLANNING** https://github.com/r3bl-org/r3bl-open-core/pull/469
- [ ] [wire up bracketed paste in pty_mux](pty-mux-bracketed-paste.md)
      https://github.com/r3bl-org/r3bl-open-core/pull/471
- [ ] [fix fish shell issues in pty_mux module](task/fix-fish-in-pty-mux.md)

# [TODO] Unify rendering

- [ ] [unify the interactive entry points to alternate screen](task_refactor_interactive_apis_to_alternate_screen.md)
- [ ] [unify styling](task_unify_cli_and_styled_text.md)

# [TODO] Clean up tasks

- [ ] [upgrade-range-for-rust_1_96_0.md](upgrade-range-for-rust_1_96_0.md)
- [ ] [rustdocs - fix readability of esc codes](fix-esc-code-formatting.md)

# [TODO] build-infra tasks

- [ ] [build-infra-add-more-terms-to-seed-jsonc.md](build-infra-add-more-terms-to-seed-jsonc.md)
- [ ] [build-infra-upgrade-support.md](build-infra-upgrade-support.md)

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
