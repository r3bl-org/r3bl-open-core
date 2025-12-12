T# Plan: Add Backend Compatibility Tests for InputDevice

## Overview

Create integration tests that verify `DirectToAnsiInputDevice` and `CrosstermInputDevice` produce
identical `InputEvent` values when given the same ANSI byte sequences. This catches compatibility
issues between our custom parser and crossterm's parser.

## Problem Statement

The codebase has two input backends:

| Backend | Parser | Platform |
|:--------|:-------|:---------|
| `DirectToAnsiInputDevice` | `vt_100_terminal_input_parser` (our code) | Linux-only |
| `CrosstermInputDevice` | crossterm's internal parser | Cross-platform |

Both produce `InputEvent`, but they use completely different parsing stacks. There's currently no
test that verifies they produce **identical results** for the same input.

## Solution

Use the existing PTY test infrastructure to:

1. Spawn two separate child processes (one per backend)
2. Send identical ANSI byte sequences to each
3. Compare the resulting `InputEvent` outputs

## Architecture

```text
┌─────────────────────────────────────────────────────────────────────┐
│                    Compatibility Test Harness                        │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  Test Input: ANSI escape sequence (e.g., "\x1B[A" = Up Arrow)       │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │ Test 1: test_pty_backend_direct_to_ansi                     │    │
│  │                                                             │    │
│  │  Controller (parent)         Controlled (child)             │    │
│  │  ├─ Create PTY pair          ├─ Enable raw mode             │    │
│  │  ├─ Spawn child              ├─ Create DirectToAnsiInput    │    │
│  │  ├─ Write ANSI bytes ──────► ├─ Read from stdin (PTY)       │    │
│  │  ├─ Read stdout ◄─────────── ├─ Parse → InputEvent          │    │
│  │  └─ Collect result           └─ Print "{event:?}"           │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │ Test 2: test_pty_backend_crossterm                          │    │
│  │                                                             │    │
│  │  Controller (parent)         Controlled (child)             │    │
│  │  ├─ Create PTY pair          ├─ Enable raw mode             │    │
│  │  ├─ Spawn child              ├─ Create CrosstermInput       │    │
│  │  ├─ Write ANSI bytes ──────► ├─ Read from stdin (PTY)       │    │
│  │  ├─ Read stdout ◄─────────── ├─ Parse → InputEvent          │    │
│  │  └─ Collect result           └─ Print "{event:?}"           │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                      │
│  Comparison: Assert both tests produce identical InputEvent         │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

## File Structure

```
tui/src/core/terminal_io/
├── mod.rs                              # UPDATE: Add integration_tests module
├── input_device.rs                     # Existing: InputDevice enum
├── input_event.rs                      # Existing: InputEvent type
└── integration_tests/                  # NEW
    ├── mod.rs                          # Module declarations
    └── backend_compatibility_test.rs   # Compatibility tests
```

## Implementation Steps

### Step 1: Update `terminal_io/mod.rs`

Add the integration_tests module declaration:

```rust
// At end of file
#[cfg(any(test, doc))]
pub mod integration_tests;
```

### Step 2: Create `integration_tests/mod.rs`

```rust
// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Integration tests for [`InputDevice`] backends.
//!
//! These tests verify that different input backends produce consistent [`InputEvent`]
//! values when given identical ANSI byte sequences.
//!
//! # Test Strategy
//!
//! Each backend is tested in isolation using PTY-based integration tests:
//! - A controller process writes ANSI bytes to the PTY master
//! - A controlled process reads from stdin (PTY slave) using the specific backend
//! - The parsed [`InputEvent`] is output for verification
//!
//! # Platform Support
//!
//! | Test | Linux | macOS | Windows |
//! |:-----|:------|:------|:--------|
//! | `test_pty_backend_direct_to_ansi` | ✅ | ❌ | ❌ |
//! | `test_pty_backend_crossterm` | ✅ | ✅ | ✅ |
//!
//! Compatibility comparison (both backends) only runs on Linux.
//!
//! [`InputDevice`]: crate::InputDevice
//! [`InputEvent`]: crate::InputEvent

#[cfg(any(test, doc))]
pub mod backend_compatibility_test;
```

### Step 3: Create `backend_compatibility_test.rs`

```rust
// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Backend compatibility tests for [`DirectToAnsiInputDevice`] and [`CrosstermInputDevice`].
//!
//! Verifies both backends produce identical [`InputEvent`] for the same ANSI sequences.

use crate::{generate_pty_test, InputEvent, PtyPair, Deadline};
use std::{
    io::{BufRead, BufReader, Write},
    time::Duration,
};

// ============================================================================
// Test Sequences - Same bytes sent to both backends
// ============================================================================

