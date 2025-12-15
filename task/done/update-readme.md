<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Task: Update README Documentation (Root + tui/)](#task-update-readme-documentation-root--tui)
  - [Overview](#overview)
  - [Documentation Split Rationale](#documentation-split-rationale)
  - [Current State](#current-state)
    - [`tui/README.md` covers:](#tuireadmemd-covers)
    - [`tui/README.md` does NOT cover (to be added):](#tuireadmemd-does-not-cover-to-be-added)
    - [Root `README.md` currently has:](#root-readmemd-currently-has)
    - [Root `README.md` should add:](#root-readmemd-should-add)
- [Implementation Plan](#implementation-plan)
  - [Step 0: Research Phase [COMPLETE]](#step-0-research-phase-complete)
    - [Step 0.0: PTY Testing Infrastructure [COMPLETE]](#step-00-pty-testing-infrastructure-complete)
    - [Step 0.1: VT100 Parser - Input Side [COMPLETE]](#step-01-vt100-parser---input-side-complete)
    - [Step 0.2: VT100 Parser - Output Side [COMPLETE]](#step-02-vt100-parser---output-side-complete)
    - [Step 0.3: direct_to_ansi Backend [COMPLETE]](#step-03-direct_to_ansi-backend-complete)
    - [Step 0.4: rustix Raw Mode Implementation [COMPLETE]](#step-04-rustix-raw-mode-implementation-complete)
  - [Step 1: Update `tui/README.md` [COMPLETE]](#step-1-update-tuireadmemd-complete)
    - [Step 1.0: Add Platform-Specific Backends Section [COMPLETE]](#step-10-add-platform-specific-backends-section-complete)
    - [Step 1.1: Add VT100/ANSI Escape Sequence Handling Section [COMPLETE]](#step-11-add-vt100ansi-escape-sequence-handling-section-complete)
    - [Step 1.2: Add Raw Mode Implementation Section [COMPLETE]](#step-12-add-raw-mode-implementation-section-complete)
    - [Step 1.3: Add PTY Testing Infrastructure Section [COMPLETE]](#step-13-add-pty-testing-infrastructure-section-complete)
    - [Step 1.4: Update Table of Contents [COMPLETE]](#step-14-update-table-of-contents-complete)
  - [Step 2: Update Root `README.md` [COMPLETE]](#step-2-update-root-readmemd-complete)
    - [Step 2.0: Add PTY Testing Reference [COMPLETE]](#step-20-add-pty-testing-reference-complete)
    - [Step 2.1: Add Platform Backend Note [COMPLETE]](#step-21-add-platform-backend-note-complete)
  - [Step 3: Key Files to Reference [COMPLETE]](#step-3-key-files-to-reference-complete)
  - [Step 4: Style Guidelines [COMPLETE]](#step-4-style-guidelines-complete)
  - [Acceptance Criteria](#acceptance-criteria)
  - [Notes](#notes)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Task: Update README Documentation (Root + tui/)

## Overview

This task adds internal implementation documentation for contributors. The documentation is split
between two READMEs based on their purpose:

- **`tui/README.md`** - Library-specific documentation (architecture, internals, testing)
- **Root `README.md`** - Project-level cross-references and development workflow updates

## Documentation Split Rationale

| Topic                      | Primary Location | Why                                                       |
| -------------------------- | ---------------- | --------------------------------------------------------- |
| Platform-Specific Backends | `tui/README.md`  | TUI architecture decision affecting library usage         |
| VT100/ANSI Handling        | `tui/README.md`  | Core TUI implementation detail                            |
| Raw Mode Implementation    | `tui/README.md`  | TUI terminal mode management                              |
| PTY Testing Infrastructure | `tui/README.md`  | TUI-specific testing approach                             |
| Testing cross-reference    | Root `README.md` | Brief mention + link for contributor discoverability      |
| Platform support note      | Root `README.md` | Brief mention of direct_to_ansi in cross-platform section |

## Current State

### `tui/README.md` covers:

- High-level architecture and 6-stage rendering pipeline
- Component system, event routing, focus management
- Layout/styling with flexbox-like concepts
- Editor and dialog box components
- Message passing architecture

### `tui/README.md` does NOT cover (to be added):

- PTY testing infrastructure
- VT100/ANSI escape sequence parsing (input & output)
- The `direct_to_ansi` backend (Linux-specific)
- The rustix-based raw mode implementation (Linux-specific)

### Root `README.md` currently has:

- "Build the workspace and run tests" section
- "Cross-Platform Verification (Windows)" section

### Root `README.md` should add:

- Brief mention of PTY testing in the testing section (with link to tui/README)
- Brief mention of platform-specific backends in cross-platform section

---

# Implementation Plan

## Step 0: Research Phase [COMPLETE]

### Step 0.0: PTY Testing Infrastructure [COMPLETE]

- Explored `tui/src/core/test_fixtures/pty_test_fixtures/`
- Key file: `generate_pty_test.rs` - macro for PTY-based integration tests
- Understood controller/controlled architecture with env var routing
- Documented `spawn_controlled_in_pty()` for multi-backend comparison tests

### Step 0.1: VT100 Parser - Input Side [COMPLETE]

- Found `vt_100_terminal_input_parser` module - IO-free design
- Documented router → keyboard/mouse/terminal_events/utf8 architecture
- Explained IR conversion to InputEvent

### Step 0.2: VT100 Parser - Output Side [COMPLETE]

- Found `vt_100_pty_output_parser` module - VTE-based state machine
- Documented OffscreenBuffer::apply_ansi_bytes() flow
- Explained AnsiToOfsBufPerformer for terminal state updates

### Step 0.3: direct_to_ansi Backend [COMPLETE]

- Located `tui/src/tui/terminal_lib_backends/direct_to_ansi/`
- Documented input (Linux-only via mio) and output (cross-platform ANSI)
- Explained ~18% performance benefit over crossterm

### Step 0.4: rustix Raw Mode Implementation [COMPLETE]

- Found `core/ansi/terminal_raw_mode/raw_mode_unix.rs`
- Documented rustix type-safe termios API vs crossterm
- Explained ORIGINAL_TERMIOS static storage and /dev/tty fallback

---

## Step 1: Update `tui/README.md` [COMPLETE]

Added new sections after the existing "Rendering and painting" section.

### Step 1.0: Add Platform-Specific Backends Section [COMPLETE]

- Added backend selection table (Linux → DirectToAnsi, macOS/Windows → Crossterm)
- Documented crossterm and DirectToAnsi backends with platform support
- Added performance benefits (~18% improvement)
- Included architecture diagram showing Stage 5 backend selection

### Step 1.1: Add VT100/ANSI Escape Sequence Handling Section [COMPLETE]

- Added input parsing flow diagram with supported modules table
- Added output parsing flow for PTY child processes
- Documented key VT100 references (1-based coords, mouse scroll codes)
- Linked to source files for implementation details

### Step 1.2: Add Raw Mode Implementation Section [COMPLETE]

- Added raw mode vs cooked mode comparison table
- Documented rustix-based implementation with code example
- Listed rustix benefits over libc
- Added RawModeGuard usage example

### Step 1.3: Add PTY Testing Infrastructure Section [COMPLETE]

- Added architecture diagram (controller/controlled flow)
- Documented `generate_pty_test!` macro usage
- Listed when to use macro vs `spawn_controlled_in_pty()`
- Added running commands and example links

### Step 1.4: Update Table of Contents [COMPLETE]

- Added 22 new TOC entries for all new sections and subsections
- Formatted with prettier

---

## Step 2: Update Root `README.md` [COMPLETE]

### Step 2.0: Add PTY Testing Reference [COMPLETE]

Added after "Key Commands" table (line 781-783):

```markdown
> **TUI Testing**: The `r3bl_tui` crate uses PTY-based testing for accurate terminal I/O
> verification. See the [PTY Testing Infrastructure](./tui/README.md#pty-testing-infrastructure)
> section in the TUI README for details on writing and running TUI tests.
```

### Step 2.1: Add Platform Backend Note [COMPLETE]

Added after "Cross-Platform Verification" section (line 1340-1343):

```markdown
> **Platform Backends**: The TUI crate supports multiple backends: `Crossterm` (cross-platform,
> default on macOS/Windows) and `DirectToAnsi` (Linux-native, ~18% better performance). The
> metadata-only verification ensures platform cfg gates work correctly for both backends. See
> [Platform-Specific Backends](./tui/README.md#platform-specific-backends) for details.
```

---

## Step 3: Key Files to Reference [COMPLETE]

Confirmed actual paths:

```
tui/src/core/test_fixtures/pty_test_fixtures/
├── generate_pty_test.rs        # PTY test macro
├── spawn_controlled_in_pty.rs  # Multi-backend comparison helper
├── mod.rs                      # Module exports
└── ...

tui/src/core/ansi/
├── vt_100_terminal_input_parser/  # Input parsing
│   ├── mod.rs                     # Main documentation
│   ├── router.rs                  # Entry point
│   ├── keyboard.rs, mouse.rs, terminal_events.rs, utf8.rs
│   └── integration_tests/         # PTY-based tests
├── vt_100_pty_output_parser/      # Output parsing
│   ├── mod.rs
│   └── performer.rs               # VTE implementation
└── terminal_raw_mode/             # Raw mode
    ├── mod.rs                     # Comprehensive docs
    ├── raw_mode_unix.rs           # rustix implementation
    └── raw_mode_core.rs           # RawModeGuard

tui/src/tui/terminal_lib_backends/
├── mod.rs                      # Pipeline architecture
├── backend_selection.rs        # TERMINAL_LIB_BACKEND constant
├── direct_to_ansi/             # Linux-native backend
│   ├── mod.rs
│   ├── input/                  # Async stdin (Linux only)
│   └── output/                 # ANSI generation
└── crossterm_backend/          # Cross-platform backend
```

---

## Step 4: Style Guidelines [COMPLETE]

Applied:

- ✅ ASCII diagrams for architecture visualization
- ✅ Tables for comparison (backend selection, raw mode vs cooked mode)
- ✅ Code examples (rustix raw mode, generate_pty_test! macro)
- ✅ Links to source files using relative paths
- ✅ Consistent heading levels (## main, ### sub)
- ✅ Inverted pyramid structure (overview → details)

---

## Acceptance Criteria

- [x] All four topics documented in `tui/README.md`
- [x] `tui/README.md` Table of Contents updated with new sections
- [x] Root `README.md` has PTY testing reference in testing section
- [x] Root `README.md` has platform backend note in cross-platform section
- [x] Code examples included where helpful
- [x] Key source files referenced with correct paths
- [x] Documentation reviewed for accuracy against code
- [x] Both READMEs pass doctoc/prettier formatting

## Notes

- ~~The `tui/README.md` mentions "Direct ANSI" at line 743 but provides no details~~ → Now
  documented
- PTY testing is critical for CI/CD reliability of TUI apps
- These docs help new contributors understand the internals
- Root README changes are minimal - just cross-references for discoverability
