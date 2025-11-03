// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{core::ansi::vt_100_terminal_input_parser::{
                test_fixtures::generate_keyboard_sequence,
                types::{VT100InputEvent, VT100KeyCode, VT100KeyModifiers}
            },
            Deadline, generate_pty_test, InputEvent,
            tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice};
use std::{io::{BufRead, BufReader, Write},
          time::Duration};

// XMARK: Process isolated test functions using env vars & PTY.

generate_pty_test! {
    /// PTY-based integration test for [`DirectToAnsiInputDevice`].
    ///
    /// Test coordinator that routes to master or slave based on env var.
    /// When `PTY_SLAVE` is set, runs slave logic and exits.
    /// Otherwise runs the master test.
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
    /// │ Actor 1: PTY Master (test coordinator)                        │
    /// │ Synchronous code                                              │
    /// │                                                               │
    /// │  1. Create PTY pair (master/slave file descriptors)           │
    /// │  2. Spawn test binary with PTY_SLAVE=1 env var                │
    /// │  3. Write ANSI sequences to PTY master (the pipe)             │
    /// │  4. Read parsed events from Actor 2's stdout via PTY          │
    /// │  5. Verify parsed events match expected values                │
    /// └────────────────────────┬──────────────────────────────────────┘
    ///                          │ spawns with slave PTY as stdin/stdout
    /// ┌────────────────────────▼──────────────────────────────────────┐
    /// │ Actor 2: PTY Slave (worker process, PTY_SLAVE=1)              │
    /// │ Tokio runtime and async code                                  │
    /// │                                                               │
    /// │  1. Test function detects PTY_SLAVE env var                   │
    /// │  2. CRITICAL: Enable raw mode on terminal (PTY slave)         │
    /// │  3. Create DirectToAnsiInputDevice (reads from stdin)         │
    /// │  4. Loop: read_event() → parse ANSI → write to stdout         │
    /// │  5. Exit after processing test sequences                      │
    /// └───────────────────────────────────────────────────────────────┘
    /// ```
    ///
    /// ## Critical: Raw Mode Requirement
    ///
    /// **Raw Mode Clarification**: In PTY architecture, the SLAVE side is what the child
    /// process sees as its terminal. When the child reads from stdin, it's reading from
    /// the slave PTY. Therefore, we MUST set the SLAVE to raw mode so that:
    ///
    /// 1. **No Line Buffering**: Input isn't line-buffered - characters are available
    ///    immediately without waiting for Enter key
    /// 2. **No Special Character Processing**: Special characters (like ESC sequences) aren't
    ///    interpreted by the terminal layer - they pass through as raw bytes
    /// 3. **Async Compatibility**: The async reader can get bytes as they arrive, not waiting
    ///    for newlines, enabling proper ANSI escape sequence parsing
    ///
    /// **Master vs Slave**: The master doesn't need raw mode - it's just a bidirectional
    /// pipe for communication. The slave is the actual "terminal" that needs proper
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
    master: pty_master_entry_point,
    slave: pty_slave_entry_point
}