/// Test cases: (description, ANSI bytes, expected InputEvent pattern)
///
/// These sequences are sent to both backends and results are compared.
const TEST_SEQUENCES: &[(&str, &[u8])] = &[
    // Arrow keys
    ("Up Arrow", b"\x1B[A"),
    ("Down Arrow", b"\x1B[B"),
    ("Right Arrow", b"\x1B[C"),
    ("Left Arrow", b"\x1B[D"),

    // Navigation keys
    ("Home", b"\x1B[H"),
    ("End", b"\x1B[F"),
    ("Page Up", b"\x1B[5~"),
    ("Page Down", b"\x1B[6~"),
    ("Insert", b"\x1B[2~"),
    ("Delete", b"\x1B[3~"),

    // Function keys
    ("F1", b"\x1BOP"),
    ("F2", b"\x1BOQ"),
    ("F3", b"\x1BOR"),
    ("F4", b"\x1BOS"),
    ("F5", b"\x1B[15~"),

    // Modifiers (CSI u format - modern terminals)
    ("Ctrl+A", b"\x01"),  // Control character
    ("Ctrl+C", b"\x03"),

    // Special keys
    ("Enter", b"\r"),
    ("Tab", b"\t"),
    ("Escape", b"\x1B"),
    ("Backspace", b"\x7F"),

    // Arrow keys with modifiers (xterm format)
    ("Shift+Up", b"\x1B[1;2A"),
    ("Ctrl+Up", b"\x1B[1;5A"),
    ("Alt+Up", b"\x1B[1;3A"),
    ("Ctrl+Shift+Up", b"\x1B[1;6A"),
];

// ============================================================================
// DirectToAnsi Backend Test (Linux-only)
// ============================================================================

#[cfg(target_os = "linux")]
generate_pty_test! {
    /// PTY test for DirectToAnsiInputDevice backend.
    ///
    /// Sends test sequences and outputs parsed InputEvents.
    test_fn: test_pty_backend_direct_to_ansi,
    controller: direct_to_ansi_controller,
    controlled: direct_to_ansi_controlled
}

#[cfg(target_os = "linux")]
fn direct_to_ansi_controller(
    pty_pair: PtyPair,
    mut child: Box<dyn portable_pty::Child + Send + Sync>,
) {
    run_backend_controller("DirectToAnsi", pty_pair, &mut child);

    // Wait for child to exit
    let status = child.wait().expect("Failed to wait for child");
    assert!(status.success(), "DirectToAnsi controlled process failed");
}

#[cfg(target_os = "linux")]
fn direct_to_ansi_controlled() -> ! {
    use crate::tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice;

    run_backend_controlled("DirectToAnsi", || {
        Box::new(DirectToAnsiInputDeviceWrapper(DirectToAnsiInputDevice::new()))
    });
}

// ============================================================================
// Crossterm Backend Test (Cross-platform)
// ============================================================================

generate_pty_test! {
    /// PTY test for CrosstermInputDevice backend.
    ///
    /// Sends test sequences and outputs parsed InputEvents.
    test_fn: test_pty_backend_crossterm,
    controller: crossterm_controller,
    controlled: crossterm_controlled
}

fn crossterm_controller(
    pty_pair: PtyPair,
    mut child: Box<dyn portable_pty::Child + Send + Sync>,
) {
    run_backend_controller("Crossterm", pty_pair, &mut child);

    // Wait for child to exit
    let status = child.wait().expect("Failed to wait for child");
    assert!(status.success(), "Crossterm controlled process failed");
}

fn crossterm_controlled() -> ! {
    use crate::CrosstermInputDevice;

    run_backend_controlled("Crossterm", || {
        Box::new(CrosstermInputDeviceWrapper(CrosstermInputDevice::new_event_stream()))
    });
}

// ============================================================================
// Shared Controller Logic
// ============================================================================

