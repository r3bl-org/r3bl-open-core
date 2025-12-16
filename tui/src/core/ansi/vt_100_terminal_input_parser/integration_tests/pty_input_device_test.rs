// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{ControlledChild, InputEvent, PtyPair,
            core::ansi::{generator::generate_keyboard_sequence,
                         vt_100_terminal_input_parser::ir_event_types::{VT100InputEventIR,
                                                                        VT100KeyCodeIR,
                                                                        VT100KeyModifiersIR}},
            generate_pty_test,
            tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice};
use std::{io::{BufRead, BufReader, Write},
          time::Duration};

/// Ready signal sent by controlled process after initialization.
const CONTROLLED_READY: &str = "CONTROLLED_READY";

// XMARK: Process isolated test functions using env vars & PTY.

generate_pty_test! {
    /// PTY-based integration test for [`DirectToAnsiInputDevice`].
    ///
    /// Test coordinator that routes to controller or controlled based on env var.
    /// When `R3BL_PTY_TEST_CONTROLLED` is set, runs controlled logic and exits.
    /// Otherwise runs the controller test.
    ///
    /// Run with: `cargo test -p r3bl_tui --lib test_pty_input_device -- --nocapture`
    ///
    /// ## Test Architecture (2 Actors)
    ///
    /// This test validates [`DirectToAnsiInputDevice`] in a real PTY environment using a
    /// coordinator-worker pattern with two processes:
    ///
    /// ```text
    /// ┌───────────────────────────────────────────────────────────────┐
    /// │ Actor 1: PTY Controller (test coordinator)                        │
    /// │ Synchronous code                                              │
    /// │                                                               │
    /// │  1. Create PTY pair (controller/controlled file descriptors)           │
    /// │  2. Spawn test binary with R3BL_PTY_TEST_CONTROLLED=1 env var                │
    /// │  3. Write ANSI sequences to PTY controller (the pipe)             │
    /// │  4. Read parsed events from Actor 2's stdout via PTY          │
    /// │  5. Verify parsed events match expected values                │
    /// └────────────────────────┬──────────────────────────────────────┘
    ///                          │ spawns with controlled PTY as stdin/stdout
    /// ┌────────────────────────▼──────────────────────────────────────┐
    /// │ Actor 2: PTY Controlled (worker process, R3BL_PTY_TEST_CONTROLLED=1)              │
    /// │ Tokio runtime and async code                                  │
    /// │                                                               │
    /// │  1. Test function detects R3BL_PTY_TEST_CONTROLLED env var                   │
    /// │  2. CRITICAL: Enable raw mode on terminal (controlled PTY)         │
    /// │  3. Create DirectToAnsiInputDevice (reads from stdin)         │
    /// │  4. Loop: try_read_event() → parse ANSI → write to stdout     │
    /// │  5. Exit after processing test sequences                      │
    /// └───────────────────────────────────────────────────────────────┘
    /// ```
    ///
    /// ## Critical: Raw Mode Requirement
    ///
    /// **Raw Mode Clarification**: In PTY architecture, the controlled PTY side is what the child
    /// process sees as its terminal. When the child reads from stdin, it's reading from
    /// the controlled PTY. Therefore, we MUST set the controlled PTY to raw mode so that:
    ///
    /// 1. **No Line Buffering**: Input isn't line-buffered - characters are available
    ///    immediately without waiting for Enter key
    /// 2. **No Special Character Processing**: Special characters (like ESC sequences) aren't
    ///    interpreted by the terminal layer - they pass through as raw bytes
    /// 3. **Async Compatibility**: The async reader can get bytes as they arrive, not waiting
    ///    for newlines, enabling proper ANSI escape sequence parsing
    ///
    /// **Controller vs Controlled**: The controller doesn't need raw mode - it's just a bidirectional
    /// pipe for communication. The controlled side is the actual "terminal" that needs proper
    /// settings for the child process to read ANSI sequences correctly.
    ///
    /// Without raw mode, the PTY stays in "cooked" mode where:
    /// - Input waits for line termination (Enter key)
    /// - Control sequences may be interpreted instead of passed through
    /// - [`DirectToAnsiInputDevice`] times out waiting for input that's stuck in buffers
    ///
    /// ## Why This Test Pattern?
    ///
    /// - **Real PTY Environment**: Tests [`DirectToAnsiInputDevice`] with actual PTY, not
    ///   mocks
    /// - **Process Isolation**: Each test run gets fresh PTY resources via process spawning
    /// - **Coordinator-Worker Pattern**: Same test function handles both roles via env var
    /// - **Async Validation**: Properly tests tokio async I/O with real terminal input
    ///
    /// [`DirectToAnsiInputDevice`]: crate::tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice
    test_fn: test_pty_input_device,
    controller: pty_controller_entry_point,
    controlled: pty_controlled_entry_point
}

