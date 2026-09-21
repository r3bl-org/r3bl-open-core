# Task: Make 0.8.0 Release for r3bl_tui and Dependent Workspace Crates

## 1. Overview & Strategy

This task orchestrates a coordinated multi-crate release anchored by the major breaking
release `r3bl_tui = "0.8.0"`.

### Why v0.8.0 is a Groundbreaking Release

1. **Mathematically & Empirically Validated Type Safety**:
    - Coordinate systems and viewport camera abstractions enforce strict domain boundaries
      between Canvas-absolute positions (`CPos`, `CCaret`, `CCol`, `CRow`, `CWidth`,
      `CHeight`) and Viewport-relative positions (`VPPos`, `VPCaret`, `VPCol`, `VPRow`,
      `VPWidth`, `VPHeight`).
    - Domain safety is grounded in programming language research:
        - **Theoretical Foundations**: Will Crichton (Stanford CS 242 & FUNARCH 2023
          paper) on the Typestate pattern, State Machine pattern, The Witness pattern, and
          self-consuming invalidation, combined with Alexis King's _Parse, don't
          validate_.
        - **Empirical Benchmarks**: Leon Heuer, Falk Woldmann Lu, and Jan Haase (FUNARCH
          2026 paper) demonstrating production faultlessness with **zero runtime
          performance penalty** (Criterion benchmarks within +/- 2% noise margin).
    - Strict trait hierarchy separating `ScreenCoordinate` (`u16`) and `StorageCoordinate`
      (`usize`).
    - Strict numeric safety: elimination of raw primitive `as` casting across the core
      framework in favor of `WideningCastTo` and `NarrowingCastTo`.