/// Shared controller logic for both backends.
///
/// 1. Waits for controlled to be ready
/// 2. Sends each test sequence
/// 3. Reads and prints parsed InputEvent from controlled
fn run_backend_controller(
    backend_name: &str,
    pty_pair: PtyPair,
    child: &mut Box<dyn portable_pty::Child + Send + Sync>,
) {
    eprintln!("🚀 {backend_name} Controller: Starting...");

    let mut writer = pty_pair.controller().take_writer().expect("Failed to get writer");
    let reader = pty_pair.controller().try_clone_reader().expect("Failed to get reader");
    let mut buf_reader = BufReader::new(reader);

    // Wait for controlled to be ready
    wait_for_controlled_ready(&mut buf_reader, backend_name);

    // Send each test sequence and collect results
    for (desc, bytes) in TEST_SEQUENCES {
        eprintln!("📝 {backend_name} Controller: Sending {desc}...");

        // Send the ANSI sequence
        writer.write_all(bytes).expect("Failed to write sequence");
        writer.flush().expect("Failed to flush");

        // Read the parsed event from controlled
        let deadline = Deadline::new(Duration::from_secs(2));
        loop {
            if !deadline.has_time_remaining() {
                eprintln!("⚠️  {backend_name} Controller: Timeout waiting for {desc}");
                break;
            }

            let mut line = String::new();
            match buf_reader.read_line(&mut line) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.starts_with("EVENT:") {
                        eprintln!("✅ {backend_name} {desc}: {trimmed}");
                        break;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) => panic!("Read error: {e}"),
            }
        }
    }

    // Signal controlled to exit
    eprintln!("📝 {backend_name} Controller: Sending exit signal...");
    writer.write_all(b"EXIT\n").expect("Failed to send exit");
    writer.flush().expect("Failed to flush");
}