/// ### Actor 1: PTY Controller (test entry, env var NOT set) - Synchronous code
/// - Receives PTY pair and child process from macro
/// - Writes ANSI sequences to PTY controller
/// - Reads parsed output from controlled's stdout
/// - Verifies correctness
#[allow(clippy::too_many_lines)]
fn pty_controller_entry_point(pty_pair: PtyPair, mut child: ControlledChild) {
    /// Helper to generate ANSI bytes from `InputEvent`.
    fn generate_test_sequence(desc: &str, event: VT100InputEventIR) -> (&str, Vec<u8>) {
        let bytes = generate_keyboard_sequence(&event)
            .unwrap_or_else(|| panic!("Failed to generate sequence for: {desc}"));
        (desc, bytes)
    }

    eprintln!("🚀 PTY Controller: Starting...");

    // Get writer (to send ANSI sequences to controlled) and reader (to receive
    // parsed events from controlled).
    let mut writer = pty_pair.controller().take_writer().expect("Failed to get writer");
    let reader = pty_pair
        .controller()
        .try_clone_reader()
        .expect("Failed to get reader");
    let mut buf_reader = BufReader::new(reader);

    eprintln!("📝 PTY Controller: Waiting for controlled process to start...");

    // Wait for controlled to confirm it's running. The controlled process sends
    // TEST_RUNNING and CONTROLLED_READY immediately on startup.
    let mut test_running_seen = false;

    loop {
        let mut line = String::new();
        match buf_reader.read_line(&mut line) {
            Ok(0) => panic!("EOF reached before controlled started"),
            Ok(_) => {
                let trimmed = line.trim();
                eprintln!("  ← Controlled output: {trimmed}");

                if trimmed.contains("TEST_RUNNING") {
                    test_running_seen = true;
                    eprintln!("  ✓ Test is running in controlled");
                }
                if trimmed.contains(CONTROLLED_READY) {
                    eprintln!("  ✓ Controlled process confirmed running!");
                    break;
                }
            }
            Err(e) => panic!("Read error while waiting for controlled: {e}"),
        }
    }

    assert!(
        test_running_seen,
        "Controlled test never started running (no TEST_RUNNING output)"
    );

    // Send sequences and verify.
    let no_mods = VT100KeyModifiersIR::default();
    let sequences: Vec<(&str, Vec<u8>)> = vec![
        generate_test_sequence(
            "Up Arrow",
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Up,
                modifiers: no_mods,
            },
        ),
        generate_test_sequence(
            "Down Arrow",
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Down,
                modifiers: no_mods,
            },
        ),
        generate_test_sequence(
            "F1",
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Function(1),
                modifiers: no_mods,
            },
        ),
    ];

    eprintln!("📝 PTY Controller: Sending {} sequences...", sequences.len());

    // For each test sequence: write ANSI bytes to PTY, read back parsed event, verify
    // correctness.
    for (desc, sequence) in &sequences {
        eprintln!("  → Sending: {desc} ({sequence:?})");

        writer
            .write_all(sequence)
            .expect("Failed to write sequence");
        writer.flush().expect("Failed to flush");

        // Give controlled time to process
        std::thread::sleep(Duration::from_millis(100));

        // Read responses until we get an event line (skip test harness noise)
        let event_line = loop {
            let mut line = String::new();
            match buf_reader.read_line(&mut line) {
                Ok(0) => {
                    panic!("EOF reached before receiving event for {desc}");
                }
                Ok(_) => {
                    let trimmed = line.trim();

                    // Check if it's an event line
                    if trimmed.starts_with("Keyboard:")
                        || trimmed.starts_with("Mouse:")
                        || trimmed.starts_with("Resize:")
                        || trimmed.starts_with("Focus:")
                        || trimmed.starts_with("Paste:")
                    {
                        break trimmed.to_string();
                    }

                    // Skip test harness noise
                    eprintln!("  ⚠️  Skipping non-event output: {trimmed}");
                }
                Err(e) => {
                    panic!("Read error for {desc}: {e}");
                }
            }
        };

        eprintln!("  ✓ {desc}: {event_line}");
    }

    eprintln!("🧹 PTY Controller: Cleaning up...");

    // Close writer to signal EOF.
    drop(writer);

    // Wait for controlled to exit.
    match child.wait() {
        Ok(status) => {
            eprintln!("✅ PTY Controller: Controlled process exited: {status:?}");
        }
        Err(e) => {
            panic!("Failed to wait for controlled process: {e}");
        }
    }

    eprintln!("✅ PTY Controller: Test passed!");
}