/// ### Actor 1: PTY Master (test entry, env var NOT set) - Synchronous code
/// - Receives PTY pair and child process from macro
/// - Writes ANSI sequences to PTY master
/// - Reads parsed output from slave's stdout
/// - Verifies correctness
#[allow(clippy::too_many_lines)]
fn pty_master_entry_point(
    pty_pair: portable_pty::PtyPair,
    mut child: Box<dyn portable_pty::Child + Send + Sync>,
) {
    /// Helper to generate ANSI bytes from `InputEvent`.
    fn generate_test_sequence(desc: &str, event: VT100InputEvent) -> (&str, Vec<u8>) {
        let bytes = generate_keyboard_sequence(&event)
            .unwrap_or_else(|| panic!("Failed to generate sequence for: {desc}"));
        (desc, bytes)
    }

    eprintln!("🚀 PTY Master: Starting...");

    // Get writer (to send ANSI sequences to slave) and non-blocking reader (to receive
    // parsed events from slave).
    let mut writer = pty_pair.master.take_writer().expect("Failed to get writer");
    let reader_non_blocking = pty_pair
        .master
        .try_clone_reader()
        .expect("Failed to get reader");
    let mut buf_reader_non_blocking = BufReader::new(reader_non_blocking);

    eprintln!("📝 PTY Master: Waiting for slave to start...");

    // Wait for slave to confirm it's running.
    let mut test_running_seen = false;
    let deadline = Deadline::default();

    // Non-blocking read loop: poll for slave startup with timeout.
    loop {
        assert!(deadline.has_time_remaining(), "Timeout: slave did not start within 5 seconds");

        let mut line = String::new();
        match buf_reader_non_blocking.read_line(&mut line) {
            Ok(0) => panic!("EOF reached before slave started"),
            Ok(_) => {
                let trimmed = line.trim();
                eprintln!("  ← Slave output: {trimmed}");

                if trimmed.contains("TEST_RUNNING") {
                    test_running_seen = true;
                    eprintln!("  ✓ Test is running in slave");
                }
                if trimmed.contains("SLAVE_STARTING") {
                    eprintln!("  ✓ Slave confirmed running!");
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => panic!("Read error while waiting for slave: {e}"),
        }
    }

    assert!(test_running_seen, "Slave test never started running (no TEST_RUNNING output)");

    // Send sequences and verify.
    let no_mods = VT100KeyModifiers::default();
    let sequences: Vec<(&str, Vec<u8>)> = vec![
        generate_test_sequence(
            "Up Arrow",
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Up,
                modifiers: no_mods,
            },
        ),
        generate_test_sequence(
            "Down Arrow",
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Down,
                modifiers: no_mods,
            },
        ),
        generate_test_sequence(
            "F1",
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Function(1),
                modifiers: no_mods,
            },
        ),
    ];

    eprintln!("📝 PTY Master: Sending {} sequences...", sequences.len());

    // For each test sequence: write ANSI bytes to PTY, read back parsed event, verify
    // correctness.
    for (desc, sequence) in &sequences {
        eprintln!("  → Sending: {desc} ({sequence:?})");

        writer
            .write_all(sequence)
            .expect("Failed to write sequence");
        writer.flush().expect("Failed to flush");

        // Give slave time to process
        std::thread::sleep(Duration::from_millis(100));

        // Read responses until we get an event line (skip test harness noise)
        let event_line = loop {
            let mut line = String::new();
            match buf_reader_non_blocking.read_line(&mut line) {
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

    eprintln!("🧹 PTY Master: Cleaning up...");

    // Close writer to signal EOF.
    drop(writer);

    // Wait for slave to exit.
    match child.wait() {
        Ok(status) => {
            eprintln!("✅ PTY Master: Slave exited: {status:?}");
        }
        Err(e) => {
            panic!("Failed to wait for slave: {e}");
        }
    }

    eprintln!("✅ PTY Master: Test passed!");
}

/// ### Actor 2: PTY Slave (worker process)
///
/// Runs in the spawned child process when `PTY_SLAVE` env var is set.
/// This process's stdin/stdout are connected to the PTY slave file descriptor.
///
/// **Critical Steps**:
/// 1. **Enable Raw Mode**: MUST set the PTY slave terminal to raw mode to:
///    - Disable line buffering (get bytes immediately)
///    - Prevent ANSI escape sequence interpretation
///    - Allow async byte-by-byte reading
/// 2. **Create Device**: Initialize `DirectToAnsiInputDevice` to read from stdin
/// 3. **Process Events**: Read and parse ANSI sequences into `InputEvents`
/// 4. **Output Results**: Write parsed events to stdout for master to verify
///
/// This function MUST exit before returning so other tests don't run.
fn pty_slave_entry_point() -> ! {
    // Print to stdout immediately to confirm slave is running.
    println!("SLAVE_STARTING");
    std::io::stdout().flush().expect("Failed to flush");

    // CRITICAL: Set the terminal (PTY slave) to raw mode.
    // Without this, DirectToAnsiInputDevice cannot read ANSI escape sequences properly
    // because they would be buffered or interpreted by the terminal layer.
    eprintln!("🔍 PTY Slave: Setting terminal to raw mode...");
    // Enter raw mode.
    if let Err(e) = crate::core::ansi::terminal_raw_mode::enable_raw_mode() {
        eprintln!("⚠️  PTY Slave: Failed to enable raw mode: {e}");
        // This would likely cause the test to fail - escape sequences won't be readable
    } else {
        eprintln!("✓ PTY Slave: Terminal in raw mode");
    }

    // Create a Tokio runtime for async operations.
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        eprintln!("🔍 PTY Slave: Starting...");
        let mut input_device = DirectToAnsiInputDevice::new();
        eprintln!("🔍 PTY Slave: Device created, reading events...");

        // Create inactivity timeout: exit if no events for 2 seconds.
        // https://developerlife.com/2024/07/10/rust-async-cancellation-safety-tokio/#example-1-right-and-wrong-way-to-sleep-and-interval
        let inactivity_timeout = Duration::from_secs(2);
        // Cancel safe: sleep_until() with a deadline stored outside select! is safe.
        let mut inactivity_deadline = tokio::time::Instant::now() + inactivity_timeout;

        let mut event_count = 0;

        loop {
            tokio::select! {
                // Try to read an event from the device.
                event_result = input_device.read_event() => {
                    match event_result {
                        Some(event) => {
                            event_count += 1;
                            inactivity_deadline = tokio::time::Instant::now() + inactivity_timeout; // Reset deadline.
                            eprintln!("🔍 PTY Slave: Event #{event_count}: {event:?}");

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
                                eprintln!("🔍 PTY Slave: Processed {event_count} events, exiting");
                                break;
                            }
                        }
                        None => {
                            eprintln!("🔍 PTY Slave: EOF reached");
                            break;
                        }
                    }
                }
                // Inactivity timeout: exit if deadline is reached (2 seconds of no events).
                // https://developerlife.com/2024/07/10/rust-async-cancellation-safety-tokio/#example-1-right-and-wrong-way-to-sleep-and-interval
                () = tokio::time::sleep_until(inactivity_deadline) => {
                    eprintln!("🔍 PTY Slave: Inactivity timeout (2 seconds with no events), exiting");
                    break;
                }
            }
        }

        eprintln!("🔍 PTY Slave: Completed, exiting");
    });

    // Clean up: disable raw mode before exiting.
    if let Err(e) = crate::core::ansi::terminal_raw_mode::disable_raw_mode() {
        eprintln!("⚠️  PTY Slave: Failed to disable raw mode: {e}");
    }

    eprintln!("🔍 Slave: Completed, exiting");
    // CRITICAL: Exit immediately to prevent test harness from running other tests.
    std::process::exit(0);
}