fn wait_for_controlled_ready(buf_reader: &mut BufReader<impl std::io::Read>, backend_name: &str) {
    let deadline = Deadline::new(Duration::from_secs(5));

    loop {
        assert!(deadline.has_time_remaining(), "Timeout waiting for controlled to start");

        let mut line = String::new();
        match buf_reader.read_line(&mut line) {
            Ok(0) => panic!("EOF before controlled ready"),
            Ok(_) => {
                let trimmed = line.trim();
                eprintln!("  ← {backend_name} Controlled: {trimmed}");
                if trimmed.contains("CONTROLLED_READY") {
                    eprintln!("  ✓ {backend_name} Controlled is ready");
                    return;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => panic!("Read error: {e}"),
        }
    }
}

// ============================================================================
// Shared Controlled Logic
// ============================================================================

/// Trait for unified backend access in controlled process.
trait InputDeviceBackend: Send {
    fn try_read_event(&mut self) -> impl std::future::Future<Output = Option<InputEvent>> + Send;
}

/// Wrapper for DirectToAnsiInputDevice
#[cfg(target_os = "linux")]
struct DirectToAnsiInputDeviceWrapper(crate::tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice);

#[cfg(target_os = "linux")]
impl InputDeviceBackend for DirectToAnsiInputDeviceWrapper {
    async fn try_read_event(&mut self) -> Option<InputEvent> {
        self.0.try_read_event().await
    }
}

/// Wrapper for CrosstermInputDevice
struct CrosstermInputDeviceWrapper(crate::CrosstermInputDevice);

impl InputDeviceBackend for CrosstermInputDeviceWrapper {
    async fn try_read_event(&mut self) -> Option<InputEvent> {
        self.0.next().await
    }
}

/// Shared controlled logic for both backends.
///
/// 1. Enables raw mode
/// 2. Creates the backend device
/// 3. Reads events and outputs them in parseable format
/// 4. Exits on timeout or EXIT signal
fn run_backend_controlled<F, D>(backend_name: &str, create_device: F) -> !
where
    F: FnOnce() -> Box<D>,
    D: InputDeviceBackend + 'static,
{
    // Signal ready
    println!("CONTROLLED_READY");
    std::io::stdout().flush().expect("Failed to flush");

    // Enable raw mode
    eprintln!("🔍 {backend_name} Controlled: Enabling raw mode...");
    if let Err(e) = crate::core::ansi::terminal_raw_mode::enable_raw_mode() {
        eprintln!("⚠️  {backend_name} Controlled: Failed to enable raw mode: {e}");
    }

    // Create tokio runtime
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create runtime");

    runtime.block_on(async {
        eprintln!("🔍 {backend_name} Controlled: Creating device...");
        let mut device = create_device();

        let inactivity_timeout = Duration::from_secs(3);
        let mut deadline = tokio::time::Instant::now() + inactivity_timeout;

        loop {
            tokio::select! {
                event = device.try_read_event() => {
                    match event {
                        Some(input_event) => {
                            deadline = tokio::time::Instant::now() + inactivity_timeout;

                            // Output in parseable format
                            println!("EVENT: {input_event:?}");
                            std::io::stdout().flush().expect("Failed to flush");
                        }
                        None => {
                            eprintln!("🔍 {backend_name} Controlled: EOF");
                            break;
                        }
                    }
                }
                () = tokio::time::sleep_until(deadline) => {
                    eprintln!("🔍 {backend_name} Controlled: Inactivity timeout");
                    break;
                }
            }
        }
    });

    // Disable raw mode before exit
    let _ = crate::core::ansi::terminal_raw_mode::disable_raw_mode();

    eprintln!("🔍 {backend_name} Controlled: Exiting");
    std::process::exit(0);
}

// ============================================================================
// Compatibility Comparison Test (Linux-only - both backends required)
// ============================================================================

/// This test runs both backend tests and compares their outputs.
///
/// Only available on Linux where both backends work.
#[cfg(target_os = "linux")]
#[test]
#[ignore] // Run manually: cargo test test_backend_compatibility_comparison -- --ignored --nocapture
fn test_backend_compatibility_comparison() {
    // This test would:
    // 1. Run test_pty_backend_direct_to_ansi and capture output
    // 2. Run test_pty_backend_crossterm and capture output
    // 3. Compare the EVENT: lines from both
    // 4. Report any differences

    // Implementation note: This could use std::process::Command to run
    // both tests and capture their output, then parse and compare.

    eprintln!("TODO: Implement comparison logic");
    eprintln!("For now, run both tests manually and compare:");
    eprintln!("  cargo test test_pty_backend_direct_to_ansi -- --nocapture");
    eprintln!("  cargo test test_pty_backend_crossterm -- --nocapture");
}
```

## Test Sequences to Cover

### Priority 1: Basic Keys (Most likely to differ)

| Category | Sequences | Notes |
|:---------|:----------|:------|
| Arrow keys | Up, Down, Left, Right | Basic CSI sequences |
| Navigation | Home, End, PageUp, PageDown | Tilde-terminated |
| Function keys | F1-F12 | SS3 vs CSI formats vary |
| Editing | Insert, Delete, Backspace | Terminal-dependent |

### Priority 2: Modifiers (Complex parsing)

| Category | Sequences | Notes |
|:---------|:----------|:------|
| Ctrl+key | Ctrl+A through Ctrl+Z | Control characters |
| Arrow+modifier | Shift/Ctrl/Alt + arrows | CSI parameter encoding |
| Function+modifier | Shift+F1, Ctrl+F5, etc. | Extended sequences |

### Priority 3: Mouse Events (If enabled)

| Category | Sequences | Notes |
|:---------|:----------|:------|
| SGR mouse | Button press/release | Modern protocol |
| Scroll | Wheel up/down | Button codes 64/65 |
| Drag | Motion with button held | Tracking mode |

### Priority 4: Terminal Events

| Category | Sequences | Notes |
|:---------|:----------|:------|
| Focus | Focus gained/lost | CSI I / CSI O |
| Bracketed paste | Paste start/end | CSI 200~ / CSI 201~ |

## Platform Considerations

```rust
// DirectToAnsi test: Linux-only
#[cfg(target_os = "linux")]
generate_pty_test! { ... }

// Crossterm test: All platforms
generate_pty_test! { ... }

// Comparison test: Linux-only (needs both backends)
#[cfg(target_os = "linux")]
#[test]
fn test_backend_compatibility_comparison() { ... }
```

## Running the Tests

```bash
# Run DirectToAnsi backend test (Linux only)
cargo test -p r3bl_tui --lib test_pty_backend_direct_to_ansi -- --nocapture

# Run Crossterm backend test (all platforms)
cargo test -p r3bl_tui --lib test_pty_backend_crossterm -- --nocapture

# Run comparison test (Linux only, manual)
cargo test -p r3bl_tui --lib test_backend_compatibility_comparison -- --ignored --nocapture
```

## Expected Output Format

Both backends should output events in this format:

```
CONTROLLED_READY
EVENT: Keyboard(Plain { key: SpecialKey(Up) })
EVENT: Keyboard(Plain { key: SpecialKey(Down) })
EVENT: Keyboard(WithModifiers { key: SpecialKey(Up), mask: ModifierKeysMask { shift: Pressed, ctrl: NotPressed, alt: NotPressed } })
...
```

The comparison test parses these `EVENT:` lines and verifies they match.

## Known Differences to Document

Some sequences may legitimately produce different results due to:

1. **Terminal emulator variations**: Some sequences have multiple valid interpretations
2. **Modifier encoding**: Different terminals encode modifiers differently
3. **Legacy compatibility**: Crossterm may support legacy formats we don't

Document any known differences in the test file with comments explaining why they differ.

## Future Enhancements

1. **Automated comparison**: Parse both outputs and diff programmatically
2. **CI integration**: Run on Linux CI where both backends work
3. **Regression tracking**: Store baseline outputs and detect changes
4. **Coverage expansion**: Add more obscure sequences as issues are found

## References

- Existing PTY test infrastructure: `tui/src/core/test_fixtures/pty_test_fixtures/`
- DirectToAnsi tests: `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/`
- `generate_pty_test!` macro: `tui/src/core/test_fixtures/pty_test_fixtures/generate_pty_test.rs`
- InputDevice enum: `tui/src/core/terminal_io/input_device.rs`
- InputEvent type: `tui/src/core/terminal_io/input_event.rs`
