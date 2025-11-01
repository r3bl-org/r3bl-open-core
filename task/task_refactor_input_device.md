# InputDevice Architecture Refactoring

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Overview](#overview)
  - [Task Description](#task-description)
  - [Current State (Context)](#current-state-context)
  - [Goals](#goals)
  - [Architecture Before vs After](#architecture-before-vs-after)
    - [Before](#before)
    - [After](#after)
- [Implementation plan](#implementation-plan)
  - [Step 1: File Reorganization [PENDING]](#step-1-file-reorganization-pending)
    - [Step 1.1: Move `input_device_ext.rs` [PENDING]](#step-11-move-input_device_extrs-pending)
      - [Step 1.1.1: Actions](#step-111-actions)
    - [Step 1.2: Move and rename Crossterm implementation [PENDING]](#step-12-move-and-rename-crossterm-implementation-pending)
      - [Step 1.2.1: Actions](#step-121-actions)
  - [Step 2: Create New Components [PENDING]](#step-2-create-new-components-pending)
    - [Step 2.1: Create `MockInputDevice` [PENDING]](#step-21-create-mockinputdevice-pending)
      - [Step 2.1.1: File content](#step-211-file-content)
      - [Step 2.1.2: Module updates](#step-212-module-updates)
    - [Step 2.2: Create Generic `InputDevice` Enum [PENDING]](#step-22-create-generic-inputdevice-enum-pending)
      - [Step 2.2.1: File content](#step-221-file-content)
  - [Step 3: Update Existing Components [PENDING]](#step-3-update-existing-components-pending)
    - [Step 3.1: Update `CrosstermInputDevice` Implementation [PENDING]](#step-31-update-crossterminputdevice-implementation-pending)
      - [Step 3.1.1: Changes](#step-311-changes)
      - [Step 3.1.2: Implementation code](#step-312-implementation-code)
    - [Step 3.2: Update `DirectToAnsiInputDevice` [PENDING]](#step-32-update-directtoansiinputdevice-pending)
      - [Step 3.2.1: Changes](#step-321-changes)
    - [Step 3.3: Update `InputDeviceExtMock` Trait [PENDING]](#step-33-update-inputdeviceextmock-trait-pending)
      - [Step 3.3.1: Updated implementation](#step-331-updated-implementation)
  - [Step 4: Update Module Exports [PENDING]](#step-4-update-module-exports-pending)
    - [Step 4.1: Update `tui/src/core/terminal_io/mod.rs` [PENDING]](#step-41-update-tuisrccoreterminal_iomodrs-pending)
    - [Step 4.2: Update `tui/src/tui/terminal_lib_backends/crossterm_backend/mod.rs` [PENDING]](#step-42-update-tuisrctuiterminal_lib_backendscrossterm_backendmodrs-pending)
    - [Step 4.3: Update `tui/src/tui/terminal_lib_backends/mod.rs` [PENDING]](#step-43-update-tuisrctuiterminal_lib_backendsmodrs-pending)
    - [Step 4.4: Update `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mod.rs` [PENDING]](#step-44-update-tuisrctuiterminal_lib_backendsdirect_to_ansiinputmodrs-pending)
  - [Step 5: Update Import Sites [PENDING]](#step-5-update-import-sites-pending)
    - [Step 5.1: Search for imports to update](#step-51-search-for-imports-to-update)
    - [Step 5.2: Files likely needing updates](#step-52-files-likely-needing-updates)
    - [Step 5.3: Search commands](#step-53-search-commands)
- [Testing Strategy](#testing-strategy)
  - [Automated Testing](#automated-testing)
  - [Manual Testing](#manual-testing)
- [Rollback Plan](#rollback-plan)
- [Definition of Done](#definition-of-done)
- [Benefits Summary](#benefits-summary)
- [Risks and Mitigations](#risks-and-mitigations)
- [Notes](#notes)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Overview

## Task Description

Refactor the input device architecture to mirror the `OutputDevice` pattern using enum dispatch,
with automatic backend selection and mock support. This achieves architectural symmetry between
input and output layers while maintaining zero external dependencies.

## Current State (Context)

Currently, the input device architecture has files in the wrong locations:

- `input_device.rs` is in `core/terminal_io/` but contains Crossterm-specific implementation
- `input_device_ext.rs` is in `terminal_lib_backends/` but defines the generic trait

This refactoring reorganizes the architecture to match the output layer:

- Generic wrapper in `core/terminal_io/`
- Backend implementations in `terminal_lib_backends/`
- Trait definition in `core/terminal_io/`

## Goals

1. [COMPLETE] Perfect architectural symmetry with OutputDevice
2. [COMPLETE] Zero new dependencies (enum dispatch, no `async-trait` or `Pin<Box<Future>>`)
3. [COMPLETE] Automatic backend selection via `TERMINAL_LIB_BACKEND`
4. [COMPLETE] Backward compatible test API (`new_mock` still works)
5. [COMPLETE] Clean separation: trait in core/, impls in backends/
6. [COMPLETE] Easy to add new backends in future

## Architecture Before vs After

### Before

```
core/terminal_io/
  └── input_device.rs          [CrosstermInputDevice implementation]
terminal_lib_backends/
  ├── input_device_ext.rs      [InputDeviceExt trait]
  ├── crossterm_backend/
  └── direct_to_ansi/
      └── input/
          └── input_device_impl.rs  [DirectToAnsiInputDevice]
```

### After

```
core/terminal_io/
  ├── input_device.rs          [Generic InputDevice enum wrapper]
  └── input_device_ext.rs      [InputDeviceExt trait - MOVED]
terminal_lib_backends/
  ├── crossterm_backend/
  │   └── input_device_impl.rs  [CrosstermInputDevice - MOVED & RENAMED]
  └── direct_to_ansi/
      └── input/
          └── input_device_impl.rs  [DirectToAnsiInputDevice - unchanged]
core/test_fixtures/input_device_fixtures/
  ├── mock_input_device.rs     [MockInputDevice - NEW]
  └── input_device_ext_mock.rs [Updated to use enum]
```

# Implementation plan

## Step 1: File Reorganization [PENDING]

This step reorganizes files to match the OutputDevice pattern, moving files to their proper
locations and preparing the codebase for the new enum-based dispatch system.

### Step 1.1: Move `input_device_ext.rs` [PENDING]

Move the trait definition from the backend layer to the core protocol layer.

**Source:** `tui/src/tui/terminal_lib_backends/input_device_ext.rs`

**Destination:** `tui/src/core/terminal_io/input_device_ext.rs`

**Rationale:** Trait definitions belong in the protocol layer (core), not backend layer.

#### Step 1.1.1: Actions

- Move file
- Update module declarations in both `mod.rs` files
- Keep trait simple with `async fn` (enum dispatch = no object safety needed)
- Remove `#[allow(async_fn_in_trait)]` if present

### Step 1.2: Move and rename Crossterm implementation [PENDING]

Move the Crossterm input device implementation to the Crossterm backend folder and rename the struct
for clarity.

**Source:** `tui/src/core/terminal_io/input_device.rs`

**Destination:** `tui/src/tui/terminal_lib_backends/crossterm_backend/input_device_impl.rs`

**Struct Rename:** `InputDevice` → `CrosstermInputDevice`

**Rationale:** Backend implementation belongs with other Crossterm code.

#### Step 1.2.1: Actions

- Create new file at destination
- Copy existing struct and impl
- Rename struct to `CrosstermInputDevice`
- Rename constructor to `new_event_stream()`
- Move existing `InputDeviceExt` impl (lines 65-99)
- Delete old file after verification

## Step 2: Create New Components [PENDING]

Create the generic enum wrapper and mock device to support the new architecture.

### Step 2.1: Create `MockInputDevice` [PENDING]

Implement a mock input device for testing purposes.

**File:** `tui/src/core/test_fixtures/input_device_fixtures/mock_input_device.rs` (NEW)

#### Step 2.1.1: File content

````rust
// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{
    gen_input_stream, gen_input_stream_with_delay, CrosstermEventResult,
    InlineVec, InputDeviceExt, InputEvent, PinnedInputStream,
};
use futures_util::{FutureExt, StreamExt};
use std::time::Duration;

/// Mock input device for testing that yields synthetic events from a vector.
///
/// Used by integration tests and unit tests to simulate user input without
/// requiring actual terminal interaction.
///
/// ## Examples
///
/// ```no_run
/// use r3bl_tui::{MockInputDevice, InputDeviceExt};
/// use smallvec::smallvec;
/// use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
///
/// let events = smallvec![
///     Ok(Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE))),
///     Ok(Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))),
/// ];
/// let mut device = MockInputDevice::new(events);
/// ```
pub struct MockInputDevice {
    resource: PinnedInputStream<CrosstermEventResult>,
}

impl MockInputDevice {
    /// Create a new mock input device that yields events from the given vector.
    pub fn new(generator_vec: InlineVec<CrosstermEventResult>) -> Self {
        Self {
            resource: gen_input_stream(generator_vec),
        }
    }

    /// Create a new mock input device with a delay between events.
    ///
    /// Useful for testing timing-sensitive behavior or simulating realistic
    /// user input speed.
    pub fn new_with_delay(
        generator_vec: InlineVec<CrosstermEventResult>,
        delay: Duration,
    ) -> Self {
        Self {
            resource: gen_input_stream_with_delay(generator_vec, delay),
        }
    }
}

impl InputDeviceExt for MockInputDevice {
    async fn next_input_event(&mut self) -> Option<InputEvent> {
        loop {
            let maybe_result_event = self.resource.next().fuse().await;
            match maybe_result_event {
                Some(Ok(event)) => {
                    let input_event = InputEvent::try_from(event);
                    if let Ok(input_event) = input_event {
                        return Some(input_event);
                    }
                    // Conversion errors are expected (filtered events)
                    // Continue reading next event
                }
                Some(Err(e)) => {
                    tracing::error!(
                        message = "Error reading mock input event.",
                        error = ?e,
                    );
                    return None;
                }
                None => return None,
            }
        }
    }
}
````

#### Step 2.1.2: Module updates

Update `tui/src/core/test_fixtures/input_device_fixtures/mod.rs`:

```rust
pub mod mock_input_device;
pub use mock_input_device::*;
```

### Step 2.2: Create Generic `InputDevice` Enum [PENDING]

Replace the existing `input_device.rs` with a generic enum wrapper.

**File:** `tui/src/core/terminal_io/input_device.rs` (REPLACE EXISTING)

#### Step 2.2.1: File content

````rust
// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{
    CrosstermEventResult, CrosstermInputDevice, DirectToAnsiInputDevice,
    InlineVec, InputDeviceExt, InputEvent, MockInputDevice,
    TERMINAL_LIB_BACKEND, TerminalLibBackend,
};
use std::time::Duration;

/// Generic input device wrapper that abstracts over different backend implementations.
///
/// Provides a unified interface for reading terminal input events, similar to how
/// [`crate::OutputDevice`] abstracts over different output backends.
///
/// ## Architecture
///
/// Uses an enum to dispatch to the appropriate backend implementation at runtime:
/// - **Crossterm**: Cross-platform terminal input (default on non-Linux)
/// - **DirectToAnsi**: Pure Rust async input with tokio (default on Linux)
/// - **Mock**: Synthetic event generator for testing
///
/// Backend selection is automatic based on [`TERMINAL_LIB_BACKEND`], or can be
/// explicitly chosen via `new_crossterm()` / `new_direct_to_ansi()`.
///
/// ## Examples
///
/// ### Auto-select backend based on platform
/// ```no_run
/// use r3bl_tui::InputDevice;
///
/// let mut device = InputDevice::new();
/// while let Some(event) = device.next_input_event().await {
///     // Process event
/// }
/// ```
///
/// ### Explicitly choose backend
/// ```no_run
/// use r3bl_tui::InputDevice;
///
/// let mut device = InputDevice::new_crossterm();
/// ```
///
/// ### Mock for testing
/// ```no_run
/// use r3bl_tui::InputDevice;
/// use smallvec::smallvec;
/// use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
///
/// let events = smallvec![
///     Ok(Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE))),
/// ];
/// let mut device = InputDevice::new_mock(events);
/// ```
#[derive(Debug)]
pub enum InputDevice {
    /// Crossterm backend - cross-platform terminal input
    Crossterm(CrosstermInputDevice),
    /// DirectToAnsi backend - pure Rust async I/O
    DirectToAnsi(DirectToAnsiInputDevice),
    /// Mock backend - synthetic events for testing
    Mock(MockInputDevice),
}

impl InputDevice {
    /// Create a new InputDevice using the platform-default backend.
    ///
    /// - Linux: DirectToAnsi (pure Rust async I/O)
    /// - Others: Crossterm (cross-platform compatibility)
    ///
    /// Backend is selected via [`TERMINAL_LIB_BACKEND`] constant.
    #[must_use]
    pub fn new() -> Self {
        match TERMINAL_LIB_BACKEND {
            TerminalLibBackend::Crossterm => Self::new_crossterm(),
            TerminalLibBackend::DirectToAnsi => Self::new_direct_to_ansi(),
        }
    }

    /// Create a new InputDevice using the Crossterm backend explicitly.
    #[must_use]
    pub fn new_crossterm() -> Self {
        Self::Crossterm(CrosstermInputDevice::new_event_stream())
    }

    /// Create a new InputDevice using the DirectToAnsi backend explicitly.
    #[must_use]
    pub fn new_direct_to_ansi() -> Self {
        Self::DirectToAnsi(DirectToAnsiInputDevice::new())
    }

    /// Create a new mock InputDevice for testing.
    ///
    /// Events are yielded from the provided vector in order.
    #[must_use]
    pub fn new_mock(generator_vec: InlineVec<CrosstermEventResult>) -> Self {
        Self::Mock(MockInputDevice::new(generator_vec))
    }

    /// Create a new mock InputDevice with a delay between events.
    ///
    /// Useful for testing timing-sensitive behavior.
    #[must_use]
    pub fn new_mock_with_delay(
        generator_vec: InlineVec<CrosstermEventResult>,
        delay: Duration,
    ) -> Self {
        Self::Mock(MockInputDevice::new_with_delay(generator_vec, delay))
    }

    /// Read the next input event asynchronously.
    ///
    /// Returns `None` if the input stream is closed or encounters an error.
    ///
    /// ## Implementation
    ///
    /// Dispatches to the appropriate backend's `next_input_event()` implementation
    /// via the [`InputDeviceExt`] trait.
    pub async fn next_input_event(&mut self) -> Option<InputEvent> {
        match self {
            Self::Crossterm(device) => device.next_input_event().await,
            Self::DirectToAnsi(device) => device.next_input_event().await,
            Self::Mock(device) => device.next_input_event().await,
        }
    }

    /// Check if this is a mock device (for testing).
    ///
    /// This field exists for API symmetry with [`crate::OutputDevice`].
    #[must_use]
    pub fn is_mock(&self) -> bool {
        matches!(self, Self::Mock(_))
    }
}

impl Default for InputDevice {
    fn default() -> Self {
        Self::new()
    }
}
````

## Step 3: Update Existing Components [PENDING]

Update the implementations of each backend to work with the new architecture.

### Step 3.1: Update `CrosstermInputDevice` Implementation [PENDING]

Rename the struct and move the `InputDeviceExt` implementation to the new location.

**File:** `tui/src/tui/terminal_lib_backends/crossterm_backend/input_device_impl.rs`

#### Step 3.1.1: Changes

- Rename struct from `InputDevice` to `CrosstermInputDevice`
- Rename `new_event_stream()` constructor (keep existing body)
- Move `InputDeviceExt` impl from `input_device_ext.rs` to this file
- Update module re-exports in `crossterm_backend/mod.rs`

#### Step 3.1.2: Implementation code

```rust
// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{CrosstermEventResult, InputDeviceExt, InputEvent, PinnedInputStream, DEBUG_TUI_SHOW_TERMINAL_BACKEND};
use crossterm::event::EventStream;
use futures_util::{FutureExt, StreamExt};
use miette::IntoDiagnostic;

/// Crossterm-based input device implementation.
///
/// Uses `crossterm::event::EventStream` for async terminal input reading.
#[allow(missing_debug_implementations)]
pub struct CrosstermInputDevice {
    pub resource: PinnedInputStream<CrosstermEventResult>,
}

impl CrosstermInputDevice {
    /// Create a new Crossterm input device with an event stream.
    #[must_use]
    pub fn new_event_stream() -> Self {
        Self {
            resource: Box::pin(EventStream::new()),
        }
    }

    /// Get the next raw crossterm event (used internally).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The input event stream has been closed
    /// - An I/O error occurs while reading input
    /// - The terminal is not available
    pub async fn next(&mut self) -> miette::Result<crossterm::event::Event> {
        match self.resource.next().fuse().await {
            Some(it) => it.into_diagnostic(),
            None => miette::bail!("Failed to get next event from input source."),
        }
    }
}

impl InputDeviceExt for CrosstermInputDevice {
    async fn next_input_event(&mut self) -> Option<InputEvent> {
        loop {
            let maybe_result_event = self.next().fuse().await;
            match maybe_result_event {
                Ok(event) => {
                    let input_event = InputEvent::try_from(event);
                    if let Ok(input_event) = input_event {
                        return Some(input_event);
                    }
                    // Conversion errors are expected in the following cases:
                    // 1. Key Release/Repeat events (filtered in InputEvent::try_from).
                    // 2. Paste events (not supported).
                    //
                    // These are normal occurrences, not bugs. We simply continue
                    // reading the next event. The TryFrom implementations handle
                    // all expected cases by returning Err(()), so we don't need
                    // to panic or log errors here.
                }
                Err(e) => {
                    DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                        tracing::error!(
                            message = "Error reading input event.",
                            error = ?e,
                        );
                    });
                    return None;
                }
            }
        }
    }
}
```

### Step 3.2: Update `DirectToAnsiInputDevice` [PENDING]

Add `InputDeviceExt` trait implementation and create a constant for the read buffer size.

**File:** `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/input_device_impl.rs`

#### Step 3.2.1: Changes

1. Add constant at top of file:

   ```rust
   /// Temporary read buffer size for stdin reads.
   const TEMP_READ_BUFFER_SIZE: usize = 256;
   ```

2. Replace hardcoded 256 in `read_event()` (line 160):

   ```rust
   let mut temp_buf = vec![0u8; TEMP_READ_BUFFER_SIZE];
   ```

3. Add `InputDeviceExt` implementation:
   ```rust
   impl InputDeviceExt for DirectToAnsiInputDevice {
       async fn next_input_event(&mut self) -> Option<InputEvent> {
           self.read_event().await
       }
   }
   ```

### Step 3.3: Update `InputDeviceExtMock` Trait [PENDING]

Update the mock trait implementation to delegate to enum constructors.

**File:** `tui/src/core/test_fixtures/input_device_fixtures/input_device_ext_mock.rs`

#### Step 3.3.1: Updated implementation

```rust
// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{CrosstermEventResult, InlineVec, InputDevice};
use std::time::Duration;

/// Extension trait for creating mock InputDevice instances for testing.
///
/// This trait provides a backward-compatible API for existing tests.
/// Internally, it delegates to the `InputDevice` enum's mock constructors.
pub trait InputDeviceExtMock {
    fn new_mock(generator_vec: InlineVec<CrosstermEventResult>) -> InputDevice;

    fn new_mock_with_delay(
        generator_vec: InlineVec<CrosstermEventResult>,
        delay: Duration,
    ) -> InputDevice;
}

impl InputDeviceExtMock for InputDevice {
    fn new_mock(generator_vec: InlineVec<CrosstermEventResult>) -> InputDevice {
        InputDevice::new_mock(generator_vec)
    }

    fn new_mock_with_delay(
        generator_vec: InlineVec<CrosstermEventResult>,
        delay: Duration,
    ) -> InputDevice {
        InputDevice::new_mock_with_delay(generator_vec, delay)
    }
}
```

## Step 4: Update Module Exports [PENDING]

Update module declarations to reflect the new file locations and exports.

### Step 4.1: Update `tui/src/core/terminal_io/mod.rs` [PENDING]

Add/update the following:

```rust
pub mod input_device;
pub mod input_device_ext;

pub use input_device::*;
pub use input_device_ext::*;
```

### Step 4.2: Update `tui/src/tui/terminal_lib_backends/crossterm_backend/mod.rs` [PENDING]

Add:

```rust
pub mod input_device_impl;

pub use input_device_impl::*;
```

### Step 4.3: Update `tui/src/tui/terminal_lib_backends/mod.rs` [PENDING]

Remove:

```rust
pub mod input_device_ext;  // Moved to core/terminal_io/
```

The trait is now exported from `core/terminal_io/`.

### Step 4.4: Update `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mod.rs` [PENDING]

Re-export `DirectToAnsiInputDevice` if not already done:

```rust
pub use input_device_impl::*;
```

## Step 5: Update Import Sites [PENDING]

Search for and update all imports to use the new locations.

**Estimated ~19 files** need import updates.

#### Step 5.1: Search for imports to update

Search for:

- `use crate::InputDevice;` - Most should continue working unchanged
- `use crate::InputDeviceExt;` - Verify imports from new location
- Internal backend code - May need `CrosstermInputDevice` explicitly

#### Step 5.2: Files likely needing updates

- `tui/src/tui/terminal_window/main_event_loop.rs`
- `tui/src/tui/dialog/dialog_engine/dialog_engine_api.rs`
- `tui/src/readline_async/*.rs` (multiple files)
- All test files using mock input devices

#### Step 5.3: Search commands

```bash
# Find all imports
rg "use.*InputDevice" tui/src/

# Find all usages
rg "InputDevice::" tui/src/

# Find mock usage
rg "new_mock" tui/src/
```

# Testing Strategy

## Automated Testing

1. **After each step:**

   ```bash
   cargo check
   cargo clippy --all-targets
   ```

2. **After all changes:**

   ```bash
   cargo test --all-targets  # Run all tests
   cargo test --doc          # Run doctests
   cargo build               # Full build
   ```

3. **Specific test verification:**

   ```bash
   # Mock tests
   cargo test --package r3bl_tui input_device_fixtures

   # Integration tests
   cargo test --package r3bl_tui --test '*'
   ```

## Manual Testing

1. **DirectToAnsi backend (Linux):**
   - Run TUI examples on Linux
   - Verify keyboard input works
   - Verify mouse input works

2. **Crossterm backend (all platforms):**
   - Run TUI examples with Crossterm forced
   - Verify same behavior as DirectToAnsi

# Rollback Plan

If issues arise:

1. Git revert to commit before refactoring
2. Individual step rollback possible (commits per step recommended)
3. Old `input_device.rs` backed up in git history

# Definition of Done

- [ ] All files moved to correct locations
- [ ] `CrosstermInputDevice` properly renamed and moved
- [ ] `DirectToAnsiInputDevice` implements `InputDeviceExt`
- [ ] `MockInputDevice` created and working
- [ ] Generic `InputDevice` enum created with all constructors
- [ ] `InputDeviceExtMock` updated to delegate to enum
- [ ] All module exports updated
- [ ] `TEMP_READ_BUFFER_SIZE` const added
- [ ] `cargo check` passes
- [ ] `cargo clippy --all-targets` passes
- [ ] `cargo test --all-targets` passes (all tests green)
- [ ] `cargo test --doc` passes
- [ ] Manual testing on Linux (DirectToAnsi) successful
- [ ] Manual testing on non-Linux (Crossterm) successful
- [ ] No import errors in any file
- [ ] Documentation updated and accurate

# Benefits Summary

[COMPLETE] **Architectural Symmetry:** Perfect mirror of OutputDevice pattern

[COMPLETE] **Zero Dependencies:** No `async-trait`, no `Pin<Box<Future>>` complexity

[COMPLETE] **Auto Backend Selection:** Uses `TERMINAL_LIB_BACKEND` automatically

[COMPLETE] **Backward Compatible:** Existing test API (`new_mock`) works unchanged

[COMPLETE] **Clean Separation:** Trait in core/, implementations in backends/

[COMPLETE] **Extensible:** Easy to add new backends (just add enum variant)

[COMPLETE] **Type Safe:** Enum dispatch provides exhaustive matching

[COMPLETE] **Simple:** Straightforward `async fn` in trait, no boxing needed

# Risks and Mitigations

| Risk                                  | Mitigation                                                          |
| ------------------------------------- | ------------------------------------------------------------------- |
| Large refactoring touching many files | Test after each step, commit incrementally                          |
| Import errors in updated files        | Comprehensive search before/after, use compiler errors as checklist |
| Mock tests break                      | Backward compatible API maintained, verify early in testing         |
| Backend selection fails               | Manual testing on both Linux and non-Linux platforms                |
| Performance regression                | Enum dispatch is zero-cost, benchmark if concerned                  |

# Notes

- This refactoring is part of the larger
  [`task_remove_crossterm.md step 8.3`](task_remove_crossterm.md#step-83-backend-device-implementation-complete)
  effort
- The generic `InputDevice` enum is simpler than trait objects while achieving the same goal
- The `is_mock()` method provides API parity with `OutputDevice` for future extensibility
- All existing test code continues to work without changes due to backward-compatible mock API
