# Task: Remove Crossterm on macOS & Windows

<!-- BEGIN mktoc -->

- [Task: Remove Crossterm on macOS & Windows](#task-remove-crossterm-on-macos--windows)
- [Overview](#overview)
    - [Prerequisite: Linux DirectToAnsi Complete](#prerequisite-linux-directtoansi-complete)
- [Implementation Plan](#implementation-plan)
- [Step 1: macOS Native Drivers & Validation [PENDING]](#step-1-macos-native-drivers--validation-pending)
- [Step 2: Windows Native Drivers & Validation [PENDING]](#step-2-windows-native-drivers--validation-pending)
- [Step 3: Resolve TODOs and Stubs [PENDING]](#step-3-resolve-todos-and-stubs-pending)
- [Step 4: Crossterm Dependency Removal [PENDING]](#step-4-crossterm-dependency-removal-pending)
- [Implementation Checklist](#implementation-checklist)

<!-- END mktoc -->

## Overview

Linux `DirectToAnsi` (both the `RenderOp` output pipeline and the `mio`-based `DirectToAnsiInputDevice` with `vt_100_terminal_input_parser`) is complete and in production. However, macOS and Windows currently still fall back to `crossterm` for terminal input and raw mode.

**Objective**: Implement native non-Crossterm input and raw-mode drivers for macOS and Windows, validate cross-platform parity across the test fleet, and remove `crossterm` from `tui/Cargo.toml`.

### Prerequisite: Linux DirectToAnsi Complete

See [task_remove_crossterm_linux.md](done/task_remove_crossterm_linux.md) for the completed Steps 0–8 architecture:
- Unified `RenderOp` output pipeline and `PixelCharRenderer` (18% faster than Crossterm)
- Pure-Rust `vt_100_terminal_input_parser` (`ChunkFramer`, `ChunkDecoder`, `CircuitBreaker`, `MaybeMore`)
- Linux `DirectToAnsiInputDevice` backed by `RRT<MioPollWorker>`

---

## Implementation Plan

## Step 1: macOS Native Drivers & Validation [PENDING]

- [ ] Implement Darwin-compatible tty poller:
    - Darwin's `kqueue(2)` (used by `mio`) fails with `EINVAL` on `/dev/tty` and PTY file descriptors.
    - Implement a `select(2)` / `poll(2)` polling driver (similar to `filedescriptor`) integrated with the `RRT` worker trait.
- [ ] Implement `SIGWINCH` signal delivery via `signal-hook` with self-pipe trick.
- [ ] Validate `DirectToAnsi` rendering operations on macOS fleet.
- [ ] Validate `DirectToAnsi` input handling on macOS fleet.

## Step 2: Windows Native Drivers & Validation [PENDING]

- [ ] Implement native Windows Console input driver:
    - Windows lacks `/dev/tty` and POSIX `termios`.
    - Implement Win32 console reader using `ReadConsoleInputW` / ConPTY virtual terminal input processing (`ENABLE_VIRTUAL_TERMINAL_INPUT`) integrated with `RRT`.
- [ ] Implement Windows console raw mode toggling via `GetConsoleMode` / `SetConsoleMode`.
- [ ] Validate `DirectToAnsi` rendering operations on Windows fleet.
- [ ] Validate `DirectToAnsi` input handling on Windows fleet.

## Step 3: Resolve TODOs and Stubs [PENDING]

**Objective**: Sweep the codebase for incomplete implementations and TODO markers as final validation before `crossterm` removal.

- [ ] Search for `TODO:` comments related to `DirectToAnsi` / `RenderOp`
- [ ] Search for `FIXME:` comments in input/output paths
- [ ] Search for `unimplemented!()` calls in render pipeline
- [ ] Review all stub functions in `DirectToAnsi` backend and either implement or remove each stub
- [ ] Verify no lingering `crossterm` references in `DirectToAnsi` code paths

## Step 4: Crossterm Dependency Removal [PENDING]

- [ ] Verify zero remaining `crossterm` usages in the codebase.
- [ ] Remove `crossterm` dependency from `tui/Cargo.toml`.
- [ ] Remove `TerminalLibBackend::Crossterm` and `crossterm_backend/`.
- [ ] Update documentation and crate architecture diagrams.
- [ ] Final validation across all platforms (`/test-cross-platform` + `./check.fish --full`).
- [ ] Mandatory manual review of all modified files.

## Implementation Checklist

- [ ] Step 1: macOS native drivers & fleet validation
- [ ] Step 2: Windows native drivers & fleet validation
- [ ] Step 3: Resolve TODOs and stubs
- [ ] Step 4: Remove `crossterm` dependency from `tui/Cargo.toml`