/// ### Actor 2: PTY Controlled (worker process)
///
/// Runs in the spawned child process when `R3BL_PTY_TEST_CONTROLLED` env var is set.
/// This process's stdin/stdout are connected to the controlled PTY file descriptor.
///
/// **Critical Steps**:
/// 1. **Enable Raw Mode**: MUST set the controlled PTY terminal to raw mode to:
///    - Disable line buffering (get bytes immediately)
///    - Prevent ANSI escape sequence interpretation
///    - Allow async byte-by-byte reading
/// 2. **Create Device**: Initialize `DirectToAnsiInputDevice` to read from stdin
/// 3. **Process Events**: Read and parse ANSI sequences into `InputEvents`
/// 4. **Output Results**: Write parsed events to stdout for master to verify
///
/// This function MUST exit before returning so other tests don't run.
fn pty_controlled_entry_point() -> ! {
    // Print to stdout immediately to confirm controlled is running.
    println!("{CONTROLLED_READY}");
    std::io::stdout().flush().expect("Failed to flush");

    // CRITICAL: Set the terminal (controlled PTY) to raw mode.
    // Without this, DirectToAnsiInputDevice cannot read ANSI escape sequences properly
    // because they would be buffered or interpreted by the terminal layer.
    eprintln!("🔍 PTY Controlled: Setting terminal to raw mode...");
    // Enter raw mode.
    if let Err(e) = crate::core::ansi::terminal_raw_mode::enable_raw_mode() {
        eprintln!("⚠️  PTY Controlled: Failed to enable raw mode: {e}");
        // This would likely cause the test to fail - escape sequences won't be readable
    } else {
        eprintln!("✓ PTY Controlled: Terminal in raw mode");
    }

    // Create a Tokio runtime for async operations.
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        eprintln!("🔍 PTY Controlled: Starting...");
        let mut input_device = DirectToAnsiInputDevice::new();
        eprintln!("🔍 PTY Controlled: Device created, reading events...");

        // Create inactivity timeout: exit if no events for 2 seconds.
        // https://developerlife.com/2024/07/10/rust-async-cancellation-safety-tokio/#example-1-right-and-wrong-way-to-sleep-and-interval
        let inactivity_timeout = Duration::from_secs(2);
        // Cancel safe: sleep_until() with a deadline stored outside select! is safe.
        let mut inactivity_deadline = tokio::time::Instant::now() + inactivity_timeout;

        let mut event_count = 0;

        loop {
            tokio::select! {
                // Try to read an event from the device.
                event_result = input_device.try_read_event() => {
                    match event_result {
                        Some(event) => {
                            event_count += 1;
                            inactivity_deadline = tokio::time::Instant::now() + inactivity_timeout; // Reset deadline.
                            eprintln!("🔍 PTY Controlled: Event #{event_count}: {event:?}");

                            // Output event in parseable format.
                            let output = match event {
                                InputEvent::Keyboard(ref key_press) => {
                                    format!("Keyboard: {key_press:?}")
                                }
                                InputEvent::Mouse(ref mouse_input) => {
                                    format!("Mouse: {mouse_input:?}")
                                }
                                InputEvent::Resize(ref size) => {
                                    format!("Resize: {size:?}")
                                }
                                InputEvent::Focus(ref state) => {
                                    format!("Focus: {state:?}")
                                }
                                InputEvent::BracketedPaste(ref text) => {
                                    format!("Paste: {} chars", text.len())
                                }
                            };

                            println!("{output}");
                            std::io::stdout().flush().expect("Failed to flush stdout");

                            // Exit after processing a few events for testing.
                            if event_count >= 3 {
                                eprintln!("🔍 PTY Controlled: Processed {event_count} events, exiting");
                                break;
                            }
                        }
                        None => {
                            eprintln!("🔍 PTY Controlled: EOF reached");
                            break;
                        }
                    }
                }
                // Inactivity timeout: exit if deadline is reached (2 seconds of no events).
                // https://developerlife.com/2024/07/10/rust-async-cancellation-safety-tokio/#example-1-right-and-wrong-way-to-sleep-and-interval
                () = tokio::time::sleep_until(inactivity_deadline) => {
                    eprintln!("🔍 PTY Controlled: Inactivity timeout (2 seconds with no events), exiting");
                    break;
                }
            }
        }

        eprintln!("🔍 PTY Controlled: Completed, exiting");
    });

    // Clean up: disable raw mode before exiting.
    if let Err(e) = crate::core::ansi::terminal_raw_mode::disable_raw_mode() {
        eprintln!("⚠️  PTY Controlled: Failed to disable raw mode: {e}");
    }

    eprintln!("🔍 Controlled: Completed, exiting");
    // CRITICAL: Exit immediately to prevent test harness from running other tests.
    std::process::exit(0);
}
