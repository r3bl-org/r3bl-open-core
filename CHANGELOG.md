<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Changelog](#changelog)
  - [Rust Development Power Tools](#rust-development-power-tools)
    - [Rust Development Power Tools (2026-01-23)](#rust-development-power-tools-2026-01-23)
    - [Rust Development Power Tools (2025-08-04)](#rust-development-power-tools-2025-08-04)
    - [Rust Development Power Tools (2025-07-22)](#rust-development-power-tools-2025-07-22)
    - [Rust Development Power Tools (2025-04-21)](#rust-development-power-tools-2025-04-21)
    - [Rust Development Power Tools (2025-03-24)](#rust-development-power-tools-2025-03-24)
    - [Rust Development Power Tools (2025-03-19)](#rust-development-power-tools-2025-03-19)
    - [Rust Development Power Tools (2024-12-04)](#rust-development-power-tools-2024-12-04)
  - [`r3bl_tui`](#r3bl_tui)
    - [v0.7.8 (2026-01-23)](#v078-2026-01-23)
    - [v0.7.7 (2026-01-23)](#v077-2026-01-23)
    - [v0.7.6 (2025-08-16)](#v076-2025-08-16)
    - [v0.7.5 (2025-08-15)](#v075-2025-08-15)
    - [v0.7.4 (2025-08-15)](#v074-2025-08-15)
    - [v0.7.3 (2025-08-04)](#v073-2025-08-04)
    - [v0.7.2 (2025-07-23)](#v072-2025-07-23)
    - [v0.7.1 (2025-05-10)](#v071-2025-05-10)
    - [v0.7.0 (2025-05-10)](#v070-2025-05-10)
    - [v0.6.0 (2024-10-21)](#v060-2024-10-21)
    - [v0.5.9 (2024-09-12)](#v059-2024-09-12)
    - [v0.5.8 (2024-09-07)](#v058-2024-09-07)
    - [v0.5.7 (2024-08-13)](#v057-2024-08-13)
    - [v0.5.6 (2024-06-29)](#v056-2024-06-29)
    - [v0.5.5 (2024-05-20)](#v055-2024-05-20)
    - [v0.5.4 (2024-05-20)](#v054-2024-05-20)
    - [v0.5.3 (2024-04-15)](#v053-2024-04-15)
    - [v0.5.2 (2024-01-14)](#v052-2024-01-14)
    - [v0.5.1 (2024-01-09)](#v051-2024-01-09)
    - [v0.5.0 (2023-12-31)](#v050-2023-12-31)
    - [v0.4.0 (2023-12-22)](#v040-2023-12-22)
    - [v0.3.10 (2023-10-29)](#v0310-2023-10-29)
    - [v0.3.9 (2023-10-29)](#v039-2023-10-29)
    - [v0.3.7 (2023-10-21)](#v037-2023-10-21)
    - [v0.3.6 (2023-10-17)](#v036-2023-10-17)
    - [v0.3.5 (2023-10-14)](#v035-2023-10-14)
    - [v0.3.3 (2023-04-20)](#v033-2023-04-20)
    - [v0.3.2 (2023-03-06)](#v032-2023-03-06)
    - [v0.3.1 (2023-03-06)](#v031-2023-03-06)
  - [`r3bl-cmdr`](#r3bl-cmdr)
    - [v0.0.26 (2026-01-23)](#v0026-2026-01-23)
    - [v0.0.25 (2026-01-23)](#v0025-2026-01-23)
    - [v0.0.24 (2025-08-16)](#v0024-2025-08-16)
    - [v0.0.23 (2025-08-15)](#v0023-2025-08-15)
    - [v0.0.22 (2025-08-15)](#v0022-2025-08-15)
    - [v0.0.21 (2025-08-04)](#v0021-2025-08-04)
    - [v0.0.20 (2025-07-23)](#v0020-2025-07-23)
    - [v0.0.19 (2025-05-10)](#v0019-2025-05-10)
    - [v0.0.18 (2025-05-10)](#v0018-2025-05-10)
    - [v0.0.17 (2025-05-10)](#v0017-2025-05-10)
    - [v0.0.16 (2024-09-13)](#v0016-2024-09-13)
    - [v0.0.15 (2024-09-12)](#v0015-2024-09-12)
    - [v0.0.14 (2024-06-29)](#v0014-2024-06-29)
    - [v0.0.13 (2024-05-20)](#v0013-2024-05-20)
    - [v0.0.12 (2024-05-12)](#v0012-2024-05-12)
    - [v0.0.11 (2024-01-14)](#v0011-2024-01-14)
    - [v0.0.10 (2024-01-02)](#v0010-2024-01-02)
    - [v0.0.9 (2023-12-31)](#v009-2023-12-31)
    - [v0.0.8 (2023-12-22)](#v008-2023-12-22)
  - [`r3bl-build-infra`](#r3bl-build-infra)
    - [v0.0.1 (2026-01-23)](#v001-2026-01-23)
  - [`r3bl_analytics_schema`](#r3bl_analytics_schema)
    - [v0.0.3 (2025-05-10)](#v003-2025-05-10)
    - [v0.0.2 (2024-09-12)](#v002-2024-09-12)
    - [v0.0.1 (2023-12-31)](#v001-2023-12-31)
- [Archived](#archived)
  - [`r3bl_rs_utils_macro`](#r3bl_rs_utils_macro)
    - [Archived (formerly renamed to `r3bl_macro`)](#archived-formerly-renamed-to-r3bl_macro)
    - [v0.9.10 (2024-09-12)](#v0910-2024-09-12)
    - [v0.9.9 (2024-04-16)](#v099-2024-04-16)
    - [v0.9.8 (2023-12-22)](#v098-2023-12-22)
    - [v0.9.7 (2023-10-21)](#v097-2023-10-21)
    - [v0.9.6 (2023-10-17)](#v096-2023-10-17)
    - [v0.9.5 (2023-10-14)](#v095-2023-10-14)
  - [`r3bl_rs_utils_core`](#r3bl_rs_utils_core)
    - [Archived (formerly renamed to `r3bl_core`)](#archived-formerly-renamed-to-r3bl_core)
    - [v0.9.16 (2024-09-12)](#v0916-2024-09-12)
    - [v0.9.15 (2024-09-07)](#v0915-2024-09-07)
    - [v0.9.14 (2024-08-13)](#v0914-2024-08-13)
    - [v0.9.13 (2024-04-15)](#v0913-2024-04-15)
    - [v0.9.12 (2024-01-07)](#v0912-2024-01-07)
    - [v0.9.11 (2024-01-02)](#v0911-2024-01-02)
    - [v0.9.10 (2023-12-22)](#v0910-2023-12-22)
    - [v0.9.9 (2023-10-21)](#v099-2023-10-21)
    - [v0.9.8 (2023-10-21)](#v098-2023-10-21)
    - [v0.9.7 (2023-10-17)](#v097-2023-10-17)
    - [v0.9.6 (2023-10-17)](#v096-2023-10-17-1)
    - [v0.9.5 (2023-10-14)](#v095-2023-10-14-1)
    - [v0.9.1 (2023-03-06)](#v091-2023-03-06)
  - [`r3bl_terminal_async`](#r3bl_terminal_async)
    - [Archived (2025-04-05)](#archived-2025-04-05)
    - [v0.6.0 (2024-10-21)](#v060-2024-10-21-1)
    - [v0.5.7 (2024-09-12)](#v057-2024-09-12)
    - [v0.5.6 (2024-08-13)](#v056-2024-08-13)
    - [v0.5.5 (2024-07-13)](#v055-2024-07-13)
    - [v0.5.4 (2024-07-12)](#v054-2024-07-12)
    - [v0.5.3 (2024-05-22)](#v053-2024-05-22)
    - [v0.5.2 (2020-05-06)](#v052-2020-05-06)
    - [v0.5.1 (2024-04-28)](#v051-2024-04-28)
    - [v0.5.0 (2024-04-22)](#v050-2024-04-22)
    - [v0.4.0 (2024-04-21)](#v040-2024-04-21)
    - [v0.3.1 (2024-04-17)](#v031-2024-04-17)
    - [v0.3.0 (2024-04-15)](#v030-2024-04-15)
  - [`md_parser_ng`](#md_parser_ng)
    - [Archived (2025-07-22)](#archived-2025-07-22)
  - [`r3bl_core`](#r3bl_core)
    - [Archived (2025-04-21)](#archived-2025-04-21)
    - [v0.10.0 (2024-10-20)](#v0100-2024-10-20)
  - [`r3bl_tuify`](#r3bl_tuify)
    - [Archived (2025-04-05)](#archived-2025-04-05-1)
    - [v0.2.0 (2024-10-21)](#v020-2024-10-21)
    - [v0.1.27 (2024-09-12)](#v0127-2024-09-12)
    - [v0.1.26 (2024-04-15)](#v0126-2024-04-15)
    - [v0.1.25 (2024-01-14)](#v0125-2024-01-14)
    - [v0.1.24 (2023-12-31)](#v0124-2023-12-31)
    - [v0.1.23 (2023-12-22)](#v0123-2023-12-22)
    - [v0.1.22 (2023-12-20)](#v0122-2023-12-20)
    - [v0.1.21 (2023-10-21)](#v0121-2023-10-21)
    - [v0.1.20 (2023-10-21)](#v0120-2023-10-21)
    - [v0.1.19 (2023-10-17)](#v0119-2023-10-17)
    - [v0.1.18 (2023-10-17)](#v0118-2023-10-17)
    - [v0.1.17 (2023-10-14)](#v0117-2023-10-14)
  - [`r3bl_script`](#r3bl_script)
    - [Archived (2025-03-31)](#archived-2025-03-31)
  - [`r3bl_log`](#r3bl_log)
    - [Archived (2025-03-30)](#archived-2025-03-30)
  - [`r3bl_test_fixtures`](#r3bl_test_fixtures)
    - [Archived (2025-03-29)](#archived-2025-03-29)
    - [v0.1.0 (2024-10-21)](#v010-2024-10-21)
    - [v0.0.3 (2024-09-12)](#v003-2024-09-12)
    - [v0.0.2 (2024-07-13)](#v002-2024-07-13)
    - [v0.0.1 (2024-07-12)](#v001-2024-07-12)
  - [`r3bl_ansi_color`](#r3bl_ansi_color)
    - [Archived (2025-03-28)](#archived-2025-03-28)
    - [v0.7.0 (2024-10-18)](#v070-2024-10-18)
    - [v0.6.10 (2024-09-12)](#v0610-2024-09-12)
    - [v0.6.9 (2023-10-21)](#v069-2023-10-21)
    - [v0.6.8 (2023-10-16)](#v068-2023-10-16)
    - [v0.6.7 (2023-09-12)](#v067-2023-09-12)
  - [`r3bl_macro`](#r3bl_macro)
    - [Archived (2025-03-11)](#archived-2025-03-11)
    - [v0.10.0 (2024-10-20)](#v0100-2024-10-20-1)
  - [`r3bl_simple_logger`](#r3bl_simple_logger)
    - [Archived (2024-09-27)](#archived-2024-09-27)
    - [v0.1.4 (2024-09-12)](#v014-2024-09-12)
    - [v0.1.3 (2023-10-21)](#v013-2023-10-21)
    - [v0.1.2 (2023-10-21)](#v012-2023-10-21)
    - [v0.1.1 (2023-10-17)](#v011-2023-10-17)
    - [v0.1.0 (2023-10-14)](#v010-2023-10-14)
  - [`r3bl_redux`](#r3bl_redux)
    - [Archived (2024-09-29)](#archived-2024-09-29)
    - [v0.2.8 (2024-09-12)](#v028-2024-09-12)
    - [v0.2.7 (2024-09-07)](#v027-2024-09-07)
    - [v0.2.6 (2023-10-21)](#v026-2023-10-21)
    - [v0.2.5 (2023-10-17)](#v025-2023-10-17)
    - [v0.2.4 (2023-10-14)](#v024-2023-10-14)
  - [`r3bl_rs_utils`](#r3bl_rs_utils)
    - [Archived (2024-09-30)](#archived-2024-09-30)
    - [v0.9.16 (2024-09-12)](#v0916-2024-09-12-1)
    - [v0.9.15 (2023-12-22)](#v0915-2023-12-22)
    - [v0.9.14 (2023-10-29)](#v0914-2023-10-29)
    - [v0.9.13 (2023-10-29)](#v0913-2023-10-29)
    - [v0.9.12 (2023-10-29)](#v0912-2023-10-29)
    - [v0.9.11 (2023-10-28)](#v0911-2023-10-28)
    - [v0.9.10 (2023-10-21)](#v0910-2023-10-21)
    - [v0.9.9](#v099)
- [More info on changelogs](#more-info-on-changelogs)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Changelog

<!-- Rust Development Power Tools section -->

## Rust Development Power Tools

This section covers build scripts, toolchain management, and developer productivity tools for the
`r3bl-open-core` monorepo. Changes here affect how you build, test, and develop—not the library
APIs themselves.

### Rust Development Power Tools (2026-01-23)

This release focuses on developer productivity: faster builds with the nightly parallel compiler,
smarter toolchain management, and a streamlined bootstrap experience. We've also removed legacy
dependencies in favor of Claude Code's built-in capabilities.

Also see [`r3bl-build-infra`](#r3bl-build-infra) below—it provides `cargo rustdoc-fmt` for formatting
rustdoc comments, which is used extensively by Claude Code skills in this repo to maintain consistent
documentation style.

**Changed:**

- **Build system migration:** Migrated from nushell to fish shell for all build scripts - `run.fish`
  replaces `run.nu` throughout the project for improved maintainability and readability
- **Copyright headers:** Shrunk copyright headers in all files for better readability
- **Claude Code configuration:** Updated Claude Code configuration files and VS Code settings for
  improved development experience
- **Updated project bootstrap experience:** `fzf` and `fish` are now required for the `bootstrap.sh`
  script to work correctly, ensuring a smoother setup process
- **Documentation:** Updated README.md with comprehensive Claude Code integration section
  documenting `.claude/` directory structure, available skills, and slash commands
- **Documentation:** Fixed script name inconsistencies (`rust-toolchain-validate.fish` not
  `rust-toolchain-install-validate.fish`)
- **Documentation:** Added tmux-r3bl-dev.fish development dashboard documentation
- **Documentation:** Clarified VSCode extensions installation workflow

**Added:**

- **`check.fish` development workflow script:** Comprehensive build automation with significant
  performance optimizations:
  - **Nightly parallel frontend compiler** (`-Z threads=N`) for ~30-40% faster builds
  - **Shared tmpfs target** (`$CARGO_TARGET_DIR=/tmp/roc/target/check`) — all tools (Claude, VSCode,
    check.fish) share one RAM-based cache, so builds are always warm
  - **Watch modes** (`--watch`, `--watch-doc`, `--watch-tests`) keep the cache perpetually warm—
    sophisticated debounce waits for a 2-second quiet period, so rapid file changes (from typing,
    Claude Code, linters, etc.) don't trigger frenzied rebuilds
  - **Two-stage doc builds**: Quick blocking (~3-5s) + full background (~90s) with staging directories
    that prevent the docs folder from going empty during slow rustdoc rebuilds
  - **ICE detection and auto-recovery** for Internal Compiler Errors
  - **Single-instance enforcement** kills orphaned watchers automatically

- **Smart rust-toolchain management** (`rust-toolchain-update.fish`, `rust-toolchain-sync-to-toml.fish`):
  Nightly Rust is powerful but unstable—these scripts automatically find stable nightlies, validate
  them against your code, and remove the hassle of toolchain management entirely.
  - **Automated stable nightly discovery**: Tests nightlies from ~1 month ago for ICE errors before
    committing—you get nightly features without nightly instability
  - **Aggressive cleanup**: Removes old nightlies while preserving known-good toolchains
  - **Corruption recovery**: Detects "Missing manifest" errors and force-removes broken toolchains
  - **Desktop notifications** for success/failure via `notify-send`
  - **Comprehensive logging** to `~/Downloads/rust-toolchain-*.log`

- **Coming soon:** [`cargo-monitor`][cargo-monitor-plan] - Rust port of `check.fish` as a proper
  cargo subcommand in `r3bl-build-infra`, bringing these optimizations to any Rust project

[cargo-monitor-plan]: https://github.com/r3bl-org/r3bl-open-core/blob/e083dc39f8de27fe81a864aad22a925d99a65fb5/task/pending/build_infra_cargo_monitor.md

**Removed:**

- **Legacy tools:** Removed `go`, `mcp-language-server`, `serena`, `uv`, and `sccache` dependencies
  (Claude Code now has built-in LSP; sccache proved unreliable)

### Rust Development Power Tools (2025-08-04)

**Added:**

- **Bootstrap script (`bootstrap.sh`):** New automated setup script for getting the project running
  on Linux and macOS systems
- **Claude custom commands:** Added 7 custom commands in `.claude/commands/` folder:
  - `/rust` - Comprehensive Rust development expert command with MCP tools guidance
  - `/ra_def` - Quick access to rust-analyzer definition lookup
  - `/ra_diag` - Quick access to rust-analyzer diagnostics
  - `/ra_edit` - Quick access to rust-analyzer file editing
  - `/ra_hover` - Quick access to rust-analyzer hover information
  - `/ra_ref` - Quick access to rust-analyzer references
  - `/ra_ren` - Quick access to rust-analyzer symbol renaming
- MCP server configuration and usage instructions for Claude Code and VS Code integration
- Enhanced MCP setup with docker and rust-analyzer support

**Changed:**

- **Consolidated build system:** Rewrote and consolidated all project-specific `run.nu` scripts into
  a single unified file at the root level for streamlined development workflow
- **Improved project bootstrap experience:** Simplified setup process with centralized commands and
  updated documentation
- Updated Claude Code settings and configurations

### Rust Development Power Tools (2025-07-22)

This release enhances the development workflow with AI-assisted coding support and improved
performance analysis tooling. Issue: <https://github.com/r3bl-org/r3bl-open-core/issues/397>. PR:
<https://github.com/r3bl-org/r3bl-open-core/pull/430>.

- Added:
  - Claude Code workflow documentation in `CLAUDE.md` for AI-assisted development
  - Task tracking system using `todo.md` and `done.md` files
  - Flamegraph profiling support with `.perf-folded` file generation
  - New profiling and benchmarking commands in `run.nu` scripts

- Changed:
  - Enhanced run.nu scripts with improved performance analysis tooling

### Rust Development Power Tools (2025-04-21)

- Changed:
  - Rename all the `run` scripts to `run.nu`, so that IDEs and editors can apply proper syntax
    highlighting (those that use file extension for this, such as RustRover).
  - Since all the excessive crates have been consolidated into one `r3bl_tui`, all the examples are
    now run using a single command:
    ```sh
    cd tui
    nu run.nu examples
    ```
  - Disable GitHub Actions for the project. With the addition of `nu run.nu build-server` there is
    no need to run this anymore, since the project, since it is possible to run the build server
    locally and run `cargo test --all-targets` and `bacon doc -W` continuously.
  - Remove `core` from the root level workspace `Cargo.toml` file.
  - Update `config.toml` in the root folder to use `28` threads for parallel compilation, instead of
    `8`. The `-Z` flag is an unstable compiler directive only applies to `nightly` Rust.
  - Update `rustfmt.toml` to wrap comments using `comment_width = 90` and `wrap_comments = true`,
    and passing the `+nightly` flag to `rustfmt` (in IDEA, etc.)

### Rust Development Power Tools (2025-03-24)

- Added:
  - Add a new `script_lib.nu` file in the workspace (root) folder that contains functions that are
    used in the `run` scripts in all the contained inside of it. This provides the ability to easy
    run examples by providing the user with an interactive list of examples to run. See
    `terminal_async`'s `run` script for an example of how to use this. It also automatically detects
    all the crates that are contained in the workspace. `Cargo.toml` still has to be updated
    manually when crates in this workspace change.
  - Add a new target to the `run` script in the workspace (root) folder that allows the use of a
    "build server". This target is called `nu run build-server`. This sets up a watcher using
    `inotifywait` to watch for changes in the source folders in `r3bl-open-core` and `rsync` them to
    a destination ("build server") when changes occur. The "build server" needs to run `bacon` to
    monitor tests, docs, and clippy warnings continuously. This offloads all the computation from
    happening on the local machine. This is preferred over creating a network share for the files on
    the local machine, and running `bacon` on the "build server" machine. The problem with file
    sharing is that a lot of locking occurs (when the remote process generates artifacts on changes)
    that slow down the local machine's rust-analyzer from being interactive (and it becomes really
    laggy and unusable).

- Removed:
  - Update the `run` script at the top level, and `Cargo.toml` at the top level to remove the
    crates: `r3bl_test_fixtures` and `r3bl_ansi_color`. Move these crates to [archived](#archived).
    `r3bl_core` now contains all the functionality that was in `r3bl_ansi_color` and
    `r3bl_test_fixtures`.

### Rust Development Power Tools (2025-03-19)

This [PR](https://github.com/r3bl-org/r3bl-open-core/pull/378) contains the details for the
following:

- Updated:
  - Add a few RUSTSEC advisories to the whitelist in `deny.toml` file. There's a total of 5 in there
    now.
    - `"RUSTSEC-2024-0436", # "paste 1.0.15"`
    - `"RUSTSEC-2024-0320", # "instant@0.1.13"`
    - `"RUSTSEC-2024-0384", # "hashbrown@0.15.0"`
    - `"RUSTSEC-2024-0402", # "yaml-rust@0.4.5"`
    - `"RUSTSEC-2024-0421", # "idna v0.5.0"`
  - Add a new target to the `run` script in `tui` folder, to run examples without logging with
    release build: `nu run release-examples-no-log`.

### Rust Development Power Tools (2024-12-04)

This [PR](https://github.com/r3bl-org/r3bl-open-core/pull/370) contains the details for the
following:

- Updated:
  - Add a few RUSTSEC advisories to the whitelist in `deny.toml` file. These are persistent warnings
    and errors for 3 crates that are unmaintained. The main one is `syntect` which has not been
    updated in 10 months. I think there is some activity in their repo that will allow this issue to
    be resolved.
  - Run the `audit-deps` function in the `all-cicd` function, which is run in GitHub Actions. With
    the whitelist in place, this is ok to run and won't produce errors (since we already know about
    these 3 crates). The `unmaintained` function takes too long to run so it is still not included
    in teh `all-cicd` function.

<!-- Active crates section -->

## `r3bl_tui`

A fully async modern TUI framework for Rust (nothing blocks the main thread). Features
include flexbox-like layouts, CSS-like styling, reactive state management, async readline
(non-blocking alternative to POSIX readline), a Markdown editor component, custom markdown
renderer with syntax highlighting, gradient colors (lolcat rainbow), Unicode/emoji
support, modal dialogs, mouse events, full VT100 input/output parsers, and a
double-buffered compositor optimized for SSH (paints only diffs). Native Linux input via
RRT/mio/epoll (no crossterm dependency). PTY testing infrastructure for real-world
automated e2e testing and snapshot testing of TUI apps. Full PTY mux primitives so you can
build your own tmux effortlessly. Auto-detects terminal capabilities and gracefully
degrades. Works on Linux, macOS, and Windows. Add to your project with `cargo add
r3bl_tui`.

### v0.7.8 (2026-01-23)

**Fixed:**

- **VSCode terminal color rendering:** Changed ANSI escape sequence format from colon-separated
  (`ESC[38:2:r:g:bm`) to semicolon-separated (`ESC[38;2;r;g;bm`) for universal terminal
  compatibility. The colon format (ITU-T T.416) is technically correct but not supported by
  VSCode's xterm.js terminal emulator and many other terminals. The semicolon format (xterm
  de-facto standard) works everywhere. Our VT100 parser still accepts both formats for maximum
  compatibility when parsing output from other applications.

- **Glyph font compatibility:** Replaced exotic Unicode glyphs with more universally-supported
  characters that render correctly across more terminal fonts. Affected glyphs: check marks,
  fail indicator, pointer, game character, and terminal icon.

### v0.7.7 (2026-01-23)

This release introduces major architectural additions including the DirectToAnsi backend for
Linux-native terminal I/O, the Resilient Reactor Thread (RRT) pattern for blocking I/O handling,
comprehensive VT100 parser upgrades enabling in-memory terminal emulation, and PTY-based integration
testing infrastructure. It also includes a PTY multiplexer with terminal multiplexing functionality
similar to tmux.

**Major Infrastructure Additions:**

- **DirectToAnsi Backend (Linux-Native):**
  A custom terminal I/O implementation bypassing Crossterm on Linux for lower latency and reduced overhead.
  See the [DirectToAnsiInputDevice docs](https://docs.rs/r3bl_tui/0.7.7/r3bl_tui/tui/terminal_lib_backends/direct_to_ansi/input/struct.DirectToAnsiInputDevice.html) for architecture details.
  - Direct ANSI escape sequence handling with full VT100 compliance
  - Input device using `mio` (epoll) for async I/O multiplexing
  - Output device with `PixelCharRenderer` and smart attribute diffing (~30% output reduction)
  - Zero-latency ESC key detection and full keyboard modifier support
  - Mouse event handling with bracketed paste support
  - SIGWINCH signal integration for terminal resize
  - Crossterm still used on macOS/Windows where platform APIs differ

- **Resilient Reactor Thread (RRT) Pattern:**
  A reusable pattern for bridging blocking I/O with async Rust—spawn a dedicated thread, broadcast
  events to async consumers, and handle graceful shutdown automatically.
  The [module documentation](https://docs.rs/r3bl_tui/0.7.7/r3bl_tui/core/resilient_reactor_thread/index.html#architecture-overview) provides a comprehensive architecture overview.
  - `ThreadSafeGlobalState<W, E>` - Thread-safe singleton pattern for RRT instances
  - `ThreadLiveness` - Running state + generation tracking for safe thread reuse
  - `SubscriberGuard<W, E>` - Manages subscriber lifecycle with waker access
  - Broadcasts events to async consumers via broadcast channels
  - Handles graceful shutdown when all consumers disconnect

- **VT100/ANSI Output Parser & In-Memory Terminal Emulation:**
  Complete VT100 ANSI implementation enabling snapshot testing.
  See the [ansi module](https://docs.rs/r3bl_tui/0.7.7/r3bl_tui/core/ansi/index.html) for the parser and [`OffscreenBuffer`](https://docs.rs/r3bl_tui/0.7.7/r3bl_tui/tui/terminal_lib_backends/offscreen_buffer/struct.OffscreenBuffer.html) for the in-memory terminal.
  - VTE parser integration with custom `Performer` implementation
  - Full support for cursor movement, erase operations, scroll regions, SGR (colors/styles)
  - Enables snapshot testing: compare expected vs actual terminal state without a real terminal

- **PTY Testing Infrastructure:**
  Real-world testing in pseudo-terminals instead of mocks.
  See the [`generate_pty_test!`](https://docs.rs/r3bl_tui/0.7.7/r3bl_tui/macro.generate_pty_test.html) macro documentation for usage and the Controller/Controlled pattern.
  - Controller/Controlled pattern for test isolation
  - `generate_pty_test!` macro for single-feature tests
  - `spawn_controlled_in_pty` for multi-backend comparison tests
  - Backend compatibility tests verifying DirectToAnsi vs Crossterm produce identical results
  - Test coverage: bracketed paste, keyboard modifiers, mouse events, SIGWINCH, UTF-8

- **Enhanced readline_async API:**
  Expanded keyboard support makes building CLI apps easier—Tab completion, arrow key navigation,
  and function keys now work out of the box.
  See the [readline_async module](https://docs.rs/r3bl_tui/0.7.7/r3bl_tui/readline_async/index.html) for the full API.
  - Tab and BackTab (Shift+Tab) key support
  - Navigation keys support (arrow keys, Home, End, PageUp, PageDown)
  - FnKey support (F1-F12)
  - Type-safe editor state methods via `ReadlineAsyncContext`
  - Extended `ReadlineEvent` enum with new variants

- **PTY Multiplexer:**
  Terminal multiplexing functionality similar to tmux.
  See the [pty_mux module](https://docs.rs/r3bl_tui/0.7.7/r3bl_tui/core/pty_mux/index.html) for the complete API.
  - Enhanced support for truecolor and TUI apps that frequently re-render their UI
  - `pty_mux_example.rs` demonstrating multiplexer capabilities with multiple TUI processes
  - Support for spawning and switching between multiple TUI processes using Ctrl+1 through Ctrl+9
  - Live status bar showing process states (🟢 running, 🔴 stopped) and keyboard shortcuts
  - OSC sequence integration for dynamic terminal title updates
  - Fake resize technique for proper TUI app repainting when switching processes
  - Support for configurable TUI processes: less, htop, claude, gitui
  - `pty_simple_example.rs` for basic PTY functionality demonstration
  - `pty_rw_echo_example.rs` for PTY echo testing and validation
  - `ansi/terminal_output.rs` module with high-level terminal operations
  - PTY integration tests that spawn real TUI apps (eg: htop) to validate VT100 parsing

**Internal Improvements:**

- Changed:
  - Refactored mio_poller module for improved clarity and thread reuse semantics
  - Reduced DirectToAnsi input device complexity
  - Thread liveness tracking integrated with mio_poller for restart capability
  - Enhanced PTY read-write session with comprehensive cursor mode support
  - Improved PTY input/output event handling with extensive terminal input event mapping
  - Enhanced color conversion in `crossterm_color_converter.rs`
  - Improved styling in readline components (`apply_style_macro.rs`)
  - Enhanced spinner rendering with better visual feedback (`spinner_render.rs`)
  - Improved crossterm backend rendering operations (`render_op_impl.rs`)
  - Integrated with r3bl_tui's TuiColor system for consistent styling
  - Cleaned up read-only session to remove read-write specific code

### v0.7.6 (2025-08-16)

Refactor and reorganize the `pty` module to improve ergonomics and usability.

- Changed:
  - Refactor the `pty` module to be more ergonomic.
  - Move the `osc_seq.rs` into a top level `osc` module.

### v0.7.5 (2025-08-15)

Fixed Windows compatibility issues with PTY exit status handling.

- Fixed:
  - Windows compatibility for PTY exit status handling by properly using Windows-specific exit code
    encoding instead of Unix signal-based encoding

### v0.7.4 (2025-08-15)

This release introduces a comprehensive PTY (pseudo-terminal) module with full process control
capabilities, enhanced spinner messaging, and complete OSC 8 hyperlink support for modern terminal
interactions.

- Added PTY module:
  - New comprehensive PTY (pseudo-terminal) module with both read_only and read_write APIs
  - Support for spawning and controlling child processes in pseudo-terminals
  - Multiple examples demonstrating PTY functionality including `spawn_pty_read_only.rs` and
    `spawn_pty_read_write.rs`
- Added OSC support:
  - Support for parsing OSC (Operating System Command) terminal control sequences
  - Complete implementation of OSC 8 escape sequences for creating clickable hyperlinks in terminals
  - Helper functions for formatting file paths as clickable links
  - Smart terminal capability detection with blacklist-based approach for modern terminals
- Enhanced Spinner functionality:
  - Dynamic `interval_message` support allowing real-time message updates during execution
  - Integrated Spinner with PTY module for real-time progress reporting during long-running
    operations
- Improved testing and documentation:
  - Enhanced PTY test coverage and fixed test failures
  - Comprehensive PTY module documentation with usage examples

### v0.7.3 (2025-08-04)

Major performance optimization release with complete architectural overhaul of the gap buffer
implementation. Eliminated memory-intensive `SmallVec<GCStringOwned>` operations that were
materializing to `String` on every render in the main event loop, achieving zero-copy performance
through the new `ZeroCopyGapBuffer` implementation.

- Added:
  - Comprehensive documentation for three index types: `ByteIndex`, `SegIndex`, `ColIndex`
  - New hierarchical organization of the graphemes module
  - Common iterator extraction for graphemes processing
  - `GCString` segment logic extracted into `segment_builder.rs`

- Changed:
  - Complete rewrite of gap buffer implementation (in 5 phases) for zero-copy operations
  - Refactored `GCString` into trait-based design with separate owned and reference implementations
  - Restructured entire graphemes folder into hierarchical organization
  - Full migration to `ZeroCopyGapBuffer` architecture

- Performance improvements:
  - 2-3x overall application performance improvement
  - Complete elimination of major bottlenecks (100% reduction each)
  - 27-89% reduction in other bottlenecks
  - 50-90x faster append operations with zero-copy access
  - ~88.64% of total execution time eliminated from top 5 bottlenecks
  - Enhanced parser performance, editor responsiveness, and memory usage

### v0.7.2 (2025-07-23)

Major performance optimization release with significant architectural improvements to the TUI
engine. The markdown parser has been completely overhauled for massive performance gains, and the
rendering pipeline has been optimized to reduce CPU usage and memory allocations. This release also
includes extensive code quality improvements and Windows compatibility fixes. Issue:
<https://github.com/r3bl-org/r3bl-open-core/issues/397>. PR:
<https://github.com/r3bl-org/r3bl-open-core/pull/430>.

- Added:
  - LRU cache infrastructure for dialog border rendering optimization
  - Comprehensive snapshot testing framework for markdown parser
  - Benchmarking infrastructure for parser performance analysis
  - Enhanced profiling support with `.perf-folded` integration
  - Memory size caching with `GetMemSize` trait
  - Efficient `Display` traits for telemetry logging

- Changed:
  - Optimized markdown parser performance by 600-5,000x using hybrid approach
  - Made `SyntaxSet` & `Theme` global resources to reduce allocations
  - Reorganized `md_parser` module structure for improved clarity
  - Flattened module structure by removing `tui_core`
  - Made `PixelChar` `Copy` by storing single char instead of string
  - Refactored undo/redo history with comprehensive test coverage
  - Migrated all tests from `test_editor.rs` to their respective modules
  - Improved ANSI output performance by avoiding `write!` macro
  - Optimized `ColorWheel` cache implementation
  - Enhanced `Pos` API to remove ambiguity

- Fixed:
  - Eliminated syntax highlighting bottleneck in markdown parser
  - Resolved paste performance issues for both clipboard and bracketed paste
  - Fixed Windows terminal compatibility problems
  - Corrected doctests that couldn't run in test environment
  - Fixed telemetry bugs in `main_event_loop.rs`
  - Addressed Rust 2024 if-let-else rescope changes
  - Resolved extensive clippy warnings and pedantic lints
  - Fixed Unicode handling in `find_substring()` optimization
  - Added language name mapping for syntax highlighting to support both language names (e.g.,
    "rust") and file extensions (e.g., "rs") in markdown code blocks

- Performance:
  - 13.6% CPU reduction through optimized grapheme segmentation and dialog border colorization
  - Optimized string truncation with ASCII fast path
  - Improved tracing performance with `record_str` for Display formatting
  - Enhanced color support detection with proper memoization

### v0.7.1 (2025-05-10)

Minor change to remove `#![feature(let_chains)]` and `#![feature(trivial_bounds)]` from lib.rs so
that the crates can easily be installed using `cargo install r3bl-cmdr` instead of
`cargo +nightly install r3bl-cmdr`.

### v0.7.0 (2025-05-10)

This release contains changes that are part of optimizing memory allocation to increase performance,
and ensure that performance is stable over time. `ch_unit.rs` is also heavily refactored and the
entire codebase updated so that a the more ergonomic `ChUnit` API is now used throughout the
codebase. No new functionality is added in this release. The telemetry gathering and reporting
mechanism is rewritten. The undo and redo functionality is also rewritten. The many crates that once
existed in the `r3bl-open-core` monorepo are now consolidated into a single crate `r3bl_tui`, and
those extraneous crates are archived.

These videos have been an inspiration for many of these changes:

- [Data oriented design](https://youtu.be/WwkuAqObplU)
- [Memory alloc](https://youtu.be/pJ-FRRB5E84)

Fixed:

- `ReadlineAsync` is renamed to `ReadlineAsyncContext` and its lifecycle is cleaned up. Previously
  there were undefined behaviors related to the timing of when an exit operation was complete. This
  has been cleaned up and rewritten. Now the lifecycle is very clear: `try_new()` creates a context.
  You can use its `Readline` as much as you want. When you're done with this session, simply call
  `request_shutdown()` and then wait for that to complete using `await_shutdown()`.
- `Spinner` lifecycle has also been upgraded to match the `ReadlineAsyncContext`. It supports clean
  shutdown as well. It's lifecycle is different. You start it using `try_start()`. This spins up the
  spinner and it starts generating output. When you're done with it, you call `request_shutdown()`,
  then call `await_shutdown()` to ensure that it has completed its shutdown process.

Moved:

- `temp_dir.rs` is moved to `script` module where it belongs. The majority of code accessing
  `TempDir` is in the `script` module, and it makes sense to move it there.
- Move the contents of `r3bl_core` into `r3bl_tui` crate. Follow this [link](#archived-2025-04-21)
  to see all the final changes made to the `r3bl_core` crate before it was moved here. The
  `r3bl_core` crate is now archived. This allows for better organization of the codebase and makes
  it easier to maintain. The reason for `r3bl_core` was separated from the start is to support
  procedural macros, which have been removed from the codebase. In the future, if DSLs that require
  procedural macros are added, then a new crate will be created for that.
- Move the contents of `r3bl_terminal_async` into `r3bl_tui` crate.
- Move the contents of `r3bl_tuify` crate into `r3bl_terminal_async`. Consolidate the `select_*`
  functions into a single `choose()` function and make it async. The `r3bl_tuify` crate is now
  archived.
- Rename `terminal_async` to `readline_async`. The `choose` (async) function is under this folder.

Removed:

- Drop the dependency on `r3bl_ansi_color`.
- Move `spinner_impl` to `r3bl_tui` crate. This code is also used in `shared_global_data.rs` in
  `r3bl_tui` crate.
- Drop the dependency on `r3bl_tuify` crate in `Cargo.toml`.
- Drop the dependency on `r3bl_tui` crate in `Cargo.toml`.
- Remove the `println()` and `println_prefixed()` methods.
- Drop the dependency on `r3bl_ansi_color`.
- Remove `size-of` crate from `Cargo.toml`.
- Delete `static_global_data.rs` file and `telemetry_global_static` module.
  - Move the `vscode` terminal color detection code to `r3bl_ansi_color`, which is where it belongs.
  - Move the telemetry functions from `telemetry_global_static` to `Telemetry` module (and its
    dependencies `RateLimiter` and `RingBuffer`) in `r3bl_core`.

Updated:

- Use the latest Rust 2024 edition.
- The `Display` and `Debug` implementations for all the structs in this crate have been cleaned up.
  The `PrettyPrintDebug` trait is now deleted, and the `Debug` trait is used instead. Wherever
  possible the `Debug` trait does not perform any allocations and uses the `write!` macro to write
  to a pre-existing buffer.

Added:

- New `memory_allocator.rs` module that allow `jemalloc` to be loaded instead of the system default
  allocator. `jemalloc` is optimized for multi-threaded use cases where lots of small objects are
  created and deleted, which is a great fit for this crate. Use this in all the examples.
- New `network_io` module that contains support for length prefixed binary protocols that can be
  used to create TCP API servers. Currently this does not include TLS support, and that needs to be
  added later. The new module also supports easy to use `bincode` and `serde` serialization and
  deserialization, in addition to compression.
- Add the ability for `ReadlineAsync` to abort the `Readline` main event loop when it is exited (by
  calling the `ReadlineAsync::exit()` method). This is one of the main differentiators between a
  sync blocking `read_line()` and an async non-blocking one. Further clean up is provided that are
  triggered by `exit()` method to ensure that all the tasks that spin up are cleaned up: 1. task to
  listen to `InputDevice`, and 2. another task to process
  `crate::readline_impl::manage_shared_writer_output::spawn_task_to_monitor_line_control_channel`.
- Add `ta_println!`, `ta_print!`, `ta_println_prefixed` macros that use `format_args!` style (just
  like `println!`, `write!`, etc). This makes the API more familiar with Rust standard library. This
  style of declarative macro is used in other crates in this monorepo, like `r3bl_core`'s
  `into_existing.rs` module.
- Introduce `EditorEngine::ast_cache` which allows the `StyleUSSpanLines` that are generated for any
  content loaded into the editor to be cached. This is a huge performance win since scrolling and
  rendering is now much faster! Since the `EditorEngine` is free to be mutated during the render
  phase, the cache is invalidated and rebuilt after the `EditorBuffer` is modified. This works hand
  in hand with the `EditorBuffer::render_cache`.
- New `tui_color!` decl macro that allows all the delightful color palette used in `giti` to be
  available to anyone using this crate! `r3bl_ansi_color` is updated as well to work with this
  macro. `AnsiStyledText` has constructor function `fg_rgb_color()` and method
  `AnsiStyledText::bg_rgb_color()` that when combined make it very easy to use the `tui_color!`
  macro to create fun colors and then use them in the `AnsiStyledText` struct (via these new easy to
  use functions). This is very common when colorizing terminal output for log formatting or a
  welcome message. You can see this in the demo example for `r3bl_tui` which uses this
  `let msg_fmt = fg_rgb_color(ASTColor::from(tui_color!(lizard_green)), &msg);`. Also
  `r3bl_ansi_color` has an equivalent macro to this called `rgb_value!`.
- Add new target in `run` Nushell script called `nu run release-examples-no-log` to run the examples
  without logging. This is useful for performance testing. Now that HUD is displayed in the
  examples, there is no need to enable logging just to see this information (via `nu run log`).
- Add `spinner_impl` module from `r3bl_terminal_async` crate.
- Add `ResponseTimesRingBuffer` to replace `telemetry_global_static` module in
  `static_global_data.rs`. This is more accurate, performant, and space efficient. It uses a fixed
  backing store (`RingBuffer`) and a rate limiter (`RateLimiter`) to ensure that the report is not
  computed too frequently (since this might be an expensive operation that is called in a hot loop,
  the main event loop).

Changed:

- Change `Drop` impl for `TempDir` silently ignore errors when the directory can't be deleted. This
  was causing some issues with tests. The `create_temp_dir()` function is changed to
  `try_create_temp_dir()` to match the style of other code. Also add a new macro
  `try_create_temp_dir_and_cd!` to make it easy to perform operations that are often performed
  together. And added a `serial_async_test_with_safe_cd!` macro for testing async code that performs
  change directory operations (for `cargo test` which runs tests in the same process).
- Make `command!()` work with `TokioCommand` instead of `std::process::Command`. All the code in the
  `script` module now works under the assumption that they will run in an async context. And a real
  world use case for this is `giti` itself, in the way that it works with `git` commands and
  `cargo install r3bl-cmdr` commands.
- Fix `Spinner` bugs that caused undefined behavior in the timing of the output it produced to
  animate interval ticks and the final tick. The bugs were a result of non deterministic lock
  contention. The lock in question is the one required to be able to write to `OutputDevice` which
  itself can be a wrapper on top of `stdout`, `stdin`, etc. Rewrite the code so that it is explicit
  in the locking and sequence requirements for display operations. This ensures that sequences that
  must occur in order don't get delayed or interleaved with another task vying for the same lock.
  Also give `Spinner` the ability to work outside a `ReadlineAsync` context (eg: in `giti`'s
  `check_upgrade` module).
- In the `ReadlineAsync::read_line()` method, don't display the dangling prompt + `\n` when the
  program exits, after calling `ReadlineAsync::exit()`.
- Rename `get_readline_event()` to `read_line()`. This is an async replacement for
  `std::io::Stdin::read_line()`.
- Update all the demo examples in the `examples` folder to use the new telemetry API and display a
  HUD above the status bar to display FPS counter.
- Use `smallvec` and `smallstr` crates to increase memory latency performance (for access, mutation,
  and allocation).
- Use `Drop` trait implementation for `EditorBufferMut` to perform validation and clean up of caret
  location after changes to editor buffer, in `EditorBuffer::get_mut()`. Also provide a no `Drop`
  implementation that can be used via `EditorBuffer::get_mut_no_drop()`, which does not perform any
  cleanup.
- Improve the history handling in `EditorBufferMut` to ensure that the history is limited in size.
- Improve the `editor_buffer_struct.rs` module to use `SelectionList` instead of `HashMap` so that
  there's no need to perform any heap allocations for selection ranges (as long as they fit within
  the initial stack allocation). Regardless, by "scalarizing" or "flattening" the map into an array,
  the memory locality is improved (and cache line performance), since there's no need to follow a
  pointer from the key to get to they value. Both key and value tuple are stored side by side in an
  array (on the stack) or moved to the heap if they spill over the stack allocation. It's not a big
  deal in this case, but it's a good practice to follow.
- Rewrite `editor_buffer` module, and add lots of test to ensure good code coverage. And fix undo /
  redo bugs that were present in the previous implementation. Use "newtype" design pattern to
  represent `CurIndex` instead of using `usize` in many places. Eliminate the use of `bool` and
  functions that check for state, by encoding all the possible states in an enum `CurIndexLocation`
  and use that instead.
- Rewrite `RenderCache` using "newtype" pattern and "scalarize" it by removing `HashMap` and
  `String` keys. Introduce `CacheEntry` and `CacheKey` (which is just derived from `u16`s). Clean up
  tests so they provide good code coverage for happy path and lots of edge cases.

### v0.6.0 (2024-10-21)

This is a major release that not only includes new functionality, but is a radical reorganization of
the crates. The reason for paying down this technical debt now is to ensure that the codebase is
easier to maintain and understand, and easier to add new features to in the future. The separation
of concerns is now much clearer, and they reflect how the functionality is used in the real world.

Another huge change is the method signature of `main_event_loop()`. This is a breaking change, and
it uses dependency injection, to provide an input device, output device, state and app to the
function! This allows for new types of applications to be built, which can carry state around
between "applets".

This is part of a total reorganization of the `r3bl-open-core` repo. This is a breaking change for
almost every crate in the repo. This [PR](https://github.com/r3bl-org/r3bl-open-core/pull/360)
contains all the changes.

- Added:
  - Provide a totally new interface for the `main_event_loop()` that allows for more flexibility in
    how the event loop is run, using dependency injection. This is a breaking change, but it is
    needed to make the codebase more maintainable and possible to test end to end. This new change
    introduces the concept of providing some dependencies to the function itself in order to use it:
    state, input device, output device, and app. The function now returns these dependencies as
    well, so that they can be used to create a running pipeline of small "applets" where all of
    these dependencies are passed around, allowing a new generation of experiences to be built, that
    are not monolithic, but are composable and testable.
  - End to end test for the `main_event_loop_impl()` which tests everything in the TUI engine! 🎉
    This feature has taken about 2 years and 7 months to implement (`2024-10-07`)! This repo was
    created in `2022-02-23`, which you can get using
    `curl https://api.github.com/repos/r3bl-org/r3bl-open-core | jq .created_at`.

- Changed:
  - Refactor lots of styling related code in preparation for the move to `core`. This will make it
    easier to maintain and test the codebase, and clean up the dependencies.
  - The latest version of `unicode-width` crate `v2.0.0` changes the widths of many of the emoji.
    This requires lots of tests to be changed in order to work w/ the new constant width values.

- Removed:
  - Move the `color_wheel` module into `r3bl_core` crate. This is to ensure that it is possible to
    import just color wheel and lolcat related functionality without having to import the entire
    `r3bl_tui` crate. And de-tangles the dependency tree, making it easier to maintain. The reason
    they ended up in `r3bl_tui` in the first place is because it was easier to develop them there,
    but since then, lots of other consumers of this functionality have emerged, including crates
    that are created by "3rd party developers" (people not R3BL and not part of `r3bl-open-core`
    repo).

### v0.5.9 (2024-09-12)

- Updated:
  - Upgrade all deps to their latest versions in `Cargo.toml` and `Cargo.lock`.
  - Improve docs in `lib.rs` and `README.md`.

### v0.5.8 (2024-09-07)

- Removed:
  - Remove `get-size` crate from `Cargo.toml`. This was causing some
    [issues with `RUSTSEC-2024-0370`](https://github.com/r3bl-org/r3bl-open-core/issues/359).

- Added:
  - Add `size-of` crate.
  - This new crate is used to calculate the size of structs in bytes.
  - Change the implementations of many structs in the following modules: `editor_buffer`,
    `dialog_buffer`, `editor_component`, `editor_engine`, `color_wheel`, `lolcat`, and following
    files: `offscreen_buffer.rs`, `main_event_loop.rs`.

- Updated:
  - Use the latest deps for all crates in `Cargo.toml` and `Cargo.lock`.

### v0.5.7 (2024-08-13)

The biggest change in this release is rewriting the example runner using the latest
`r3bl_terminal_async` crate, and dropping the use of `reedline` crate (which is no longer used to
run the examples).

`r3bl_terminal_async` is fully async and allows seamless creation of REPLs and shells. It also
supports pause and resume for spinners, along with many other features.

- Updated:
  - Change the main examples launcher (which you can run using `nu run examples`) so that it
    correctly handles raw mode transitions, and also correctly uses the `r3bl_terminal_async` crate
    to ask the user for input (using "async readline").
  - Drop dependency on `reedline`. Use `r3bl_terminal_async` instead to get async readline
    capabilities. Update examples to use this new crate, and make example launcher easier to
    maintain.

- Fixed:
  - Fix a minor and subtle bug with shutdown signal in the main event loop.
    [PR](https://github.com/r3bl-org/r3bl-open-core/pull/336/) to fix this
    [issue](https://github.com/r3bl-org/r3bl-open-core/issues/331).

### v0.5.6 (2024-06-29)

The biggest change in this release is the rewrite of the Markdown parser. This was done because the
previous parser was not able to handle many corner cases in parsing complex fragments from a single
line of text, which is common in Markdown. The new parser is exhaustively tested and is able to
handle many more corner cases.

- Fixed:
  - Rewrite most of the Markdown parser and add exhaustive tests and lots of corner cases which were
    not covered before. A lot of these issues were found by using the `edi` binary target for a few
    weeks as a Markdown editor. Here's the [PR](https://github.com/r3bl-org/r3bl-open-core/pull/332)
    with these fixes.

- Updated:
  - Fix docs (for docs.rs and README.md for github and crates.io).
  - Fix clippy warnings.
  - Make minor refactors and cleanups.

### v0.5.5 (2024-05-20)

- Updated:
  - Fix typos in `README.md`.

### v0.5.4 (2024-05-20)

- Updated:
  - `README.md`
    - Fix image loading problems on crates.io. The `README.md` is shown in crates.io, and `lib.rs`
      is shown in docs.rs.
    - Clean up the content in the `README.md` file, and make it current, and update `lib.rs` to
      match.

### v0.5.3 (2024-04-15)

- Updated:
  - Dependency changes inherited from `r3bl_rs_utils_core` version `0.9.13`, and
    `r3bl_rs_utils_macro` version `0.9.9`.
  - Lots of clippy fixes.

### v0.5.2 (2024-01-14)

- Updated:
  - Dependency updated `reedline` version `0.28.0`, `r3bl_rs_utils_core` version `0.9.12`.

### v0.5.1 (2024-01-09)

- Added:
  - Simple function `ColorWheel::lolcat_into_string()` that receives a string and colorizes it using
    some defaults. It is similar to the `ColorWheel::colorize_into_string()` which it uses under the
    hood, but it is simpler to use.

### v0.5.0 (2023-12-31)

- Changed:
  - Rename `run.nu` to `run` in the `tui` folder. This simplifies commands to run it, eg:
    `nu run build`, or `./run build`.
  - Rename `run.nu` to `run` in the top level folder as well.
  - Replace the `run` command with `examples` in the `run` nushell script. To run an example you use
    `nu run examples`. and provide instructions on the `run` script at the top level folder of this
    monorepo. Update `lib.rs` and `README.md` to reflect this change. The behavior of the `run`
    nushell script is more uniform across all crates in this repo.
  - In `app.rs`, change `App` trait function `app_handle_signal()` to receive 2 new arguments:
    `component_registry_map`, and `has_focus`. This makes it similar to `app_handle_input_event()`.

- Fixed:
  - Editor component now cleans up state correctly after new content loads. This includes the
    undo/redo stack, and the render ops cache (for the content).
  - Fix `tui/examples/demo/ex_pitch` example to correctly move back and forward between slides.

- Added:
  - <kbd>Escape</kbd> key now clears the selection.
  - <kbd>Ctrl+A</kbd> now selects all text.
  - Tests for `EditorComponent` for undo / redo history, text selection, and clipboard service.
  - Add tests to editor component for clipboard service.

### v0.4.0 (2023-12-22)

- Changed:
  - Drop the use of Redux for state management entirely. Replace this with mutable state. And a new
    architecture for App and Component, that is more like Elm, rather than React and Redux.
  - Async middleware functions no longer use Redux for propagating state transitions to the app;
    instead, they now achieve this through
    [async `tokio::mpsc` channels](https://tokio.rs/tokio/tutorial/channels). Here's a
    [design doc](https://docs.google.com/document/d/1OMB1rX6cUL_Jxpl-OUWMhJijM7c4FoDrK6qDViVXBWk/edit)
    for this change. Here's the [issue](https://github.com/r3bl-org/r3bl-open-core/issues/196) and
    [PR](https://github.com/r3bl-org/r3bl-open-core/pull/205) for this change. Here are some videos
    that go over this massive change:
    - <https://youtu.be/o2CVEikbEAQ>
    - <https://youtu.be/Ne5-MXxt97A>
  - In the editor component, disable the syntect highlighter for the editor by default and just use
    the custom MD parser. For files that are not Markdown, we will probably need to enable syntect
    in the future since it is not covered by the custom MD parser & highlighter combo.

- Fixed:
  - Fix the custom MD parser so that it correctly parses plain text.

- Added:
  - Add undo, redo support for the editor component.
  - Add binary target for `edi` which is going to be a Markdown editor similar to `nano` or `micro`.
    It is meant to showcase what the `r3bl_tui` crate can do. It is also meant to be a useful
    productivity tool.
  - Add function `colorize_into_string()` to make it easy to apply color wheel to a string and then
    convert it into an ANSI styled string that can be used to print to the terminal emulator. Also
    added conversion function `convert_tui_color_into_r3bl_ansi_color()` to convert from `TuiColor`
    to `r3bl_ansi_term::Color`.
  - In editor component, add support for caching rendered output of content. When the content
    changes, or the viewport size or window size change, the cache is invalidated. This is useful
    for performance reasons. It also leverages the undo/redo system for cache invalidation (which
    makes it fast to invalidate the render ops cache w/out having to do a content comparison to
    detect changes).
  - Add lots of editor component tests for selection, content cache.

- Updated:
  - Update dependency on `reedline` crate to `0.27.1`.
  - Update dependency on `r3bl_rs_utils_core` to `0.9.10`.
  - Update dependency on `r3bl_rs_utils_macro` to `0.9.8`.

### v0.3.10 (2023-10-29)

- Changed:
  - Replaced `arboard` crate with `copypasta-ext`.
    - `arboard` was not working well on macOS and Windows.
    - The `copypasta-ext` crate should fix the problem w/ dropping the clipboard contents when an
      app using the editor component exits.
  - Added deps are upgraded to their latest versions.
  - Changed `cargo.deny` so that it now accepts `ISC` license.
- Added:
  - Support for select, copy, cut, paste, and delete have been added to the editor component.

### v0.3.9 (2023-10-29)

- Changed:
  - Dropped support for `clipboard` crate. Used `arboard` instead which is actively maintained and
    supported by 1Password. New Github Actions have been added to ensure that `cargo-deny` is used
    in order to check for crates going unmaintained (along w/ license audit checks). There are known
    issues w/ this crate on Wayland & Arch.
    <https://github.com/r3bl-org/r3bl-open-core/commit/3ba4ff821373361bedcd0b7185a4b6ba15b745c8>

### v0.3.7 (2023-10-21)

- Changed:
  - Dropped support for `palette` crate. Use `colorgrad` instead. More info here:
    <https://github.com/r3bl-org/r3bl-open-core/issues/162>

- Updated:
  - Upgraded all deps to their latest versions.

### v0.3.6 (2023-10-17)

- Changed:
  - Switched to using `r3bl_ansi_color` to detect terminal color capabilities and color output and
    conversions.
  - Apply `#[serial]` on tests that mutate global variables to make those tests un-flaky. This was
    already being done in `r3bl_ansi_color`, just bringing this over to the `r3bl_tui` crate with
    this release.

- Removed:
  - Dependency on `ansi_term` which is no longer maintained
    <https://rustsec.org/advisories/RUSTSEC-2021-0139.html>.
  - Needless dependencies on crates that are not used.

### v0.3.5 (2023-10-14)

- Added:
  - Support for selecting text using keyboard.
  - Support for copying text to clipboard using keyboard.
- Fixed:
  - Main event loop was actually doing the wrong thing and blocking on the thread. Even though it
    accepted an input event asynchronously using `AsyncEventStream` (`EventStream` is provided by
    `crossterm` and built using tokio async streams), it was blocking this task (running in parallel
    on a thread) as it was waiting for the input event to be processed by the app. The fix allows
    the main thread to simply spawn a new task (in parallel, on a thread) to process the input
    event. An `mpsc` channel is used in order for the async work done to signal to the main thread
    that it should break out of its infinite loop.

### v0.3.3 (2023-04-20)

- Added:
  - Add `ColorSupport` as a way to detect terminal emulator capabilities at runtime. This uses the
    [`concolor_query`](https://docs.rs/concolor-query/latest/concolor_query/) crate to detect
    terminal emulator capabilities at runtime.
  - At the `RenderOps` level, update `to_crossterm_color()` so that it uses `ColorSupport` to
    determine the best color to use based on terminal emulator capabilities at runtime. It can
    automatically convert from truecolor to ANSI 256 to grayscale. Note that if a color is specified
    as truecolor, then it will automatically be downgraded. If it is specified as ANSI or grayscale
    then it will not be downgraded.
  - Add `ColorWheel` as a way to consolidate all gradient related coloring. Gradients can be
    specified in truecolor, ANSI 256, or grayscale. The `ColorWheel` will automatically use the
    correct colors based on the terminal emulator capabilities at runtime using `ColorSupport`.
  - Add new Markdown parser written using [`nom`](https://crates.io/crates/nom) crate called
    `parse_markdown()`.
    - This parser not only parses regular Markdown but it also supports R3BL extensions for notes
      (metadata: tags, title, authors, date).
    - And it also supports smart lists (ordered and unordered). Smart lists also have support for
      todos (in the form of checked and unchecked items).
  - Add a new syntax highlighting engine for the new Markdown parser, in the `EditorComponent`
    called `try_parse_and_highlight()`.
    - It formats headings using different gradients for each heading levels 1-6. It also has elegant
      fallbacks for ANSI256 and grayscale.
    - It formats metadata (tags, title, authors, date) using different fg and bg colors.
    - Smart lists are formatted using different fg and bg colors. Ordered and unordered lists are
      formatted differently. Checked and unchecked items are formatted differently.
    - For code blocks, the `syntect` crate is used to do syntax highlighting based on the correct
      language of the code block. Since the R3BL theme `r3bl.tmTheme` specifies colors in truecolor,
      they will automatically be downgraded to ANSI256 or grayscale based on terminal emulator
      capabilities at runtime thanks to `to_crossterm_color()`.
  - To make console log debugging nicer, some new traits have been added `ConsoleLogInColor`,
    `PrettyPrintDebug`. These traits work together. If a struct implements `PrettyPrintDebug` then
    it gets the implementation of `ConsoleLogInColor` for free (which gives it the ability to print
    using fg and bg colors to the console).

### v0.3.2 (2023-03-06)

- Fixed:
  - Bug when trying to render an app that's taller than the offscreen buffer / terminal height

### v0.3.1 (2023-03-06)

- Added:
  - First changelog entry.
  - Remove dependency on ansi-parser crate:
    [issue](https://github.com/r3bl-org/r3bl-open-core/issues/91).
  - Make lolcat code better: [issue](https://github.com/r3bl-org/r3bl-open-core/issues/76).
    - Add `ColorSupport` as a way to detect terminal emulator capabilities at runtime.
    - Add `ColorWheel` as a way to consolidate all gradient related coloring. Use `ColorSupport` as
      a way to fallback from truecolor, to ANSI 256, to grayscale gracefully based on terminal
      emulator capabilities at runtime.
  - Provide for ANSI 256 color fallback for MacOS terminal app:
    [issue](https://github.com/r3bl-org/r3bl-open-core/issues/79)

- Removed:
  - Removed lolcat example from demo.

- Changed:
  - The first demo example (`ex_app_no_layout`) now has support for animation. It automatically
    increments the state every second and the gradient color wheel is updated accordingly.

## `r3bl-cmdr`

`r3bl-cmdr` provides two fully async TUI applications (built on [`r3bl_tui`](#r3bl_tui))
for developers. Both are currently available as early access preview 🐣. Install with
`cargo install r3bl-cmdr`.

- 😺 **giti** - An interactive git CLI app designed to give you more confidence and a
  better experience when working with git. Fully async—never blocks the main thread.
  Features visual branch selection and streamlined commit workflows.

- 🦜 **edi** - A TUI Markdown editor that lets you edit Markdown files in your terminal in
  style. Fully async—never blocks the main thread. Features gradient colors, smart
  terminal capability detection (gracefully degrades), smart list formatting, full emoji
  support, language-specific syntax highlighting inside fenced code blocks, SSH optimized
  (only repaints what's changed), and a zero-copy gap buffer for responsive editing even
  in large files.

### v0.0.26 (2026-01-23)

**Fixed:**

- **VSCode terminal color rendering:** Colors now display correctly in VSCode's integrated terminal
  (via updated r3bl_tui v0.7.8 dependency). Previously, colors appeared washed out or missing
  because VSCode's xterm.js terminal emulator doesn't support the colon-separated ANSI escape
  sequence format.

### v0.0.25 (2026-01-23)

Streamlined the crate by removing the experimental `ch` binary to focus on the core tools: `giti`
(interactive git) and `edi` (markdown editor).

- Removed:
  - `ch` binary and all associated code (Claude Code prompt history recall tool)
  - `ch` module from library exports

- Changed:
  - Updated documentation to reflect only `giti` and `edi` as the main binaries

### v0.0.24 (2025-08-16)

Documentation update for the `ch` command.

- Added:
  - Documentation for the `ch` command and its features, including usage examples and configuration
    options in the `README.md` file and `lib.rs` (for `docs.rs`).
  - ![ch video](https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/cmdr/videos/ch.gif?raw=true)

### v0.0.23 (2025-08-15)

Refactored `ch` command type system and enhanced user experience with improved output formatting.

- Changed:
  - Refactored `ch` command result type to `ChResult` for improved type clarity and better output
    formatting
  - Enhanced output messages for better user experience
  - ![ch video](https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/cmdr/videos/ch.gif?raw=true)

### v0.0.22 (2025-08-15)

Major feature release introducing the new `ch` binary for Claude Code prompt history management and
significant upgrade experience improvements. The new `ch` command provides a TUI interface for
browsing and copying previous Claude Code prompts, while the upgrade command now uses PTY for
resilient real-time feedback during the update process.

- Added new `ch` binary:
  - Claude Code prompt history recall and clipboard management tool
  - TUI selection interface using `choose()` function for browsing previous prompts
  - Cross-platform support for Linux, macOS, and Windows with automatic configuration detection
  - Clipboard integration for copying selected prompts to system clipboard
  - Image handling with automatic saving to `~/Downloads` directory using friendly filenames
  - Smart project matching that finds Claude projects from current or parent directories
  - Interactive terminal detection with graceful error handling for non-interactive environments
  - ![ch video](https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/cmdr/videos/ch.gif?raw=true)
- Enhanced upgrade experience:
  - Complete overhaul of upgrade command using PTY for resilient and rich user experience
  - Real-time feedback showing live progress from rustup and cargo install messages
  - Eliminated timeout appearance issues - upgrade process shows continuous progress
  - Improved error handling and recovery mechanisms during upgrade process
  - Added OSC 8 hyperlink support to make saved image file paths clickable in compatible terminals
  - Implemented blacklist-based terminal capability detection for OSC 8 hyperlinks
  - Enhanced user experience with rich feedback instead of misleading timeout screens
  - ![giti-upgrade video](https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/cmdr/videos/giti-upgrade.gif?raw=true)

### v0.0.21 (2025-08-04)

Performance optimization release benefiting from major infrastructure improvements across the
codebase, particularly the zero-copy gap buffer architecture. While there are no new features in
this release, the underlying performance gains significantly improve the overall user experience.

- Performance improvements:
  - 2-3x overall application performance improvement
  - Complete elimination of major bottlenecks (100% reduction each)
  - 27-89% reduction in other bottlenecks
  - 50-90x faster append operations with zero-copy access
  - ~88.64% of total execution time eliminated from top 5 bottlenecks
  - Enhanced parser performance, editor responsiveness, memory usage, and large document handling

### v0.0.20 (2025-07-23)

Minor release focusing on `edi` bug fixes, code quality improvements and documentation. Issue:
<https://github.com/r3bl-org/r3bl-open-core/issues/397>. PR:
<https://github.com/r3bl-org/r3bl-open-core/pull/430>.

- Fixed:
  - Clippy warnings about missing error documentation across the crate
  - Improved error handling documentation consistency
  - Fixed paste bug in `edi` where pasting was extremely slow, and many other long standing bugs
  - Huge performance improvements for `edi` making it super responsive, and much faster than before;
    see the PR for details

### v0.0.19 (2025-05-10)

Cleaning up more `let_chains` that were left after [v0.0.18](#v0018-2025-05-10) was made.

### v0.0.18 (2025-05-10)

Minor change to remove `#![feature(let_chains)]` and `#![feature(trivial_bounds)]` from lib.rs so
that the crates can easily be installed using `cargo install r3bl-cmdr` instead of
`cargo +nightly install r3bl-cmdr`.

### v0.0.17 (2025-05-10)

This is part of a total reorganization of the `r3bl-open-core` repo. This is a breaking change for
almost every crate in the repo. This [PR](https://github.com/r3bl-org/r3bl-open-core/pull/360)
contains all the changes.

This release also contains changes that are part of optimizing memory allocation to increase
performance, and ensure that performance is stable over time. `ch_unit.rs` is also heavily
refactored and the entire codebase updated so that a the more ergonomic `ChUnit` API is now used
throughout the codebase. No new functionality is added in this release.

- Added:
  - New `memory_allocator.rs` module that allow `jemalloc` to be loaded instead of the system
    default allocator. `jemalloc` is optimized for multi-threaded use cases where lots of small
    objects are created and deleted, which is a great fit for this crate. Use this in all the binary
    targets.
  - Add support for the binaries in the crate to upgrade themselves when a new version is detected.
  - Implement a new way for the binaries to detect when a new release of the crate is available so
    they can prompt the user that they can manually update or automatically update (above).

- Removed:
  - Drop the dependency on `r3bl_ansi_color`.

- Updated:
  - Use the latest Rust 2024 edition.
  - This release just uses the latest deps from `r3bl-open-core` repo, since so many crates have
    been reorganized and renamed. The functionality has not changed at all, just the imports.

- Changed:
  - Modernize the codebase so that it uses the latest code from `r3bl_tui`. The functionality is
    largely the same, however, almost every single line is rewritten, especially for `giti`.
  - Clean up all the styling and UI strings in the apps, so they are all in one place. This will
    make it easy to internationalize in the future, and make it possible to make changes easily and
    reduce maintenance burdens.

### v0.0.16 (2024-09-13)

- Updated:
  - Minor fix to documentation in `lib.rs` and `README.md` to use GIF instead of MP4 files for the
    `edi` and `giti` videos. The MP4 files were not showing on docs.rs, crates.io, or github.com.

### v0.0.15 (2024-09-12)

- Updated:
  - Upgrade all deps to their latest versions in `Cargo.toml` and `Cargo.lock`.
  - Improve docs in `lib.rs` and `README.md`.
  - Update `UPDATE_IF_NOT_THIS_VERSION` to `0.0.15`. This is kept in sync w/ the deployed backend
    `r3bl-base`.

### v0.0.14 (2024-06-29)

The most significant change in this release is the use of the latest release of the Markdown parser
from `r3bl_tui`, which improves the editing experience of writing each individual line of Markdown
text. Common edge cases that were not handled before are now handled correctly. And these are cases
that come up quite frequently when editing Markdown in a text editor.

- Fixed:
  - Use the latest release of the `r3bl_tui` crate version `0.5.6` which fixes a lot of common bugs
    with the Markdown parser. This are critical bug fixes that are needed for the `edi` binary
    target, to make it a stable and usable Markdown editor for daily use.

- Changed:
  - Use the latest release of the `r3bl_tui` crate version `0.5.5`.
  - Clean up `main_event_loop` and get rid of needless `'static` in `AS` trait bound.
  - Fix cargo clippy doc warnings.
  - Update `UPDATE_IF_NOT_THIS_VERSION` to `0.0.14`. This is kept in sync w/ the deployed backend
    `r3bl-base`.

- Updated:
  - Dependencies for `syntect`, `strum`, `strum-macros`, `reedline`, `serial_test` bumped to their
    latest versions.

### v0.0.13 (2024-05-20)

- Changed:
  - `Cargo.toml` now points to the correct documentation link on docs.rs.
  - `README.md` now has the correct URL for the hero image (that will load on crates.io and not just
    github.com).
  - `lib.rs` has the same URL for the hero image as `README.md`.
  - Update `UPDATE_IF_NOT_THIS_VERSION` to `0.0.13`. This is kept in sync w/ the deployed backend
    `r3bl-base`.

### v0.0.12 (2024-05-12)

- Changed:
  - Use the latest deps of all the `r3bl_*` crates to fix breaking build when
    `cargo install r3bl-cmdr` is run. Not sure how long this has been broken. Moving forwards, this
    will be checked using a VM on every release. Update `release-guide.md` instructions with this
    info. - `r3bl_ansi_color = { path = "../ansi_color", version = "0.6.9" }` -
    `r3bl_rs_utils_core = { path = "../core", version = "0.9.13" }` -
    `r3bl_rs_utils_macro = { path = "../macro", version = "0.9.9" }` -
    `r3bl_tui = { path = "../tui", version = "0.5.3" }` -
    `r3bl_tuify = { path = "../tuify", version = "0.1.26" }`
  - Update the `UPDATE_IF_NOT_THIS_VERSION` to `0.0.12`. This is kept in sync w/ the deployed
    backend `r3bl-base`.

### v0.0.11 (2024-01-14)

- Added:
  - `edi`, `giti`: Add checks to see if binary needs to be upgraded. - Search for
    `UPDATE_IF_NOT_THIS_VERSION` in `r3bl-open-core` repo (in `cmdr` folder), and in `r3bl-base`
    repo. `UPDATE_IF_NOT_THIS_VERSION` is set to `0.0.11` for this release. - If upgrade is needed,
    then display a message to the user asking them to run `cargo install r3bl-cmdr` again.
  - `giti` add feature: `giti branch checkout`
  - `giti` add feature: `giti branch new`
  - Add `reedline` version `0.28.0` dependency in `Cargo.toml`.

### v0.0.10 (2024-01-02)

- Fixed:
  - Refactor & clean up the analytics client code.

- Updated:
  - Use the latest `r3bl_rs_utils_core` version `0.9.11`.

### v0.0.9 (2023-12-31)

- Added:
  - Anonymized analytics reporting to prioritize feature development for `edi` and `giti`.

- Changed:
  - Replace the `run` command with `examples` in the `run` nushell script. To run an example you use
    `nu run examples`. and provide instructions on the `run` script at the top level folder of this
    monorepo. Update `lib.rs` and `README.md` to reflect this change. The behavior of the `run`
    nushell script is more uniform across all crates in this repo.

### v0.0.8 (2023-12-22)

- Changed:
  - Rename `run.nu` to `run` and update `README.md` and `lib.rs` to reflect this change. This is a
    more ergonomic command to use, when using it directly eg: `./run build` (macOS, Linux), or
    `nu run build` (Windows).

- Added:
  - Add binary target `giti`. This is an interactive git client that is tuified. It is a
    productivity tool for git workflows, and is meant as a replacement for directly using `git`.
    This also serves as a real world example of using the `r3bl_tuify` crate.
    - View all the `giti branch` subcommands (e.g. `delete`, `checkout`, `new`, etc.) and select one
      subcommand using the `select_from_list()` when `giti branch` runs.
    - Delete one or more branches using `select_from_list()` when `giti branch delete` command runs.
  - Add binary target `edi`. This is a powerful TUI Markdown editor. You can use it to create new MD
    files, or edit any type of text file. It supports syntax highlighting for most file formats
    (though `.toml` and `.todo` are missing).
  - Add binary target `rc` aka `r3bl-cmdr`.

## `r3bl-build-infra`

Cargo subcommands that automate the tedious parts of Rust development and speed up the
slow parts— documentation formatting, toolchain management, and build optimization.
Install with `cargo install r3bl-build-infra`.

### v0.0.1 (2026-01-23)

Initial release with `cargo rustdoc-fmt`—a cargo subcommand that formats markdown tables
and converts inline links to reference-style in rustdoc comments.

- Added:
  - `cargo-rustdoc-fmt` binary - Cargo subcommand for formatting rustdoc comments
    - Markdown table alignment in `///` and `//!` doc comments
    - Inline-to-reference link conversion for cleaner documentation
    - Workspace-aware processing (specific files, directories, or entire workspace)
    - Git integration (auto-detect changed files, staged/unstaged, from latest commit)
    - Check mode for CI verification (`--check` flag)
    - Selective formatting (tables only, links only, or both)
  - Modular library API for programmatic use

**Coming Soon 🚀**

- `cargo-monitor` - Unified development workflow automation (Rust port of `check.fish` and
  `rust-toolchain*.fish`):
  - Watch mode with continuous compilation, testing, and doc building on file changes
  - tmpfs target directory for ~2-3x faster builds in RAM
  - Automated nightly toolchain validation and corruption recovery
  - ICE (Internal Compiler Error) detection and auto-recovery
  - Two-stage doc builds (quick blocking + full background)
  - Cross-platform support (Linux/macOS)
  - See the [cargo-monitor implementation plan][cargo-monitor-plan] for details

[cargo-monitor-plan]: https://github.com/r3bl-org/r3bl-open-core/blob/e083dc39f8de27fe81a864aad22a925d99a65fb5/task/pending/build_infra_cargo_monitor.md

## `r3bl_analytics_schema`

### v0.0.3 (2025-05-10)

- Updated:
  - Use the latest deps.

### v0.0.2 (2024-09-12)

- Updated:
  - Use the latest Rust 2024 edition.
  - Upgrade all deps to their latest versions in `Cargo.toml` and `Cargo.lock`.
  - Improve docs in `lib.rs` and `README.md`.

### v0.0.1 (2023-12-31)

- Added:
  - Initial support structs for use by `r3bl-base` and `r3bl-cmdr`.

# Archived

<!-- Archived section -->

You can find all these archived crates in the
[archive](https://github.com/r3bl-org/r3bl-open-core-archive) repo.

## `r3bl_rs_utils_macro`

### Archived (formerly renamed to `r3bl_macro`)

This crate was renamed to `r3bl_macro` to make it consistent with the naming for all crates in this
repo, but `r3bl_macro` was subsequently archived in 2025-03-11.

### v0.9.10 (2024-09-12)

- Updated:
  - Upgrade all deps to their latest versions in `Cargo.toml` and `Cargo.lock`.
  - Improve docs in `lib.rs` and `README.md`.

### v0.9.9 (2024-04-16)

- Updated:
  - Use the latest `r3bl_rs_utils_core` version `0.9.13`.

### v0.9.8 (2023-12-22)

- Updated:
  - Use latest `r3bl_rs_utils_core` version `0.9.10`. Remove unused dependencies, and update to the
    latest ones.

### v0.9.7 (2023-10-21)

- Updated:
  - Upgrade all deps to their latest versions.

### v0.9.6 (2023-10-17)

- Updated:
  - Update `r3bl_rs_utils_core` crate due to
    <https://rustsec.org/advisories/RUSTSEC-2021-0139.html>, and `ansi_term` not being maintained
    anymore.

### v0.9.5 (2023-10-14)

- Updated:
  - Dependency on `simplelog` is replaced w/ `r3bl_simple_logger` (which is in the `r3bl_rs_utils`
    repo workspace as `simple_logger`).

## `r3bl_rs_utils_core`

### Archived (formerly renamed to `r3bl_core`)

This crate was renamed to `r3bl_core` to make it consistent with the naming for all crates in this
repo, but `r3bl_core` was subsequently archived in 2025-04-21.

### v0.9.16 (2024-09-12)

- Updated:
  - Upgrade all deps to their latest versions in `Cargo.toml` and `Cargo.lock`.
  - Improve docs in `lib.rs` and `README.md`.

### v0.9.15 (2024-09-07)

- Removed:
  - Remove `get-size` crate from `Cargo.toml`. This was causing some
    [issues with `RUSTSEC-2024-0370`](https://github.com/r3bl-org/r3bl-open-core/issues/359).

- Added:
  - Add `size-of` crate.
  - This new crate is used to calculate the size of structs in bytes (eg: `Vec<UnicodeString>` which
    is on the heap).
  - Change the implementations of many structs in the following modules: `tui_core`.
  - Add `common_math.rs` to `common` module, to make it easy to format numbers with commas. This is
    useful for displaying size in bytes or kilobytes, etc. in log output messages.

- Updated:
  - Use the latest deps for all crates in `Cargo.toml` and `Cargo.lock`.

### v0.9.14 (2024-08-13)

The main additions to this release are the `StringLength` enum, the `timed!()` macro, and the
`ok!()` macro.

- Added:
  - New enum `StringLength` that can be used to calculate the length of strings that have ANSI
    escape sequences in them. It also uses `UnicodeWidth` to calculate the " display" width of the
    (stripped) string. It also memoizes the result so that it is fast to calculate the length of the
    same string multiple times. This is used in the `r3bl_terminal_async` crate. It also has a
    method to calculate the SHA256 hash of a given `String`, and return it as a `u8`.
  - New declarative macro `timed!()` that measures the time the given expression takes to run using
    `time::Instant::now()`. If you use `timed!($expr)` then it will return a tuple of
    `($expr, duration)`.
  - New declarative macro `ok!()` that is just syntactic sugar for `Ok(())`. If you use `ok!($expr)`
    then it will return `Ok($expr)`.
  - Here's the [PR](https://github.com/r3bl-org/r3bl-open-core/pull/349) with all the code related
    to this release.

### v0.9.13 (2024-04-15)

- Changed:
  - Removed `syntect` dep.
  - Rename `Style` to `TuiStyle`.
  - Lots of cargo clippy fixes.

### v0.9.12 (2024-01-07)

- Added:
  - Add `generate_friendly_random_id()` to generate human readable and friendly IDs.

### v0.9.11 (2024-01-02)

- Added:
  - Add more variants to the `CommonErrorType` enum: `ConfigFolderCountNotBeCreated`,
    `ConfigFolderPathCouldNotBeGenerated`.

### v0.9.10 (2023-12-22)

- Updated:
  - Upgrade all the deps to their latest versions: `serde` version `1.0.190`. Propagate this to all
    the other crates in the `r3bl-open-core` repo, and bump their version numbers: e.g. `tuify`,
    `macro`, `tui`, `cmdr`.

### v0.9.9 (2023-10-21)

- Updated:
  - Upgrade all deps to their latest versions.

### v0.9.8 (2023-10-21)

- Updated:
  - Upgrade all deps to their latest versions.

### v0.9.7 (2023-10-17)

- Updated:
  - Dependency on `simple_logger` updated due to this security advisory
    <https://rustsec.org/advisories/RUSTSEC-2021-0139.html>. `simple_logger` itself had to drop
    `ansi_term`.

### v0.9.6 (2023-10-17)

- Removed:
  - Dependency on `ansi_term` is dropped due to this security advisory
    <https://rustsec.org/advisories/RUSTSEC-2021-0139.html>. Flagged when running CI/CD job on Ockam
    [repo](https://github.com/build-trust/ockam).

- Updated:
  - Documentation for `r3bl_simple_logger` crate. And how to think about it vs. using log facilities
    from the `r3bl_rs_utils_core` crate. Update docs there too.

### v0.9.5 (2023-10-14)

- Updated:
  - Dependency on `simplelog` is replaced w/ `r3bl_simple_logger` (which is in the `r3bl_rs_utils`
    repo workspace as `simple_logger`).
  - `TuiColor` has a few new variants. They can be `RgbValue`, `AnsiValue`, or `ANSIBasicColor`. It
    is safe to use just `RgbValue` since the library will degrade gracefully to ANSI 256 or
    grayscale based on terminal emulator capabilities at runtime (provided by `to_crossterm_color()`
    and `ColorSupport`). If a color is specified as `AnsiValue` or `ANSIBasicColor` then it will not
    be downgraded.

### v0.9.1 (2023-03-06)

- Added:
  - First changelog entry.
  - Move lolcat into `tui_core` crate.
- Removed:
  - ANSI escape sequences are no longer used internally in any intermediate format used by the TUI
    engine. It is reserved exclusively for output to stdout using (for now) crossterm. This opens
    the door for future support for GUI app (not just terminal emulators).

## `r3bl_terminal_async`

### Archived (2025-04-05)

Migrate the contents of `r3bl_terminal_async` crate into `r3bl_tui`.

### v0.6.0 (2024-10-21)

This is a major version upgrade and potentially a breaking change if you use the tracing modules in
this crate. This [PR](https://github.com/r3bl-org/r3bl-open-core/pull/360) contains all the changes.

- Added:
  - Add tests to ensure that the tracing module works as expected. This includes using the
    `assert_cmd` trait to test the output of a test binary that is run as a subprocess. Ensure that
    stdout and stderr are captured and can be tested for correctness. Also ensure that
    `SharedWriter` works as expected. Also ensure that file log output works as expected.

- Changed:
  - Use the latest Rust 2024 edition.
  - Refactor the tracing and Jaeger related code into 2 separate modules. This is laying the
    groundwork for these modules to be moved into `r3bl_core` crate. Radically simplify the tracing
    configuration design and init mechanisms, so they are easy to understand, use, and maintain.
  - Introduce the use of `InputDevice` and `OutputDevice` to make it consistent with `r3bl_tui`
    crate on how DI is used to provide input and output devices. The input device provides a way to
    get user input events from stdin (or from a test fixture). The output device provides a way to
    output to stdout (or to a test fixture). Replace the use of type aliases with the actual structs
    from `r3bl_core` crate.

- Deleted:
  - Move the Jaeger tracing module to the `tcp-api-server` crate in the
    [`rust-scratch`](https://github.comnazmulidris/rust-scratch/) repo. This wasn't really used
    anywhere else. Also remove all the OpenTelemetry related dependencies from this crate.
  - Move the tracing module into the `r3bl_core` crate, in the mono repo.

### v0.5.7 (2024-09-12)

- Updated:
  - Upgrade all deps to their latest versions in `Cargo.toml` and `Cargo.lock`.
  - Improve docs in `lib.rs` and `README.md`.

### v0.5.6 (2024-08-13)

The biggest change in this release is complete support for pause and resume. Now when the output is
paused, input is also paused, with the exception of allowing <kbd>Ctrl+C</kbd> and <kbd>Ctrl+D</kbd>
through.

The second big change is how spinners now work. Once a spinner is started, <kbd> Ctrl+C</kbd> and
<kbd>Ctrl+D</kbd> are directed to the spinner, to cancel it. Spinners can also be checked for
completion or cancellation by long running tasks, to ensure that they exit as a response to user
cancellation.

The third (and final) change is that `ReadlineAsync::try_new()` now accepts prompts that can have
ANSI escape sequences in them. By using `r3bl_rs_utils_core::StringLength` to calculate the display
width of strings containing ANSI escape sequences (by memoizing the results of the calculations),
the cost of repeatedly calculating this display width is almost eliminated.

Here's the [PR](https://github.com/r3bl-org/r3bl-open-core/pull/349) with all the code related to
this release.

- Added:
  - Add support to extend pause and resume functionality to the entire crate. Now, when the output
    is paused, for eg, when the spinner is running, then the input to the readline is also stopped,
    until output is resumed. This wasn't the case in the past, and it was possible to type and
    update the prompt while the output was paused.
  - Add user cancellation support for spinners. Once a spinner is started, <kbd>Ctrl+C</kbd> and
    <kbd>Ctrl+D</kbd> are directed to the spinner, to cancel it. Spinners can also be checked for
    completion or cancellation by long running tasks, to ensure that they exit as a response to user
    cancellation. Update the `examples/terminal_async.rs` to show how to best use this new feature.
  - Add better examples for how to use `ReadlineAsync::try_new()` in Rust docs.
  - Add a new example `async_shell.rs` to demonstrate how to use `ReadlineAsync` to create an
    interactive shell (with `bash` under the covers) that can orchestrate a shell asynchronously
    using [`tokio::process`](https://docs.rs/tokio/latest/tokio/process/struct.Child.html).

- Changed:
  - Clean up the shutdown mechanism for `ReadlineAsync` and `Readline` so that it is automatic, and
    doesn't require the use of `close()` anymore. By simply dropping the `Readline` instance, it
    will automatically clean up after itself (and correctly handle raw mode entry and exit).

### v0.5.5 (2024-07-13)

This minor release just updates the `r3bl_test_fixtures` crate to version `0.0.2` which adds a new
function to create an async stream that yields results (from a vec) at a specified interval.

- Changed:
  - Bump dependency on `r3bl_test_fixtures` to version `0.0.2`.

### v0.5.4 (2024-07-12)

This release migrates the test fixtures out of this crate and into a new top level crate in the
`r3bl-open-core` monorepo called `r3bl_test_fixtures`. This is to make it easier to maintain and
test the fixtures and allow all the other crates in this monorepo to use them. Here are all the
links for this release: [crates.io](https://crates.io/crates/r3bl_terminal_async),
[docs.rs](https://docs.rs/r3bl_terminal_async),
[GitHub](https://github.com/r3bl-org/r3bl-open-core/tree/main/terminal_async).

- Changed:
  - Remove the test fixtures out of this crate and into a new top level crate in the
    `r3bl-open-core` monorepo called `r3bl_test_fixtures`. This is to make it easier to maintain and
    test the fixtures and allow all the other crates in this monorepo to use them.

- Added:
  - Add `r3bl_test_fixtures` version `0.0.1` as a `dev-dependency` to this crate.

### v0.5.3 (2024-05-22)

This release adds a new module for checking port availability on a host, and adds a new function to
clean up the prompt when the CLI exits. It also adds a new module to allow for OpenTelemetry (OTel)
tracing to be added to the tracing setup. This uses the latest version of Jaeger and OpenTelemetry.

- Added:
  - New module to check for port availability on a host called `port_availability`. This is useful
    for checking if a port is available before starting a server (which is a common use case for
    interactive CLI programs).
  - Add `ReadlineAsync::print_exit_message()` - This cleans the prompt so it doesn't linger in the
    display output. This is intended to be used as the final display message when the CLI exits.
  - For greater flexibility `tracing_setup.rs` `try_create_layers(..)` now returns a `Vec<Layer>`.
    This allows for more flexibility in the future to add more layers to the tracing setup, such as
    adding an OTel (OpenTelemetry) layer, or a Jaeger layer, etc.
  - Add `jaeger_setup` module, to allow OTel layer to be added to the tracing setup. It uses the
    latest version of Jaeger and OpenTelemetry. The docs in `tokio.rs`
    [website](https://tokio.rs/tokio/topics/tracing-next-steps) (at the time of this writing) are
    out of date and use version `0.16.0` of `opentelemetry-jaeger` crate who's exporter component
    has already been deprecated and will be removed
    [soon](https://github.com/open-telemetry/opentelemetry-specification/pull/2858/files). Details
    are in [PR 326](https://github.com/r3bl-org/r3bl-open-core/pull/326). More info in this
    [blog post](https://broch.tech/posts/rust-tracing-opentelemetry/).

- Changed:
  - `try_create_layers(..)` also adds a level filter layer to the layers it returns. This is to
    ensure that the log level is set correctly for the log output, even if other layers (like the
    OTel / Jaeger) layer are sandwiched later. This is a minor change that should not affect the
    public API. Details are in [PR 326](https://github.com/r3bl-org/r3bl-open-core/pull/326).

### v0.5.2 (2020-05-06)

- Changed:
  - Rewrite the `tracing_setup.rs` file so that it is easier to understand and maintain. The
    creation of multiple layers in tracing is now streamlined with no code redundancy. The
    `r3bl_terminal_async::tracing_writer_config::Writer` is renamed to
    `r3bl_terminal_async::WriterArg`. This is minor change that should only affect `clap`
    configuration in CLI programs that use this. Use the best practices from the tokio tracing docs
    [here](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/index.html#runtime-configuration-with-layers)
    as inspiration for this change.

### v0.5.1 (2024-04-28)

- Changed:
  - Simplify `SpinnerRenderer` so that it is no longer a trait. Replace with plain functions receive
    a mutable ref to a `SpinnerStyle`. This trait just added more noise, making it more difficult to
    grok what this code does.
  - `SharedWriter` now silently ignores errors on `write()` for all cloned instances. Only the very
    first instance will return an error. This is to prevent needless error messages being displayed
    when using tokio tracing. This default behavior can easily be overridden by setting the
    `silent_error` field to `false` in the `SharedWriter` struct. Added tests for this as well.

- Added:
  - `ReadlineEvent::Resized` is a new variant that reports when the terminal has been resized. This
    is a feature request from [here](https://github.com/r3bl-org/r3bl-open-core/issues/321).
  - `SharedWriter` now has a constructor `new(...)` so that it is easier to create a new instance of
    it.

### v0.5.0 (2024-04-22)

- Changed:
  - Refactor `TracingConfig`` with better expression of display preference:
    - Require a `SharedWriter` for `TracingConfig` (no longer optional).
    - Fix log filename bug (now file extensions are used when supplied).
    - Redo `TracingConfig` with better expression of display preference (`stdin`, `stdout`,
      `SharedWriter`).
  - Update example `examples/terminal_async.rs` to use the `tracing_setup::init` and provide a real
    `TracingConfig` struct, which outputs logs to display ( `SharedWriter`) and file.

### v0.4.0 (2024-04-21)

- Changed:
  - Remove use of `TokioMutex`. There are some dangers to being "cancel safe" when using async Rust.
    This is outlined in the following:
    [docs](https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html), and
    [video](https://www.youtube.com/watch?v=1zOd52_tUWg&t=2088s). It is better to avoid using a
    `TokioMutex` to check for cancellation and instead to use broadcast channel for shutdown
    signals, just like the code already does. The changes made in this release are related to
    removing the use of `TokioMutex` all together in favor of the `StdMutex` since there is really
    no need to use it at all. And thus avoid any potential of "cancel safe" errors cropping up!

### v0.3.1 (2024-04-17)

- Updated:
  - Minor refactoring to remove the use of `JoinHandle::abort()` and replace it with a tokio
    broadcast channel to signal tasks to stop. This is a more graceful, flexible, and powerful way
    to stop tasks that are running in parallel. This change is applied to `spinner.rs` and
    `readline.rs`. The public API remains exactly the same.

### v0.3.0 (2024-04-15)

This is the first release of this crate.

- Added:
  - A new crate in this repo that allows for async terminal input and output. This is useful for
    building TUIs that are async and can handle input and output in parallel. To build apps that are
    not full TUI, this is a great option to create interactive CLIs and REPLs that are fully async
    and multithreaded (with input and output) with a really powerful (multi) line editor and prompt.

## `md_parser_ng`

### Archived (2025-07-22)

Experimental markdown parsers archived after performance analysis showed the legacy parser remains
the best option. The nom-based NG parser and Simple parser implementations have been moved to the
archive repository for educational purposes.

- Archived:
  - Moved experimental `md_parser_ng` (nom-based) and Simple parser implementations to
    [`r3bl-open-core-archive`](https://github.com/r3bl-org/r3bl-open-core-archive)
  - NG parser showed 600-5,000x performance degradation compared to legacy parser due to virtual
    array abstraction overhead
  - Simple parser achieved performance parity with legacy parser (within 25%) but lacked
    battle-tested reliability
  - Decision made to optimize existing legacy parser rather than replace it with experimental
    implementations
  - Archived parsers remain available as educational references for alternative parsing approaches

## `r3bl_core`

### Archived (2025-04-21)

This release has **lots** of major breaking changes. Lots of crates in the monorepo have of code.
The codebase has roughly 48K lines _not counting comment lines_. And about 25K lines of code have
been added, and 14K lines have been removed. been moved into this crate (and then been archived).
This is a major release that is part of a total reorganization of the `r3bl-open-core` repo. These
changes pay down all the technical debt accrued over the past 2 years of development.

Our goal is to ensure a clean and maintainable codebase that is easy to understand and easy to add
new features to in the future. And also a codebase that does not require extreme precision to use
correctly or maintain (eg: try to remove off by one errors by leveraging the type system
extensively).

We want the code to be very difficult to use incorrectly or modify incorrectly, by leveraging
compiler driven development and Rust's fantastic type system. Lots of powerful idioms and design
patterns are used in this release.

- It does have some major rewrites of existing functionality to be much faster and easier to use
  (focus on ergonomics, such as the `arg: impl Into<T>` where `T` is a struct pattern) which heavily
  leverages the
  ["newtype" design pattern / newtype](https://doc.rust-lang.org/rust-by-example/generics/new_types.html).
  The goal is to make this API difficult to use (as a whole) incorrectly. And trivial to use (as a
  whole) correctly.
- It contains changes that are part of optimizing memory allocation to increase performance, and
  ensure that performance is stable over time. `ch_unit.rs` is also rewritten and the entire
  codebase updated so that a the more ergonomic `ChUnit` API is now used throughout the codebase.
  Lots of missing test cases and documentation have been added to ensure that the code is stable and
  reliable over time.
- The entire codebase has been revamped to be strongly typed and no longer uses primitive types
  (like `usize` or `ChUnit`) to represent column index, row index, width, height, position, and size
  / dimension, using the "newtype" design pattern / idiom. This makes it difficult to use the API
  incorrectly. It also makes the entire API less error prone, relaxing to use, and much easier to
  maintain. New types and the `arg: impl Into<T>` where `T` is a struct is used in `Pos`,
  `RowIndex`, `ColIndex`, `Dim`, `ColWidth`, `RowHeight` types, and their aliases (`Width`,
  `Height`) and lots of helper functions `row()`, `col()`, `width()`, `height()`, and methods to
  convert between types and transform other types into these types.
- The `graphemes` module containing `UnicodeString` handling have been totally rewritten. They are
  now no allocation structures! In all past versions, the `UnicodeString` was a `String` under the
  hood. Now it is a `InlineString` which is a `smallstr` that initially allocates on the stack, and
  it can spill over into the heap if it needs to. The struct is now named `GCString`. The "newtype"
  design pattern / idiom and `arg: impl Into<T>` in `GCString` and other specific types are used,
  such as `SegIndex`, `SegWidth`, `ByteIndex`, and works with `ColWidth`, `ColIndex`, et al. It is
  quite difficult to use this API incorrectly!
- The `color_wheel` module now uses "newtype" design pattern / idiom and `arg: impl Into<T>` where
  `T` is a struct pattern. `bool` is replace with enums as well.
- A new telemetry API is also in this release, which makes it easy to measure and report performance
  metrics in a memory access and CPU performant way. This is built for the `r3bl_tui` main event
  loop (which is a very hot loop).
- `new_style!` (decl macro) replaces `tui_style!` (proc macro). `tui_color!` also replaces `color!`.
  You are not expected to work with `TuiStyle` or `TuiColor` directly. Instead, you are expected to
  work with these decl macros.
- `log_support` moves from `r3bl_log` (which never got published after getting created). This was
  meant to be the first release of this crate, but it is moved into `r3bl_core` now. It is a top
  level crate in the `r3bl-open-core` that is meant to hold all the logging related functionality
  for all the other crates in this monorepo. It uses `tracing` under the covers to provide
  structured logging. It also provides a custom formatter that is a `tracing-subscriber` crate
  plugin. This release contains changes that are part of optimizing memory allocation to increase
  performance, and ensure that performance is stable over time. `ch_unit.rs` is also heavily
  refactored and the entire codebase updated so that a the more ergonomic `ChUnit` API is now used
  throughout the codebase.

These videos have been an inspiration for many of these changes:

- [Data oriented design](https://youtu.be/WwkuAqObplU)
- [Memory alloc](https://youtu.be/pJ-FRRB5E84)
- [Compiler driven development](https://www.youtube.com/watch?v=_oaGNy3_798)

Here are the highlights:

- [PR](https://github.com/r3bl-org/r3bl-open-core/pull/370/commits/20fe5e730a0f592c203c85a68ee6e5b345136f44)
  1. Add a new declarative macro to effortlessly create global mutable thread safe singletons
     (without using `unsafe`).
  2. Replace all the ignored doc tests with `no_run` (just compile) or compile and run. For all Rust
     source files (in the entire monorepo, and not just this crate / folder).
- [PR](https://github.com/r3bl-org/r3bl-open-core/pull/376/commits/39bf421bb86d4de004bffd08f35df12ce3ef8541)
  1. There's a new converter `convert_to_ansi_color_styles` which converts a `TuiStyle` into a `Vec`
     of `r3bl_ansi_term::Style`.
  2. This is for `lolcat_api` enhancements which now allow for an optional default style to be
     passed in, that will be applied to the generated lolcat output.

- Moved:
  - Move the contents of `r3bl_ansi_color` crate into `r3bl_core`. There is no need to have that
    crate as an external dependency. Moving it where it belongs. It was developed as a separate
    crate at the start, since the `r3bl_tui` codebase was in a much earlier stage when it wasn't
    clear where `r3bl_ansi_term` fits with `r3bl_tui` and `r3bl_tuify`, etc. `term.rs` is now in
    `r3bl_core` where it belongs. This is where the functions to get the terminal window size and
    width belong, and whether the terminal is interactive or not. Terminal color detection
    capabilities and low level color output manipulation are now all in `r3bl_core`.
  - Move the contents of `r3bl_test_fixtures` into `r3bl_core`. This is a top level crate in the
    `r3bl-open-core` that is meant to hold all the test fixtures related functionality for all the
    other crates in this monorepo.
  - Move the contents of `r3bl_log` (all the tracing and logging functionality) in here.
  - Move the contents of the `r3bl_script` here. It is a top level crate in the `r3bl-open-core`
    that is meant to hold all the scripting related functionality for all the other crates in this
    monorepo. It provides a way to run scripts in a safe and secure way, that is meant to be a
    replacement for writing scripts in `fish` or `bash` or `nushell` syntax.

- Changed:
  - Consolidate the color structs from `r3bl_core` and `r3bl_ansi_color`, since `r3bl_ansi_color` is
    deprecated and its functionality has been moved into `r3bl_core`. The `ASTColor` and `TuiColor`
    structs have the same underpinning structs, which they're composed on top of. The reason for the
    distinction is just to make it clear the differences between TUI output and console output
    (directly to stdout or colorful log message) use cases. The conversion functions and traits are
    also consolidated in one place.
  - In `decl_macros/macros.rs` change the semantics of the `with!` macro, so that the `$id` is only
    contained to the `run` block and doesn't get added to the caller's scope / block.
  - Add new declarative macros to mimic `.join(..)` method on `Vec` and `String`. This makes it
    easier to join a collection of items into a string, or a collection of strings into a string
    with (`join!` & `join_with_index!`) or without allocations (`join_fmt!` &
    `join_with_index_fmt!`).
  - Add `read_from_file` module in the `into_existing.rs` file which allows for reading from a file
    in a memory efficient way.
  - Clean up the `Display` and `Debug` implementations for all the structs in this crate. The
    `PrettyPrintDebug` trait is still needed to get around the orphan rule, since type aliases are
    used for all the data structures for the MD parser; as they can't implement `Debug`, so they
    must implement `PrettyPrintDebug`. It is much easier to do this than to rewrite all these
    underlying data structures without type aliases! Type aliases make it easier to maintain the
    code and allow for easy and seamless addition for operator overloading and other niceties.
    Wherever possible the `Debug` trait does not perform any allocations and uses the `write!` macro
    to write to a pre-existing buffer.
  - The `graphemes` module containing `UnicodeString` handling have been totally rewritten. They are
    now optimized for memory latency (access, mutation, and allocation). For performance reasons
    they are not "no allocation" structures. `GCString` now owns a `InlineString` under the hood.
    The `UnicodeStringSegment`, now called `Seg`, does not own anything (no heap or string
    allocation, and is a very "scalarized" struct), and needs a `GCString` to be able to do
    anything. All the existing code that relies on this has been rewritten to accommodate this
    change. These changes are similar in spirit to the changes for `ColIndex`, `RowIndex`,
    `RowHeight`, `ColWidth`, `Dim`, `Pos`, etc and follow the same API style and principles.
  - Use "newtype" design pattern / idiom and `arg: impl Into<T>` where `T` is a struct pattern in
    the `color_wheel` module.
    - Replace all the `f64` types with wrapper structs like `Seed`, `Freq`, `SeedDelta`, `Spread`,
      and implement `AddAssign` ops between some of them. Use `arg: into Impl<T>` where `T` are
      these new (struct) types.
    - Replace `bool` with enum as well. Remove `unwrap()` calls in the constructor for the
      `ColorWheel` struct.
  - Fix all the Rust doc tests that were marked with `ignore`. Remove the `ignore` with either run
    and compile, or just `no_run` (compile only) in some cases where the code can't be run, but
    needs to be compiled.
  - Remove the `Serialize` and `Deserialize` derive macro from all the core data structs. The only
    struct that needs to be serialized and deserialized, is `kv.rs`. There is another struct in a
    different crate that needs this (in `r3bl_analytics` crate). There is no reason to have these
    derive macros in the `r3bl_core` crate. It is also an impedance to have no allocation structs,
    since `Deserialize` has some requirement for allocation.
  - Use the latest Rust 2024 edition.
  - Fix all the Rust docs that are ignored (in all Rust source files in this crate), and replace
    them with doc comments that compile successfully.
  - Replace the use of `bool` with meaningful enums to enhance code readability.

- Added:
  - Make the public API more ergonomic and use the `options: impl Into<TracingConfig>` pattern for
    all the functions that need to be configured. This makes it easy to define simple configuration
    options, while allowing for easy composition of more complex options. We really like this
    pattern and intend to refactor the entire codebase over time to use this.
  - [Archive](#archived-2025-03-11) the `tui_style!` proc macro. Replace it with an easier to use
    decl macro `new_style!`. This allows the `r3bl_macro` crate to be removed from the workspace,
    and all the crates in it. `new_style!` makes it a breeze to work with `TuiStyle` struct, so
    there is no need to manipulate it directly except if you need to.
  - Expand the functionality of `tui_color!` and rename it from `color!`. This macro makes it
    trivial to create different variants of `TuiColor` without having to work with the complex
    variants of the struct.
  - Add a new test fixture `temp_dir::create_temp_dir()` to make it easy to create temporary
    directories for tests. Any temporary directories created are automatically cleaned up after the
    test is done. The `TempDir` struct implements many traits that make it ergonomic to use with
    `std::fs`, `std::path` and `std::fmt`. Here are the PRs for this change:
    - [PR 1](https://github.com/r3bl-org/r3bl-open-core/pull/372)
    - [PR 2](https://github.com/r3bl-org/r3bl-open-core/pull/373)
  - New hashmap which remembers insertion order called `OrderedMap`.
  - New module `stack_alloc_types` that contain data structures that are allocated on the stack. If
    they grow too big, they are then moved to the heap. Under the covers, the `small_vec` and
    `small_str` crates are used. Lots of macros are provided to make it easy to work with these data
    structures (eg: `inline_string!`, `tiny_inline_string!`). And to make it easy to write into them
    without allocating them. Please note that if you make the
    `r3bl_ansi_color::sizing::DEFAULT_STRING_STORAGE_SIZE` number too large, eg: more than `16`,
    then it will slow down the TUI editor performance use.
  - `Percent` struct now has a `as_glyph()` method which displays the percentage as a Unicode glyph
    ranging from `STATS_25P_GLYPH` to `STATS_50P_GLYPH` to `STATS_75P_GLYPH` to `STATS_100P_GLYPH`
    depending on its value.
  - `Telemetry` is a struct that handles telemetry measurement & reporting. It is used in the
    `r3bl_tui` crate to measure and report the time it takes to render the TUI, and other
    performance metrics.
  - `TimeDuration` is a struct that handles efficient rendering of `Duration` in a human readable
    form, that does not allocate any memory (in the function that implements the `Display` trait).
  - `RateLimiter` is a struct that handles rate limiting for a given task. It is used to limit the
    rate at which a task can be executed. It is used in telemetry measurement & reporting for the
    `r3bl_tui` crate.
  - `RingBufferStack` and `RingBufferHeap` are structs that store a fixed number of elements in a
    ring buffer. One is allocated on the stack and the other on the heap. It is used in `Telemetry`
    measurement & reporting in the `r3bl_core` crate and in undo redo history in the
    `EditorBufferHistory` in the `r3bl_tui` crate.
  - Since `ch!` macro is removed, add new functions to replace it: `ch()`, `usize()`, `f64()`, etc.
    These functions provide better compiler type checking, better readability, better composability,
    and are much easier to use than the `ch!` macro.
  - `UnicodeString` now implements `std::fmt::Display`, so it is no longer necessary to use
    `UnicodeString.string` to get to the underlying string. This is just more ergonomic. This is
    added in `convert.rs`. More work needs to be done to introduce different types for holding
    "width / height" aka "column / row counts", and " column / row index". Currently there is a
    `Size` struct, and `Position` struct, but most of the APIs don't really use one or another, they
    just use `ChUnit` directly.
  - `lolcat_api` enhancements that now allow for an optional default style to be passed in to
    `ColorWheel::lolcat_into_string` and `ColorWheel::colorize_into_string`, that will be applied to
    the generated lolcat output.
  - `convert_to_ansi_color_styles` module that adds the ability to convert a `TuiStyle` into a `Vec`
    of `r3bl_ansi_term::Style`.
  - `string_helpers.rs` has new utility functions to check whether a given string contains any ANSI
    escape codes `contains_ansi_escape_sequence`. And to remove needless escaped `\"` characters
    from a string `remove_escaped_quotes`.
  - A new declarative macro `create_global_singleton!` that takes a struct (which must implement
    `Default` trait) and allows it to be simply turned into a singleton.
    - You can still use the struct directly. Or just use the supplied generated associated function
      on the struct called `get_mut_singleton()` and use the singleton directly. It does _NOT_ use
      `unsafe`.
    - The code is safe and uses `Arc<Mutex<T: Default>>`, where `T` is your struct, under the covers
      with `std::sync::Once` to make all this work.
    - Please take a look at the code itself for more details, and the docs have usage examples.
    - Another neat thing about this declarative macro is that it generates Rust docs for the
      generated code itself and these docs include references to the types and static variables that
      are generated.

- Removed:
  - Drop the dependency on `r3bl_ansi_color`.
  - Drop the dependency on `r3bl_log`.
  - Drop the dependency on `r3bl_script`.
  - Drop the dependency on `r3bl_test_fixtures`.
  - Remove `size-of` crate from `Cargo.toml`.
  - The `ch!` macro was confusing. It is now removed, and `ch_unit.rs` has clean conversions to and
    from other types. There are easy to use, typed checked functions, like `ch()`, `usize()`,
    `f64()`, etc.
  - Remove the following declarative macros that were not being used anywhere, and there are
    suitable candidates in the standard library that can be used instead:
    - `unwrap_option_or_run_fn_returning_err!`
    - `unwrap_option_or_compute_if_none!`

### v0.10.0 (2024-10-20)

This is a major release that does not include any new functionality, but is a radical reorganization
& rename of the crate, it used to be [`r3bl_rs_utils_core`](#rename-to-r3bl_core).

The `r3bl-open-core` repo was started in `2022-02-23`, about 2 years, 7 months ago, (which you can
get using `curl https://api.github.com/repos/r3bl-org/r3bl-open-core | jq .created_at`). We have
learned many lessons since then after writing about 125K lines of Rust code.

And it is time to pay down the accrued technical debt, to ensure that the codebase is easier to
maintain and understand, and easier to add new features to in the future. The separation of concerns
is now much clearer, and they reflect how the functionality is used in the real world.

- Removed:
  - Remove the dependency on `r3bl_simple_logger` and archive it. You can read the details in its
    [CHANGELOG entry](#archived-2024-09-27). Tokio tracing is now used under the covers.
  - Remove all the functions like `log_debug`, `log_info`, etc. and favor directly using tokio
    tracing macros for logging, eg: `tracing::debug!`, `tracing::info!`, etc.

- Changed:
  - `WriterConfig` can now be merged with other instances. This was a requirement for the
    `TracingConfig` to be able to merge multiple `WriterConfig` instances into a single
    `WriterConfig` instance. The code is in `src/log_support/public_api.rs` since this functionality
    is related to making the API easier to use by callers. This is in support of the
    `options: impl Into<TracingConfig>` pattern.
  - Two `TracingConfig` instances can be added together to create a new `TracingConfig` instance.
    This is needed for composability and an easy to use API for callers. Lots of converts are
    provide to make it easy to convert from a variety of configuration types into a `TracingConfig`
    instance. The code is in `src/log_support/public_api.rs`. This is in support of the
    `options: impl Into<TracingConfig>` pattern.
  - Rename the `debug!` macro, which is confusing, since it clashes with logging, to `console_log!`.
    This macro is used in many places in the codebase for quick formatted output to console (via
    `eprintln!`). The code to format the output is in the `console_log_impl.rs` file.
  - Reorganize the `src` folder to make sure that there aren't any top level files, and that
    everything is in a module. This is to make it easier to add new modules in the future.
  - The latest version of `unicode-width` crate `v2.0.0` changes the widths of many of the emoji.
    This requires lots of tests to be changed in order to work w/ the new constant width values.

- Added:
  - Simplify the actual logging API into a single function, and allow use of tokio tracing, macros
    for for logging, eg: `tracing::debug!`, `tracing::info!`, etc. See `logging_api.rs` for more
    details.
  - Move the `color_wheel` module into `r3bl_core` crate. This is to ensure that it is possible to
    import just color wheel and lolcat related functionality without having to import the entire
    `r3bl_tui` crate. And de-tangles the dependency tree, making it easier to maintain. The reason
    they ended up in `r3bl_tui` in the first place is because it was easier to develop them there,
    but since then, lots of other consumers of this functionality have emerged, including crates
    that are created by "3rd party developers" (people not R3BL and not part of `r3bl-open-core`
    repo).
  - Move the `kv.rs` module into `storage` from the `nazmulidris/rust-scratch/tcp-api-server` repo.
    This provides an in-memory / in-process key value store that is built on top of
    [`sled`](https://docs.rs/sled/latest/sled/). This eliminates the need to use files to save /
    load data.
  - Move the `miette_setup_global_report_handler.rs` from the
    [ `nazmulidris/rust-scratch/tcp-api-server`](https://github.com/nazmulidris/rust-scratch/) repo.
    This allows customization of the miette global report handler at the process level. Useful for
    apps that need to override the default report handler formatting.
  - Add `OutputDevice` that abstracts away the output device (eg: `stdout`, `stderr`,
    `SharedWriter`, etc.). This is useful for end to end testing, and adapting to a variety of
    different input and output devices (in the future). Support for this is provided in
    [`r3bl_test_fixtures`](#r3bl_test_fixtures).
  - Add `InputDevice` that abstracts away the input device (eg: `stdin`). This is useful for end to
    end testing. This is useful for end to end testing, and adapting to a variety of different input
    and output devices (in the future). Support for this is provided in
    [`r3bl_test_fixtures`](#r3bl_test_fixtures).
    - Moved:
  - Move some code from `r3bl_tuify`'s `term.rs` into `r3bl_core`. This module provides a simple API
    to detect the size of a terminal window and its width.

## `r3bl_tuify`

### Archived (2025-04-05)

This crate is now archived and is no longer maintained. All its functionality has been moved into
the `r3bl_terminal_async` crate.

### v0.2.0 (2024-10-21)

This is part of a total reorganization of the `r3bl-open-core` repo. This is a breaking change for
almost every crate in the repo. This [PR](https://github.com/r3bl-org/r3bl-open-core/pull/360)
contains all the changes.

- Updated:
  - Use the latest Rust 2024 edition.
  - This release just uses the latest deps from `r3bl-open-core` repo, since so many crates have
    been reorganized and renamed. The functionality has not changed at all, just the imports.

- Removed:
  - Drop the dependency on `r3bl_ansi_color`.
  - Move some of `term.rs` into:
    - `r3bl_core` - The functions to get the terminal window size and width.
    - `r3bl_ansi_color` - The functions to detect whether the current process is running in an
      interactive TTY or not.

### v0.1.27 (2024-09-12)

- Updated:
  - Upgrade all deps to their latest versions in `Cargo.toml` and `Cargo.lock`.
  - Improve docs in `lib.rs` and `README.md`.

### v0.1.26 (2024-04-15)

- Updated:
  - Make `clip_string_to_width_with_ellipsis` pub so that other crates can use it (eg:
    `r3bl_terminal_async`).
  - Change the names of enums to be more readable.
    - `IsTTYResult::IsTTY` -> `TTYResult::IsInteractive`.
    - `IsTTYResult::IsNotTTY` -> `TTYResult::IsNotInteractive`.
  - Using latest deps for `r3bl_rs_utils_core` version `0.9.13`, and `r3bl_rs_utils_macro` version
    `0.9.9`.

### v0.1.25 (2024-01-14)

- Updated:
  - Dependency updated `reedline` version `0.28.0`, `r3bl_rs_utils_core` version `0.9.12`.
- Added:
  - Add `tuify/src/constants.rs` with color constants.

### v0.1.24 (2023-12-31)

- Changed:
  - Rename `run.nu` to `run`. This simplifies commands to run it, eg: `nu run build`, or
    `./run build`.
  - Replace the `run` command with `examples` in the `run` nushell script. To run an example you use
    `nu run examples`. and provide instructions on the `run` script at the top level folder of this
    monorepo. Update `lib.rs` and `README.md` to reflect this change. The behavior of the `run`
    nushell script is more uniform across all crates in this repo.

- Added:
  - Add a new top level function `select_from_list_with_multi_line_header()` in `public_api.rs` to
    allow for multi-line headers in the list selection menu. This allows ANSI formatted strings to
    be used in each header line.

- Fixed:
  - In `select_from_list()`, the `max_width_col_count` is now respected to limit the max width of
    the terminal window that is used.

### v0.1.23 (2023-12-22)

- Updated:
  - Update dependency on `r3bl_rs_utils_core` to `0.9.10`.

### v0.1.22 (2023-12-20)

- Updated:
  - Update dependency on `reedline` crate to `0.27.1`.
  - Update dependency on `r3bl_rs_utils_core` to `0.9.9`.

- Removed:
  - Remove dependency on `r3bl_tui` crate.

- Changed:
  - Change the default theme so that it is better looking and more readable on Mac, Linux, and
    Windows. Add many different themes to choose from.

- Added:
  - `Ctrl + c` now behaves just like the `Escape` key. In the past, pressing `Ctrl + c` would do
    nothing the user could not exit the app by pressing this shortcut.
  - More code quality and ability to test the main event loop, by creating a new
    `TestVecKeyPressReader` struct, and abstracting the `read()` (from `stdin`) into a
    `KeyPressReader` trait. This is similar to what is done for `TestStringWriter` (to `stdout`).

### v0.1.21 (2023-10-21)

- Updated:
  - Upgrade all deps to their latest versions.

### v0.1.20 (2023-10-21)

- Updated:
  - Bug fix: <https://github.com/r3bl-org/r3bl-open-core/issues/170>

### v0.1.19 (2023-10-17)

- Updated:
  - Use the latest `r3bl_rs_utils_core` crate due to
    <https://rustsec.org/advisories/RUSTSEC-2021-0139.html>, and `ansi_term` not being maintained
    anymore.

### v0.1.18 (2023-10-17)

- Updated:
  - Use the latest `r3bl_rs_utils_core` crate due to
    <https://rustsec.org/advisories/RUSTSEC-2021-0139.html>, and `ansi_term` not being maintained
    anymore.

### v0.1.17 (2023-10-14)

- Updated:
  - Dependency on `simplelog` is replaced w/ `r3bl_simple_logger` (which is in the `r3bl_rs_utils`
    repo workspace as `simple_logger`).

## `r3bl_script`

### Archived (2025-03-31)

This crate never got published. And it was absorbed into `r3bl_core` and archived.

## `r3bl_log`

### Archived (2025-03-30)

This crate never got published. And it was absorbed into `r3bl_core` and archived.

## `r3bl_test_fixtures`

### Archived (2025-03-29)

This crate has been absorbed into `r3bl_core`. It is now archived. The copy in `r3bl_core` adds a
new fixture to make it easy to create temporary directories for tests.

- Updated:
  - Use the latest Rust 2024 edition.

### v0.1.0 (2024-10-21)

This is part of a total reorganization of the `r3bl-open-core` repo. This is a breaking change for
almost every crate in the repo. This [PR](https://github.com/r3bl-org/r3bl-open-core/pull/360)
contains all the changes.

- Changed:
  - Some type aliases were defined here redundantly, since they were also defined in `r3bl_core`
    crate. Remove these duplicate types and add a dependency to `r3bl_core` crate.

- Added:
  - Add ability for `StdoutMock` to be turned into an `OutputDevice` struct for mocks that are
    needed in tests. This is done via the `OutputDeviceExt` trait that is implemented for
    `OutputDevice`, which adds this method: `OutputDevice::new_mock()`.
  - Add ability for `mod@async_input_stream` to be turned into an `InputDevice` struct for mocks
    that are needed in tests. This is done via the `InputDeviceExt` trait that is implemented for
    `InputDevice`, which adds this method: `InputDevice::new_mock(vec![])`.

### v0.0.3 (2024-09-12)

- Updated:
  - Upgrade all deps to their latest versions in `Cargo.toml` and `Cargo.lock`.
  - Improve docs in `lib.rs` and `README.md`.

### v0.0.2 (2024-07-13)

This release adds a new function to create an async stream that yields results (from a vec) at a
specified interval. This is useful for testing async functions that need to simulate a stream of
events with a delay.

- Added:
  - Add `gen_input_stream_with_delay()` to create an async stream that yields results ( from a vec)
    at a specified interval. This is useful for testing async functions that need to simulate a
    stream of events with a delay.
  - Add more tests.

- Misc:
  - The [async_cancel_safe](https://github.com/nazmulidris/rust-scratch/tree/main/async_cancel_safe)
    example repo now uses this crate. Here's a
    [tutorial](https://developerlife.com/2024/07/10/rust-async-cancellation-safety-tokio/) about it.

### v0.0.1 (2024-07-12)

This is the first release of this crate. It is a top level crate in the `r3bl-open-core` that is
meant to hold all the test fixtures for all the other crates in this monorepo. It primarily tests
input events coming from user input via keyboard and mouse (eg: crossterm events). And it tests
output that is sent to `stdout` which it mocks. Here are all the links for this release:
[crates.io](https://crates.io/crates/r3bl_test_fixtures),
[docs.rs](https://docs.rs/r3bl_test_fixtures),
[GitHub](https://github.com/r3bl-org/r3bl-open-core/tree/main/test_fixtures).

- Added:
  - Add a new top level crate in the `r3bl-open-core` monorepo called `r3bl_test_fixtures`. This is
    to make it easier to maintain and test the fixtures and allow all the other crates in this
    monorepo to use them. These fixtures are migrated from `r3bl_terminal_async` crate, where they
    were gestated, before being graduated for use by the entire monorepo.

## `r3bl_ansi_color`

### Archived (2025-03-28)

Move the contents of `r3bl_ansi_term` crate into `r3bl_core`. There is no need to have that crate as
an external dependency. Moving it where it belongs. It was developed as a separate crate at the
start, since the `r3bl_tui` codebase was in a much earlier stage when it wasn't clear where
`r3bl_ansi_term` fits with `r3bl_tui` and `r3bl_tuify`, etc.

Here are the changes that were made before moving it to `r3bl_core` and archiving this repo.

This is a minor change that adds `vscode` to the list of environment variables that mean that
`truecolor` is supported. It removes duplication of `term.rs` in the `r3bl-open-core` repo and
workspace. The names are also cleaned up so there's no confusion about using `Color` or `Style`
which are so generic and used in many other crates.

- Updated:
  - Use the latest Rust 2024 edition.

- Added:
  - Support for inline (stack allocated) data structures (`InlineVecASTStyles` and
    `AnsiStyledText::to_small_str()`). Please note that if you make the
    `r3bl_ansi_color::sizing::DEFAULT_STRING_STORAGE_SIZE` number too large, eg: more than `16`,
    then it will slow down the TUI editor performance use.
  - Lots of new easy constructor functions to make it easy to colorize `&str` like `red("hello")`,
    `green("world")`, etc. This removes the requirement to depend on `crossterm::Stylize` for
    colorizing strings that are intended to be displayed in console stdout / stderr. Methods are
    provided to add background colors as well `AnsiStyledText::bg_dark_grey()`.
  - Easy constructor functions are provided `fg_rgb_color()` and `AnsiStyledText::bg_rgb_color()`.
    Together they allow easy integration with `tui_color!` and make it trivial to write code that
    uses styles / colors from the `r3bl_tui` crate and apply them to `AnsiStyledText` which is a
    very common pattern when colorizing log output.
  - Easy macro `rgb_value!` to create lots of beautiful colors with ease. This is similar to
    `r3bl_tui` crate's `tui_color!` macro.

- Removed:
  - `term.rs` is now in `r3bl_core`. Support for `$TERM_PROGRAM` = `vscode` to the list of
    environment variables that mean that `truecolor` is supported. This is in `check_ansi_color.rs`
    file.

- Changed:
  - `Color` is now `ASTColor`.
  - `Style` is now `ASTStyle`.
  - Proper converters `From` implementations are provided to convert between `ASTColor` and
    `RGBColor`, and `Ansi256Color`.

### v0.7.0 (2024-10-18)

This is part of a total reorganization of the `r3bl-open-core` repo. This is a breaking change for
almost every crate in the repo. This [PR](https://github.com/r3bl-org/r3bl-open-core/pull/360)
contains all the changes.

- Added:
  - Move code from `r3bl_core`'s `term.rs` to detect whether `stdin`, `stdout`, `stderr` is
    interactive. This has a dependency on the standard library, and not `crossterm` anymore. The API
    exposed here is ergonomic, and returns an `enum` rather than `bool`, which make it easier to use
    and understand.

### v0.6.10 (2024-09-12)

- Updated:
  - Upgrade all deps to their latest versions in `Cargo.toml` and `Cargo.lock`.
  - Improve docs in `lib.rs` and `README.md`.

### v0.6.9 (2023-10-21)

- Updated:
  - Upgrade all deps to their latest versions.

### v0.6.8 (2023-10-16)

- Added:
  - Support for `Grayscale` color output. This is in preparation of making the color support work
    across all platforms (MacOS, Linux, Windows). And use this in the `r3bl_tui` crate. Update tests
    to reflect this.

- Removed:
  - Dependency on `once-cell` removed by replacing `Arc<Mutex<_>>` with `unsafe` and `AtomicI8`.

### v0.6.7 (2023-09-12)

- Added:
  - Tests.

- Replaced:
  - `justfile` is now replaced with `nu` script `run.nu`.

## `r3bl_macro`

### Archived (2025-03-11)

The only purpose for having this crate is the requirements for Rust to have procedural macros be in
a separate crate. Proc macros also increase the build time. For this reason we have rewritten the
`tui_style!` macro as a declarative macro in the `r3bl_core` crate called `new_style!`. And are
archiving this crate.

This crate used to be called `r3bl_rs_utils_macro`. It was renamed to `r3bl_macro` in late 2024. And
in early 2025, it is being archived.

### v0.10.0 (2024-10-20)

This is a major release that does not include any new functionality, but is a radical reorganization
& rename of the crate, it used to be [`r3bl_rs_utils_macro`](#rename-to-r3bl_macro).

The `r3bl-open-core` repo was started in `2022-02-23`, about 1 year, 7 months, and 11 days ago,
(which you can get using
`curl https://api.github.com/repos/r3bl-org/r3bl-open-core | jq .created_at`). We have learned many
lessons since then after writing about 125K lines of Rust code.

And it is time to pay down the accrued technical debt, to ensure that the codebase is easier to
maintain and understand, and easier to add new features to in the future. The separation of concerns
is now much clearer, and they reflect how the functionality is used in the real world.

This [PR](https://github.com/r3bl-org/r3bl-open-core/pull/360) contains all the changes.

Changed:

- The name of this repo used to be [`r3bl_rs_utils_macro`](#rename-to-r3bl_macro).
- The modules and functions in this crate which are used (by other crates in this monorepo) are left
  unchanged. Only the unused modules and functions are moved to the
  [`r3bl-open-core-archive`](https://github.com/r3bl-open-core-archive) repo.

Deleted:

- Move all the unused modules and functions to the
  [`r3bl-open-core-archive`](https://github.com/r3bl-open-core-archive) repo.

## `r3bl_simple_logger`

### Archived (2024-09-27)

This crate has been moved into the
[r3bl-open-core-archive](https://github.com/r3bl-org/r3bl-open-core-archive) repo for archival
purposes. It is no longer maintained. This crate was only added to this mono repo since it had
become unmaintained. We now use tokio tracing, so this is no longer required.

### v0.1.4 (2024-09-12)

- Updated:
  - Upgrade all deps to their latest versions in `Cargo.toml` and `Cargo.lock`.
  - Improve docs in `lib.rs` and `README.md`.

### v0.1.3 (2023-10-21)

- Updated:
  - Upgrade all deps to their latest versions.

### v0.1.2 (2023-10-21)

- Updated:
  - Upgrade all deps to their latest versions.

### v0.1.1 (2023-10-17)

- Replaced:
  - Dependency on `ansi_term` is dropped due to this security advisory
    <https://rustsec.org/advisories/RUSTSEC-2021-0139.html>. Replaced with `r3bl_ansi_color`.

- Added:
  - Documentation for `r3bl_simple_logger` crate. And how to think about it vs. using log facilities
    from the `r3bl_rs_utils_core` crate. Update docs there too.

### v0.1.0 (2023-10-14)

- Added:
  - First changelog entry. This crate is a fork of the
    [`simplelog`](https://crates.io/crates/simplelog) repo w/ conditional compilation (feature
    flags) removed. This crate was causing transitive dependency issues in upstream repos that added
    `r3bl_tuify` as a dependency. Here's a link to the related
    [issue](https://github.com/r3bl-org/r3bl-open-core/issues/160).

## `r3bl_redux`

### Archived (2024-09-29)

This crate has been moved into the
[r3bl-open-core-archive](https://github.com/r3bl-org/r3bl-open-core-archive) repo for archival
purposes. It is no longer maintained. The redux pattern was removed from the TUI engine in 2024, in
favor of "Elm style" or "signals" based architecture.

### v0.2.8 (2024-09-12)

- Updated:
  - Upgrade all deps to their latest versions in `Cargo.toml` and `Cargo.lock`.
  - Improve docs in `lib.rs` and `README.md`.

### v0.2.7 (2024-09-07)

- Removed:
  - Remove `get-size` crate from `Cargo.toml`. This was causing some
    [issues with `RUSTSEC-2024-0370`](https://github.com/r3bl-org/r3bl-open-core/issues/359).

- Updated:
  - Use the latest deps for all crates in `Cargo.toml` and `Cargo.lock`.

### v0.2.6 (2023-10-21)

- Updated:
  - Upgrade all deps to their latest versions.

### v0.2.5 (2023-10-17)

- Updated:
  - Dependency on `r3bl_rs_utils_core` & `r3bl_rs_utils_macro` crates due to
    <https://rustsec.org/advisories/RUSTSEC-2021-0139.html>, and `ansi_term` not being maintained
    anymore.

### v0.2.4 (2023-10-14)

- Updated:
  - Dependency on `simplelog` is replaced w/ `r3bl_simple_logger` (which is in the `r3bl_rs_utils`
    repo workspace as `simple_logger`).

- Removed:
  - Dependency on `ansi_term` which is no longer maintained
    <https://rustsec.org/advisories/RUSTSEC-2021-0139.html>.
  - Needless dependencies on crates that are not used.

## `r3bl_rs_utils`

### Archived (2024-09-30)

This crate has been moved into the
[r3bl-open-core-archive](https://github.com/r3bl-org/r3bl-open-core-archive) repo for archival
purposes. It is no longer maintained.

### v0.9.16 (2024-09-12)

- Updated:
  - Upgrade all deps to their latest versions in `Cargo.toml` and `Cargo.lock`.
  - Improve docs in `lib.rs` and `README.md`.

### v0.9.15 (2023-12-22)

- Updated:
  - Add single dependency on `r3bl_rs_utils_core` version `0.9.10`.

- Removed:
  - Remove all the unnecessary dependencies from `Cargo.toml`.
  - Remove all unnecessary `dev-dependencies` from `Cargo.toml`.

- Moved:
  - All the source code from the top level folder of the `r3bl-open-core` repo into the `utils` sub
    folder. The crate `r3bl_rs_utils` used to reside at the top level folder of this repo. It has
    been moved into the `utils` sub folder. At the top level, only a workspace remains to link all
    the contained crates together for efficient builds.

### v0.9.14 (2023-10-29)

- Updated:
  - Upgrade all deps to their latest versions (including `r3bl_tui` w/ latest copy, paste, cut,
    delete support).

### v0.9.13 (2023-10-29)

- Updated:
  - Upgraded `r3bl_tui` to latest version.

### v0.9.12 (2023-10-29)

- Forgot to update the r3bl_tui dependency in Cargo.toml.

### v0.9.11 (2023-10-28)

- Updated:
  - Upgrade all deps to their latest versions.

### v0.9.10 (2023-10-21)

- Updated:
  - Upgrade all deps to their latest versions.

### v0.9.9

- Changes:
  - Use latest dependencies on the `r3bl_rs_utils` repo. Lots of needless dependencies have been
    dropped.
  - Drop `ansi_term` dependency due to security advisory
    <https://rustsec.org/advisories/RUSTSEC-2021-0139.html>.

<!-- changelog info section -->

# More info on changelogs

- https://keepachangelog.com/en/1.0.0/
- https://co-pilot.dev/changelog