2. **Groundbreaking Systems Performance & Memory Efficiencies**:
    - **`Flat2DArray` (SIMD-Friendly Contiguous Offscreen Buffer)**: Replaced fragmented
      `Vec<Vec<PixelChar>>` layout with a single contiguous 1D backing store indexed as 2D
      for `OfsBuf`, maximizing CPU cache locality, eliminating pointer indirection, and
      enabling SIMD batch operations.
        - 📖 **Deep Dive Article & YouTube Video**:
          [Build with Naz : High-Performance Flat 2D Arrays in Rust (SIMD, L1 Cache)](https://developerlife.com/2026/07/14/build-high-performance-flat-2d-arrays-in-rust/)
          |
          [YouTube Video Walkthrough (@developerlifecom)](https://www.youtube.com/@developerlifecom)
        - 📊 **Empirical Performance Uplifts (Benchmarked vs `Vec<Vec<T>>`)**:
            - **Compositor Screen Reading / Rendering**: **2.3x speedup** linearly
              streaming flat contiguous memory into L1 D-Cache vs chasing scattered heap
              pointers.
            - **2D Traversal & SIMD (.chunks_exact)**: **1.8x speedup** using pointer
              arithmetic over scalar modulo/division iteration (which stalls CPU execution
              pipelines on variable terminal widths).
            - **Screen Clear**: **1.4x speedup** for complex cell structs via `.fill()`,
              and **up to 60,000x speedup** for primitive types.
            - **Frame-Time Consistency**: Eliminated up to **±98% frame-time
              variance/jitter** caused by scattered heap cache misses, guaranteeing
              smooth, flatline UI rendering.
        - 💾 **Memory Efficiencies**:
            - **Layout & Size Introspection**: **39.0x speedup** calculating memory size
              on `Box<[T]>` vs recursively walking pointer graphs.
            - **Allocation Footprint**: Slashed from `N + 1` separate heap allocations
              (each carrying a 24-byte header) to a **single contiguous allocation** with
              a 16-byte wide pointer.
            - **L1 Cache Line Alignment**: Guarantees sequential 64-byte hardware
              prefetching (`LEVEL1_DCACHE_LINESIZE`), turning the L1 cache into a
              zero-latency prefetch conveyor belt.
    - **`fast_strings` (Zero-Allocation ANSI Formatting)**: Bypasses standard library
      `String` heap allocations and `std::fmt::Formatter` state machine overhead during
      hot-loop ANSI rendering.
    - **Linux Input Performance Fix**: Eliminated `yield_now` spins and optimized
      mio/epoll edge-triggered stdin drain on Linux, drastically slashing CPU usage and
      input latency.
    - **`ZeroCopyGapBuffer`**: Optimized memory allocation strategies for editing large
      files in `edi`.

3. **High-Performance Pure Std-Thread MCP Server**:
    - **`r3bl-rust-analyzer-mcp-server`**: Built with pure Rust standard library threads
      rather than heavy async runtimes to deliver instant AST navigation, hover, and
      compiler diagnostics for AI coding agents.
        - 📖 **Deep Dive Article**:
          [To async or not to async: Building a fast, std-thread Rust MCP server (developerlife.com)](https://developerlife.com/2026/08/22/to-async-or-not-to-async-rust-mcp-server/)

4. **Resilient Terminal Input & Protocol Architecture**:
    - **Linux `Shift+Home` & CSI Recovery**: Decoded modified navigation keys
      (`Shift+Home`, `Ctrl+Home`, `Shift+End`, `Ctrl+End`, `Shift+F1`..`F4`) and
      implemented resilient recovery in `StatefulInputParser::advance` that purges
      unrecognized terminated sequences, preventing permanent input event loop lockups.
    - **`MaybeMore` State Machine & Zero-Latency ESC Disambiguation**: Replaced loose
      booleans with the `MaybeMore` enum (`Drained`, `KernelMayHaveMore`,
      `RemainingInReadBuffer`), enabling 0ms zero-latency ESC handling while correctly
      reassembling multi-packet escape sequences across SSH.
    - **OSC Terminal Query Absorption**: Implemented `OscScanState` and
      `scan_osc_sequence` state machine to scan, frame, and absorb terminal query
      responses (OSC 10/11 color queries, OSC 52 clipboard) on `stdin`, preventing text
      leakage and disambiguating `Alt+]` with zero latency.
    - **Kitty Keyboard Protocol (`CSI u`) Enhancement**: Implemented progressive keyboard
      enhancement negotiation (`\x1b[>1u` / `\x1b[<1u`) to unambiguously resolve
      previously unresolvable key combinations (`Alt+[`, `Shift+Enter`, `Ctrl+Tab`,
      `Alt+Escape`).
    - **Sans-IO Architecture**: Completely decoupled functional byte stream parsing from
      OS I/O, making the parser reusable in tests, `pty_mux`, and recording/playback.

5. **`App` Trait & Architecture Overhaul**:
    - `app_init` renamed to `app_init_components`, `app_start_background_services` added,
      and `app_render` return type changed to `CommonResult`.
    - 10 mode variants extracted from `RenderOpCommon` into `TerminalModeController` on
      `OutputDevice`.
    - `PtySession` made synchronous, `AsyncPtySession` introduced, and `RRTFactory`
      eliminated in favor of ADT const params and resilient state machines.
    - `FullScreenTuiModeGuard` introduced for panic-safe terminal mode restoration.

### Scope of Crates to Release

All workspace crates depending on `r3bl_tui` will be updated to depend on `0.8.0` and
published sequentially:

| Crate                           | Directory                   | Target Version | Current Version | Crate Type                           | Crates.io Dependency Updates |
| :------------------------------ | :-------------------------- | :------------- | :-------------- | :----------------------------------- | :--------------------------- |
| `r3bl_tui`                      | `tui/`                      | **`0.8.0`**    | `0.7.8`         | Library                              | None (Core library)          |
| `r3bl-build-infra`              | `build-infra/`              | **`0.0.6`**    | `0.0.5`         | Binary (`cargo-rustdoc-fmt`)         | `r3bl_tui = "0.8.0"`         |
| `r3bl-cmdr`                     | `cmdr/`                     | **`0.0.27`**   | `0.0.26`        | Binary (`giti`, `edi`, `env-source`) | `r3bl_tui = "0.8.0"`         |
| `r3bl-rust-analyzer-mcp-server` | `rust-analyzer-mcp-server/` | **`1.1.5`**    | `1.1.4`         | Binary (MCP server)                  | `r3bl_tui = "0.8.0"`         |
| `r3bl_analytics_schema`         | `analytics_schema/`         | `0.0.3`        | `0.0.3`         | Library                              | Unchanged (Stable schema)    |

### Release Notes Architecture (`docs/release-notes/<crate>/`)

Release notes will be maintained as standalone Markdown documents in
`docs/release-notes/<crate>/vX.Y.Z.md` (no redundant `README.md` index inside
`docs/release-notes/`).

Every release note will begin with a **standardized discoverability intro section** that
explains what the crate does, its value proposition, installation instructions, and
deep-dive article links for sharing across social channels (Hacker News, Reddit `r/rust`,
LinkedIn).

### Bi-Directional Linking Strategy: `CHANGELOG.md` ⟷ Release Notes

To ensure seamless navigation and zero dead ends for developers reading the repo offline,
in IDEs, or on GitHub:

1. **`CHANGELOG.md` ➔ Release Notes**: Every version header in `CHANGELOG.md` includes an
   explicit callout linking directly to the GitHub Release page:

    ```markdown
    ### v0.8.0 (2026-09-18)

    > 🔗 **Release Notes & Migration Guide**:
    > [v0.8.0-tui](https://github.com/r3bl-org/r3bl-open-core/releases/tag/v0.8.0-tui)
    ```

2. **Release Notes ➔ `CHANGELOG.md`**: Every release note in
   `docs/release-notes/<crate>/vX.Y.Z.md` ends with a cross-reference linking back to the
   granular technical changelog entry (pinned to the release tag):

    ```markdown
    ## 📄 Full Changelog

    - [r3bl_tui v0.8.0 Changelog Entry](https://github.com/r3bl-org/r3bl-open-core/blob/v0.8.0-tui/CHANGELOG.md#v080-2026-09-18)
    ```

## 2. Publication Order DAG

Because crates.io checks that dependencies specified with explicit version requirements
exist on crates.io at publish time:

```text
Step 1: Publish r3bl_tui v0.8.0 (must be live on crates.io first)
           │
           ├───────────────────────────┼───────────────────────────┐
           ▼                           ▼                           ▼
Step 2: r3bl-build-infra v0.0.6   Step 3: r3bl-cmdr v0.0.27   Step 4: r3bl-rust-analyzer-mcp-server v1.1.5
```

## 3. Step-by-Step Implementation Plan

### Overview of Parallel Workstreams

```text
┌────────────────────────────────────────────────────────┐
│ Track A (Parallel - Available Now on main):            │
│ Phase 1: Historical release notes migration            │
│ Phase 2: Pre-draft standalone release notes            │
│ Phase 3: Pre-draft CHANGELOG.md entries                │
└──────────────────────────┬─────────────────────────────┘
                           │
┌──────────────────────────▼─────────────────────────────┐
│ Track B (Sequential - After worktree merge):           │
│ Phase 4: Merge worktree & SSOT documentation sync      │
│ Phase 5: Release r3bl_tui v0.8.0                       │
│ Phase 6: Release r3bl-build-infra v0.0.6               │
│ Phase 7: Release r3bl-cmdr v0.0.27                     │
│ Phase 8: Release r3bl-rust-analyzer-mcp-server v1.1.5  │
│ Phase 9: Community visibility & housekeeping           │
└────────────────────────────────────────────────────────┘
```

## Track A: Pre-Release Documentation, Release Notes & Changelogs (In Parallel Now)

_This track can be executed immediately on `main` while work on
`../roc-fix-shift-home-lockup` continues in parallel._

### Phase 1: Historical Release Notes Migration

- [x] Create `docs/release-notes/` directories:
    ```bash
    mkdir -p docs/release-notes/r3bl_tui
    mkdir -p docs/release-notes/r3bl-cmdr
    mkdir -p docs/release-notes/r3bl-build-infra
    mkdir -p docs/release-notes/r3bl-rust-analyzer-mcp-server
    ```
- [x] Retrieve all 18 historical GitHub releases via `gh release` and save to
      `docs/release-notes/<crate>/<version>.md`:
    - Note on directory naming: `docs/release-notes/<crate>/` uses full crate package
      names matching top-level sections in `CHANGELOG.md` (`r3bl_tui`, `r3bl-cmdr`,
      `r3bl-build-infra`, `r3bl-rust-analyzer-mcp-server`).
    - Use the following command to get the markdown content for each tag and save it to
      the appropriate file:
      `gh release view <tag> --json body --jq .body > docs/release-notes/<crate>/<version>.md`
    - Historical release mapping:
        - `r3bl_tui` (7 releases): `v0.7.2-tui` ->
          `docs/release-notes/r3bl_tui/v0.7.2.md`, `v0.7.3-tui` -> `v0.7.3.md`,
          `v0.7.4-tui` -> `v0.7.4.md`, `v0.7.5-tui` -> `v0.7.5.md`, `v0.7.6-tui` ->
          `v0.7.6.md`, `v0.7.7-tui` -> `v0.7.7.md`, `v0.7.8-tui` -> `v0.7.8.md`
        - `r3bl-cmdr` (7 releases): `v0.0.20-cmdr` ->
          `docs/release-notes/r3bl-cmdr/v0.0.20.md`, `v0.0.21-cmdr` -> `v0.0.21.md`,
          `v0.0.22-cmdr` -> `v0.0.22.md`, `v0.0.23-cmdr` -> `v0.0.23.md`, `v0.0.24-cmdr`
          -> `v0.0.24.md`, `v0.0.25-cmdr` -> `v0.0.25.md`, `v0.0.26-cmdr` -> `v0.0.26.md`
        - `r3bl-build-infra` (4 releases): `v0.0.1-build-infra` ->
          `docs/release-notes/r3bl-build-infra/v0.0.1.md`, `v0.0.2-build-infra` ->
          `v0.0.2.md`, `v0.0.4-build-infra` -> `v0.0.4.md`, `v0.0.5-build-infra` ->
          `v0.0.5.md`
        - _Note on `r3bl-rust-analyzer-mcp-server`_: It has 0 historical releases in this
          repository (it originated in a separate repository and was consolidated later).
          Since Git does not track empty directories, its folder will be tracked once
          `v1.1.5.md` is created in Phase 2.
- [x] Audit untracked files via `git status` to ensure all 18 files were populated.
- [x] **Mandatory manual review:** Verify historical release notes are populated cleanly
      in `docs/release-notes/`.
    - [x] `docs/release-notes/r3bl_tui/` (7 files)
    - [x] `docs/release-notes/r3bl-cmdr/` (7 files)
    - [x] `docs/release-notes/r3bl-build-infra/` (4 files)
- [x] Commit historical release notes explicitly:
    ```bash
    git add docs/release-notes/
    git commit -m "[docs] Backfill historical release notes
    ```

Task: make-0.8.0-release.md" ```

### Phase 2: Pre-Draft Standalone Release Notes & Migration Guides

- [x] Draft Release Notes in `docs/release-notes/r3bl_tui/v0.8.0.md`:
    - Include standardized **Discoverability Intro**:

        ````markdown
        > **r3bl_tui** is a fully async, immediate-mode TUI framework for Rust inspired by
        > React, Elm, and web technologies. It features flexbox layouts, CSS-like styling,
        > reactive state architecture, a custom Markdown renderer with syntax
        > highlighting, gradient colors, emoji/grapheme clustering, modal dialogs, mouse
        > support, async non-blocking readline, and diff-based rendering optimized for
        > SSH. Built with zero-copy gap buffers, SIMD-friendly offscreen buffers
        > (`Flat2DArray`), zero-allocation ANSI string generation, and VT100 PTY
        > multiplexing primitives.

        Add to `Cargo.toml`:

        ```toml
        [dependencies]
        r3bl_tui = "0.8.0"
        ```
        ````

        Read the architectural deep dive:
        [Build with Naz : High-Performance Flat 2D Arrays in Rust (SIMD, L1 Cache)](https://developerlife.com/2026/07/14/build-high-performance-flat-2d-arrays-in-rust/)

        ```

        ```

    - Include the comprehensive **Migration Guide** (Before vs After code snippets for
      `VPPos`/`VPSize`, `App` trait, `RenderOpCommon`, and PTY sessions).
    - Include highlights: Mathematically & Empirically Validated Type Safety,
      `Flat2DArray`, `fast_strings`, `MaybeMore` 0ms ESC handling, OSC query absorption,
      Kitty Keyboard Protocol `CSI u`.
    - Include benchmark stats (2.3x rendering, 1.8x traversal, 39x memory size
      introspection, +-98% jitter elimination).
    - Link to tag-pinned `CHANGELOG.md` anchor
      (`https://github.com/r3bl-org/r3bl-open-core/blob/v0.8.0-tui/CHANGELOG.md#v080-2026-09-18`).
      _(Note: Update the date `2026-09-18` in the anchor and header to match the actual
      date of publication if different)._

- [x] Draft Release Notes in `docs/release-notes/r3bl-build-infra/v0.0.6.md`:
    - Include standardized **Discoverability Intro**:

        ```markdown
        > **r3bl-build-infra** provides developer tools and utilities for Rust projects
        > and documentation automation.
        >
        > - 📐 **`cargo-rustdoc-fmt`**: CLI tool that formats Markdown tables and
        >   automatically converts inline code references into clean reference-style
        >   intra-doc links with technical term linking.

        Install with: `cargo install r3bl-build-infra --force`
        ```

    - Include changelog highlights for `cargo-rustdoc-fmt` and dependency bump.
    - Link to tag-pinned `CHANGELOG.md` anchor
      (`https://github.com/r3bl-org/r3bl-open-core/blob/v0.0.6-build-infra/CHANGELOG.md#v006-2026-09-18`).

- [x] Draft Release Notes in `docs/release-notes/r3bl-cmdr/v0.0.27.md`:
    - Include standardized **Discoverability Intro**:

        ```markdown
        > **r3bl-cmdr** is a suite of fast, fully async TUI & CLI developer productivity
        > tools built on `r3bl_tui`.
        >
        > - 😺 **`giti`**: Interactive Git CLI with visual branch selection and
        >   streamlined commit workflows.
        > - 🦜 **`edi`**: Terminal Markdown editor featuring syntax highlighting, gradient
        >   colors, emoji support, SSH-optimized diff-rendering, and a high-performance
        >   zero-copy gap buffer.
        > - 📜 **`env-source`**: Blazing fast cross-platform environment loader. Direct
        >   100x faster Rust replacement for Fish [`bass`] (`bass.py`) on Unix and
        >   seamless `.bat` environment loader for PowerShell on Windows.

        Install with: `cargo install r3bl-cmdr --force`
        ```

    - Highlight new tool: `env-source`.
    - Highlight fixes and performance boosts in `edi` and `giti`.
    - Link to tag-pinned `CHANGELOG.md` anchor
      (`https://github.com/r3bl-org/r3bl-open-core/blob/v0.0.27-cmdr/CHANGELOG.md#v0027-2026-09-18`).

- [x] Draft Release Notes in `docs/release-notes/r3bl-rust-analyzer-mcp-server/v1.1.5.md`:
    - Include standardized **Discoverability Intro**:

        ```markdown
        > **r3bl-rust-analyzer-mcp-server** is a high-performance Model Context Protocol
        > (MCP) server for `rust-analyzer`. Built with pure Rust standard library threads
        > (no async runtime overhead) to provide lightning-fast AST code navigation, type
        > hover, definition lookup, code actions, and compiler diagnostics directly to AI
        > coding agents (Claude, Antigravity, Cursor, etc.).

        Read the architectural deep dive:
        [To async or not to async: Building a fast, std-thread Rust MCP server (developerlife.com)](https://developerlife.com/2026/08/22/to-async-or-not-to-async-rust-mcp-server/)

        Install with: `cargo install r3bl-rust-analyzer-mcp-server --force`
        ```

    - Link to tag-pinned `CHANGELOG.md` anchor
      (`https://github.com/r3bl-org/r3bl-open-core/blob/v1.1.5-rust-analyzer-mcp-server/CHANGELOG.md#v115-2026-09-18`).

- [x] Audit diffs via `git diff docs/release-notes/` (or `git status`) to verify clean
      markdown generation.

- [x] **Mandatory manual review:** Verify all 4 drafted release note files and aligned
      core documentation.
    - [x] `docs/release-notes/r3bl_tui/v0.8.0.md`
    - [x] `docs/release-notes/r3bl-build-infra/v0.0.6.md`
    - [x] `docs/release-notes/r3bl-cmdr/v0.0.27.md`
    - [x] `docs/release-notes/r3bl-rust-analyzer-mcp-server/v1.1.5.md`
    - [x] `README.md`
    - [x] `tui/src/lib.rs`
    - [x] `tui/README.md`
    - [x] `tui/Cargo.toml`
    - [x] `tui/src/readline_async/mod.rs`
    - [x] `.vscode/settings.json`

### Phase 3: Pre-Draft `CHANGELOG.md` Entries

- [x] Update `CHANGELOG.md` for `r3bl_tui`:
    - Add section `### v0.8.0 (2026-09-18)` directly under `## r3bl_tui`.
    - Add GitHub release direct link callout:
      `> 🔗 **Release Notes & Migration Guide**: [v0.8.0-tui](https://github.com/r3bl-org/r3bl-open-core/releases/tag/v0.8.0-tui)`
    - Document **Type Safety at Scale**:
        - Grounded in programming language research: Will Crichton (Stanford CS 242,
          FUNARCH 2023) and Alexis King ("Parse, don't validate").
        - Empirical validation: Leon Heuer, Falk Woldmann Lu, and Jan Haase (FUNARCH 2026)
          demonstrating zero runtime overhead (+/- 2% noise margin on Criterion).
        - Separation of `Canvas` (`CPos`, `CCaret`, `CCol`, `CRow`, `CWidth`, `CHeight`)
          and `Viewport` (`VPPos`, `VPCaret`, `VPCol`, `VPRow`, `VPWidth`, `VPHeight`)
          coordinate spaces.
        - Strict `ScreenCoordinate` (`u16`) vs `StorageCoordinate` (`usize`) trait
          hierarchy.
        - Complete elimination of raw `as` casts in favor of `WideningCastTo` and
          `NarrowingCastTo`.
    - Document **Breaking Changes**:
        - Renamed `Pos`/`Size` to `VPPos`/`VPSize` and
          `RowIndex`/`ColIndex`/`RowHeight`/`ColWidth` to
          `VPRow`/`VPCol`/`VPHeight`/`VPWidth`.
        - Renamed `App::app_init` to `App::app_init_components`.
        - Changed `App::app_render` signature to return `CommonResult`.
        - Added `App::app_start_background_services`.
        - Removed 10 mode variants from `RenderOpCommon` (moved to
          `TerminalModeController`).
        - Made `PtySession` synchronous and added `AsyncPtySession`.
        - Replaced `copypasta-ext` with `copypasta`; removed default `emacs` feature.
    - Document **Added**:
        - Complete VT100 PTY multiplexer (`pty_mux`) with mouse forwarding, alt-screen,
          and DA1 response generation.
        - `Flat2DArray` SIMD-friendly 2D-indexed contiguous backing store for `OfsBuf`
          (see
          [Build with Naz : High-Performance Flat 2D Arrays in Rust (SIMD, L1 Cache)](https://developerlife.com/2026/07/14/build-high-performance-flat-2d-arrays-in-rust/)).
        - `fast_strings` zero-allocation ANSI escape sequence generation engine.
        - `MaybeMore` stream availability state machine for 0ms zero-latency ESC
          disambiguation.
        - `OscScanState` and `scan_osc_sequence` state machine parsing and absorbing
          terminal query replies (OSC 10/11 color queries, OSC 52 clipboard).
        - Kitty Keyboard Protocol progressive enhancement (`CSI u`) resolving `Alt+[`,
          `Shift+Enter`, `Ctrl+Tab`, and `Alt+Escape`.
        - Sans-IO functional byte stream parser architecture.
        - `FullScreenTuiModeGuard` for panic-safe terminal mode restoration.
    - Document **Fixed**:
        - Linux `Shift+Home`, `Ctrl+Home`, `Shift+End`, and unrecognized CSI input freeze
          fix.
        - Linux epoll edge-triggered stdin drain and `yield_now` slowdown fix.
        - `Readline` deadlock fix via strict lock hierarchy.
        - Deadlock-free `PtyPair` orchestration and watchdog timeouts.
        - Windows ConPTY EOF and terminal restoration fixes.
    - Document **Performance & Memory Efficiencies**:
        - `Flat2DArray` contiguous memory layout eliminating pointer vector layout
          bottlenecks
          ([Build with Naz : High-Performance Flat 2D Arrays in Rust (SIMD, L1 Cache)](https://developerlife.com/2026/07/14/build-high-performance-flat-2d-arrays-in-rust/)).
        - **2.3x rendering speedup**: linearly streaming contiguous bytes into L1 cache
          for screen compositing.
        - **1.8x traversal speedup**: `.chunks_exact(cols)` eliminating CPU
          division/modulo pipeline stalls.
        - **1.4x to 60,000x screen clear speedup**: contiguous SIMD `.fill()` operations.
        - **39.0x memory size introspection speedup**: zero-indirection `Box<[T]>` memory
          layout.
        - **Jitter elimination**: eradicated up to +-98% frame-time variance.
        - Zero-allocation string building throughout ANSI generation (`fast_strings`).
        - Initial memory allocation and capacity optimizations for `ZeroCopyGapBuffer`.

- [x] Update `CHANGELOG.md` for `r3bl-build-infra`:
    - Add section `### v0.0.6 (2026-09-18)`.
    - Add GitHub release direct link callout:
      `> 🔗 **Release Notes**: [v0.0.6-build-infra](https://github.com/r3bl-org/r3bl-open-core/releases/tag/v0.0.6-build-infra)`
    - Document technical term auto-linking additions in `cargo-rustdoc-fmt`.
    - Document dependency bump to `r3bl_tui 0.8.0`.

- [x] Update `CHANGELOG.md` for `r3bl-cmdr`:
    - Add section `### v0.0.27 (2026-09-18)`.
    - Add GitHub release direct link callout:
      `> 🔗 **Release Notes**: [v0.0.27-cmdr](https://github.com/r3bl-org/r3bl-open-core/releases/tag/v0.0.27-cmdr)`
    - Document `env-source` cross-platform environment script evaluator.
    - Document `edi` and `giti` migration to `r3bl_tui 0.8.0` with `Flat2DArray`,
      `ZeroCopyGapBuffer`, and new coordinate types.
    - Document Windows ConPTY EOF fix.

- [x] Update `CHANGELOG.md` for `r3bl-rust-analyzer-mcp-server`:
    - Add section `### v1.1.5 (2026-09-18)`.
    - Add GitHub release direct link callout:
      `> 🔗 **Release Notes**: [v1.1.5-rust-analyzer-mcp-server](https://github.com/r3bl-org/r3bl-open-core/releases/tag/v1.1.5-rust-analyzer-mcp-server)`
    - Document dependency bump to `r3bl_tui 0.8.0`.

- [x] Update `CHANGELOG.md` Table of Contents at the top of the file:
    - Replace the legacy `doctoc` comment block (`<!-- START doctoc generated TOC ... -->`
      through `<!-- END doctoc generated TOC ... -->`) at the top of `CHANGELOG.md` with:
        ```markdown
        <!-- BEGIN mktoc -->
        <!-- END mktoc -->
        ```
    - Run `mktoc CHANGELOG.md` to populate the new TOC between those markers.

- [x] Audit diffs via `git diff CHANGELOG.md` to ensure only the new sections and TOC were
      updated.

- [x] **Mandatory manual review:** Verify `CHANGELOG.md` entries and TOC.
    - [x] `CHANGELOG.md` entries for all 4 crates
    - [x] `mktoc` TOC update

- [x] Commit all Track A documentation changes explicitly and push to remote:

    ```bash
    git add docs/release-notes/ README.md tui/src/lib.rs tui/README.md tui/Cargo.toml tui/src/readline_async/mod.rs .vscode/settings.json CHANGELOG.md task/make-0.8.0-release.md
    git commit -m "[docs] Pre-draft v0.8.0 release notes, CHANGELOG, and update core docs

    Task: make-0.8.0-release.md"
    git push origin main
    ```

## Track B: Release Execution (Post-Merge of `fix-shift-home-lockup`)

_Execute this track once the worktree branch `fix-shift-home-lockup` is fully completed
and ready to merge._

### Phase 4: Workspace Merge & Documentation SSOT Sync

- [ ] Merge worktree branch `fix-shift-home-lockup` (PR #490) into `main` and clean up
      worktree:
    - Verify all checks pass on PR #490 and merge via rebase:
        ```bash
        gh pr merge 490 --rebase --delete-branch
        ```
    - Clean up the local worktree and branch:
        ```bash
        git worktree remove ../roc-fix-shift-home-lockup
        git branch -d fix-shift-home-lockup
        git fetch --prune
        ```
    - On `main` branch, pull latest rebased commits:
        ```bash
        git checkout main
        git pull origin main
        git status
        ```
- [ ] Update `tui/src/lib.rs` (Single Source of Truth under `//! # Why R3BL?`):
    - Add subsection `//! ## Mathematically & Empirically Validated Type Safety` detailing
      FUNARCH 2023 & 2026 academic research, typestate coordinate boundaries, and zero
      runtime penalty (using `//! ##` so rustdoc navigation shows it cleanly and
      `cargo readme` converts it to `###` in `README.md`).
    - Add subsection `//! ## High-Performance Systems Architecture` detailing
      `Flat2DArray` SIMD contiguous memory layout, `fast_strings` zero-alloc ANSI
      formatting, and non-blocking epoll event loops, linking to
      [Build with Naz : High-Performance Flat 2D Arrays in Rust (SIMD, L1 Cache)](https://developerlife.com/2026/07/14/build-high-performance-flat-2d-arrays-in-rust/).
- [ ] Generate `tui/README.md` from `tui/src/lib.rs`:
    ```bash
    cd tui && cargo readme > README.md && cd ..
    ```
- [ ] Update root `README.md`:
    - Add subsection `### Mathematically & Empirically Validated Type Safety` under
      `## Why R3BL TUI?`.
    - Add subsection `### High-Performance Systems Architecture` under `## Why R3BL TUI?`.
    - Update workspace crates overview to explicitly feature
      `r3bl-rust-analyzer-mcp-server` (linking to
      [To async or not to async](https://developerlife.com/2026/08/22/to-async-or-not-to-async-rust-mcp-server/))
      and `env-source`.
    - Run `mktoc` on root `README.md`.
- [ ] Synchronize `r3bl_tui` dependency requirements across workspace `Cargo.toml` files
      (keep dependent crate package versions at current versions until their respective
      release phases):
    - In `tui/Cargo.toml`: Set `version = "0.8.0"`.
    - In `build-infra/Cargo.toml`: Keep package `version = "0.0.5"`, update dependency
      `r3bl_tui = { path = "../tui", version = "0.8.0" }`.
    - In `cmdr/Cargo.toml`: Keep package `version = "0.0.26"`, update dependency
      `r3bl_tui = { path = "../tui", version = "0.8.0" }`.
    - In `rust-analyzer-mcp-server/Cargo.toml`: Keep package `version = "1.1.4"`, update
      dependency `r3bl_tui = { path = "../tui", version = "0.8.0" }`.
- [ ] Update `docs/release-guide.md`:
    - Add a dedicated release workflow script block for `r3bl-rust-analyzer-mcp-server`
      under `Full workflow`.
    - Add `rust-analyzer-mcp-server` to the canonical examples table.
    - Update version numbers and tag templates across the bash script examples for all
      crates.
- [ ] Run full workspace verification & cross-platform checks:
    ```bash
    cargo update --workspace
    ./check.fish --fmt
    ./check.fish --clippy
    ./check.fish --quick-doc
    ./check.fish --test
    ./check.fish --full
    ```
- [ ] Run cross-platform verification via `/test-cross-platform` on macOS and Windows
      fleet.
- [ ] Audit diffs line-by-line via `git diff` across all modified files to ensure zero
      collateral changes.
- [ ] **Mandatory manual review:** Verify branch state, `tui/src/lib.rs`, `tui/README.md`,
      root `README.md`, all `Cargo.toml` updates, and cross-platform check results.
    - [ ] `tui/src/lib.rs` and `tui/README.md`
    - [ ] Root `README.md`
    - [ ] All 4 crate `Cargo.toml` files
    - [ ] `docs/release-guide.md`
    - [ ] `./check.fish --full` and cross-platform tests pass cleanly

### Phase 5: Release `r3bl_tui` v0.8.0 (Core Library)

- [ ] Perform dry-run publication:
    ```bash
    cd tui && cargo publish --dry-run --allow-dirty --no-verify && cd ..
    ```
- [ ] Audit diff via `git diff tui/` to ensure dry run did not generate unwanted
      artifacts.
- [ ] **Mandatory manual review:** Verify `tui` dry-run succeeds cleanly.
- [ ] **User Permission Checkpoint**: Request user confirmation before publishing
      `r3bl_tui` to crates.io.
- [ ] Create git commit and tag for `r3bl_tui` using explicit staging (stages `tui`,
      workspace manifests, and internal dependency updates):
    ```bash
    git add tui/ README.md Cargo.lock Cargo.toml build-infra/Cargo.toml cmdr/Cargo.toml rust-analyzer-mcp-server/Cargo.toml docs/release-guide.md
    git commit -m "v0.8.0-tui"
    git tag -a v0.8.0-tui -m "v0.8.0-tui"
    ```
- [ ] Publish `r3bl_tui` to crates.io:
    ```bash
    cd tui && cargo publish --no-verify --allow-dirty && cd ..
    ```
- [ ] Verify `r3bl_tui` 0.8.0 is live on crates.io:
    - Query `cargo search r3bl_tui` or
      `curl -s https://crates.io/api/v1/crates/r3bl_tui | grep '"max_version":"0.8.0"'`.
    - Note: crates.io sparse index caching can take a few minutes to reflect locally;
      running `cargo publish --dry-run` on downstream `build-infra` is a foolproof
      verification check.
- [ ] Push commit and tag to remote:
    ```bash
    git push origin main && git push origin v0.8.0-tui
    ```
- [ ] Create GitHub Release for `v0.8.0-tui`:
    ```bash
    gh release create v0.8.0-tui --title "v0.8.0-tui" --notes-file docs/release-notes/r3bl_tui/v0.8.0.md
    ```
- [ ] Audit `git status` to verify working tree is clean.
- [ ] **Mandatory manual review:** Verify `r3bl_tui 0.8.0` is live on crates.io and GitHub
      release page is created.
    - [ ] crates.io: `https://crates.io/crates/r3bl_tui` shows `0.8.0`
    - [ ] GitHub release:
          `https://github.com/r3bl-org/r3bl-open-core/releases/tag/v0.8.0-tui`

### Phase 6: Release `r3bl-build-infra` v0.0.6 (Tooling Crate)

- [ ] Bump package version in `build-infra/Cargo.toml`: Set `version = "0.0.6"`.
- [ ] Generate README from SSOT `build-infra/src/lib.rs`:
    ```bash
    cd build-infra && cargo readme > README.md && cd ..
    ```
- [ ] Perform dry-run publication:
    ```bash
    cd build-infra && cargo publish --dry-run --allow-dirty --no-verify && cd ..
    ```
- [ ] Audit diff via `git diff build-infra/` to verify version bump and README generation.
- [ ] **Mandatory manual review:** Verify `build-infra` dry-run succeeds cleanly.
- [ ] **User Permission Checkpoint**: Request user confirmation before publishing
      `r3bl-build-infra` to crates.io.
- [ ] Create git commit and tag for `r3bl-build-infra` using explicit staging:
    ```bash
    git add build-infra/ Cargo.lock
    git commit -m "v0.0.6-build-infra"
    git tag -a v0.0.6-build-infra -m "v0.0.6-build-infra"
    ```
- [ ] Publish `r3bl-build-infra` to crates.io:
    ```bash
    cd build-infra && cargo publish --no-verify --allow-dirty && cd ..
    ```
- [ ] Push commit and tag to remote:
    ```bash
    git push origin main && git push origin v0.0.6-build-infra
    ```
- [ ] Create GitHub Release for `v0.0.6-build-infra`:
    ```bash
    gh release create v0.0.6-build-infra --title "v0.0.6-build-infra" --notes-file docs/release-notes/r3bl-build-infra/v0.0.6.md
    ```
- [ ] Re-install binary locally:
    ```bash
    cargo install --path build-infra --force
    ```
- [ ] Audit `git status` to verify clean working tree.
- [ ] **Mandatory manual review:** Verify `r3bl-build-infra 0.0.6` is live on crates.io
      and GitHub release page is created.
    - [ ] crates.io: `https://crates.io/crates/r3bl-build-infra` shows `0.0.6`
    - [ ] GitHub release:
          `https://github.com/r3bl-org/r3bl-open-core/releases/tag/v0.0.6-build-infra`

### Phase 7: Release `r3bl-cmdr` v0.0.27 (CLI & Apps Crate)

- [ ] Bump package version in `cmdr/Cargo.toml`: Set `version = "0.0.27"`.
- [ ] Generate README from SSOT `cmdr/src/lib.rs`:
    ```bash
    cd cmdr && cargo readme > README.md && cd ..
    ```
- [ ] Perform dry-run publication:
    ```bash
    cd cmdr && cargo publish --dry-run --allow-dirty --no-verify && cd ..
    ```
- [ ] Audit diff via `git diff cmdr/` to verify version bump and README generation.
- [ ] **Mandatory manual review:** Verify `cmdr` dry-run succeeds cleanly.
- [ ] **User Permission Checkpoint**: Request user confirmation before publishing
      `r3bl-cmdr` to crates.io.
- [ ] Create git commit and tag for `r3bl-cmdr` using explicit staging:
    ```bash
    git add cmdr/ Cargo.lock
    git commit -m "v0.0.27-cmdr"
    git tag -a v0.0.27-cmdr -m "v0.0.27-cmdr"
    ```
- [ ] Publish `r3bl-cmdr` to crates.io:
    ```bash
    cd cmdr && cargo publish --no-verify --allow-dirty && cd ..
    ```
- [ ] Push commit and tag to remote:
    ```bash
    git push origin main && git push origin v0.0.27-cmdr
    ```
- [ ] Create GitHub Release for `v0.0.27-cmdr`:
    ```bash
    gh release create v0.0.27-cmdr --title "v0.0.27-cmdr" --notes-file docs/release-notes/r3bl-cmdr/v0.0.27.md
    ```
- [ ] Re-install binary locally:
    ```bash
    cargo install --path cmdr --force
    ```
- [ ] Audit `git status` to verify clean working tree.
- [ ] **Mandatory manual review:** Verify `r3bl-cmdr 0.0.27` is live on crates.io and
      GitHub release page is created.
    - [ ] crates.io: `https://crates.io/crates/r3bl-cmdr` shows `0.0.27`
    - [ ] GitHub release:
          `https://github.com/r3bl-org/r3bl-open-core/releases/tag/v0.0.27-cmdr`

### Phase 8: Release `r3bl-rust-analyzer-mcp-server` v1.1.5 (MCP Server)

- [ ] Bump package version in `rust-analyzer-mcp-server/Cargo.toml`: Set
      `version = "1.1.5"`.
- [ ] _Do NOT run `cargo readme`_: `rust-analyzer-mcp-server/README.md` is hand-crafted
      and maintained directly as the crates.io landing page.
- [ ] Perform dry-run publication:
    ```bash
    cd rust-analyzer-mcp-server && cargo publish --dry-run --allow-dirty --no-verify && cd ..
    ```
- [ ] Audit diff via `git diff rust-analyzer-mcp-server/` to verify version bump.
- [ ] **Mandatory manual review:** Verify `rust-analyzer-mcp-server` dry-run succeeds
      cleanly.
- [ ] **User Permission Checkpoint**: Request user confirmation before publishing
      `r3bl-rust-analyzer-mcp-server` to crates.io.
- [ ] Create git commit and tag for `r3bl-rust-analyzer-mcp-server` using explicit
      staging:
    ```bash
    git add rust-analyzer-mcp-server/ Cargo.lock
    git commit -m "v1.1.5-rust-analyzer-mcp-server"
    git tag -a v1.1.5-rust-analyzer-mcp-server -m "v1.1.5-rust-analyzer-mcp-server"
    ```
- [ ] Publish `r3bl-rust-analyzer-mcp-server` to crates.io:
    ```bash
    cd rust-analyzer-mcp-server && cargo publish --no-verify --allow-dirty && cd ..
    ```
- [ ] Push commit and tag to remote:
    ```bash
    git push origin main && git push origin v1.1.5-rust-analyzer-mcp-server
    ```
- [ ] Create GitHub Release for `v1.1.5-rust-analyzer-mcp-server`:
    ```bash
    gh release create v1.1.5-rust-analyzer-mcp-server --title "v1.1.5-rust-analyzer-mcp-server" --notes-file docs/release-notes/r3bl-rust-analyzer-mcp-server/v1.1.5.md
    ```
- [ ] Re-install binary locally:
    ```bash
    cargo install --path rust-analyzer-mcp-server --force
    ```
- [ ] Audit `git status` to verify clean working tree.
- [ ] **Mandatory manual review:** Verify `r3bl-rust-analyzer-mcp-server 1.1.5` is live on
      crates.io and GitHub release page is created.
    - [ ] crates.io: `https://crates.io/crates/r3bl-rust-analyzer-mcp-server` shows
          `1.1.5`
    - [ ] GitHub release:
          `https://github.com/r3bl-org/r3bl-open-core/releases/tag/v1.1.5-rust-analyzer-mcp-server`

### Phase 9: Community Visibility & Housekeeping

- [ ] Share release announcements across developer communities following
      `docs/release-guide.md`:
    - [ ] **Reddit (`r/rust`)**: Post
          `[Release] r3bl_tui v0.8.0: Async TUI library with 2D Canvas/Viewport coords, Flat2DArray SIMD layout, and 0ms ESC handling`
          with body from `docs/release-notes/r3bl_tui/v0.8.0.md`.
    - [ ] **Hacker News (Show HN)**: Post
          `Show HN: R3BL TUI v0.8.0 - Async Rust TUI with Flat2DArray SIMD layout and zero-cost typestate safety`
          linking to GitHub release or developerlife.com article.
    - [ ] **LinkedIn**: Post highlights of the 0.8.0 release family (`r3bl_tui`,
          `r3bl-cmdr`, `r3bl-build-infra`, `r3bl-rust-analyzer-mcp-server`) with
          performance metrics and video links.
- [ ] Update `task/prepare-v0.8.0-meta-task.md` to reflect publication completion.
- [ ] Audit diff via `git diff task/` to verify clean metadata updates.
- [ ] **Mandatory manual review:** Verify all release tracking tasks are marked complete.
    - [ ] `task/prepare-v0.8.0-meta-task.md`
    - [ ] `task/make-0.8.0-release.md`

<!-- cspell:words Falk Heuer Woldmann Haase developerlifecom DCACHE LINESIZE Workstreams SSOT -->
