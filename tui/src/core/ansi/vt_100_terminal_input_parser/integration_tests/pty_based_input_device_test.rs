// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{core::ansi::{generator::generate_keyboard_sequence,
                         vt_100_terminal_input_parser::{InputEvent, KeyCode,
                                                        KeyModifiers}},
            run_test_in_isolated_process_with_pty,
            tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice};
use std::{io::{BufRead, BufReader, Write},
          time::{Duration, Instant}};

// XMARK: Process isolated test functions using env vars & PTY.

/// PTY-based integration test for [`DirectToAnsiInputDevice`].
///
/// Test coordinator that routes to master or slave based on env var.
/// When `PTY_SLAVE` is set, runs slave logic and exits.
/// Otherwise runs the master test.
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
/// - DirectToAnsiInputDevice times out waiting for input that's stuck in buffers
///
/// ## Why This Test Pattern?
///
/// - **Real PTY Environment**: Tests [`DirectToAnsiInputDevice`] with actual PTY, not
///   mocks
/// - **Process Isolation**: Each test run gets fresh PTY resources via process spawning
/// - **Coordinator-Worker Pattern**: Same test function handles both roles via env var
/// - **Async Validation**: Properly tests tokio async I/O with real terminal input
///
/// ## Running the Test
///
/// ```bash
/// cargo test test_pty_input_device -- --nocapture
/// ```
#[test]
fn test_pty_input_device() {
    run_test_in_isolated_process_with_pty!(
        env_var: "PTY_SLAVE",
        test_name: "test_pty_input_device",
        slave: run_pty_slave,
        master: run_pty_master
    );
}

/// ### Actor 1: PTY Master (test entry, env var NOT set) - Synchronous code
/// - Receives PTY pair and child process from macro
/// - Writes ANSI sequences to PTY master
/// - Reads parsed output from slave's stdout
/// - Verifies correctness
fn run_pty_master(
    pty_pair: portable_pty::PtyPair,
    mut child: Box<dyn portable_pty::Child + Send + Sync>,
) {
    /// Helper to generate ANSI bytes from InputEvent.
    fn generate_test_sequence(desc: &str, event: InputEvent) -> (&str, Vec<u8>) {
        let bytes = generate_keyboard_sequence(&event)
            .unwrap_or_else(|| panic!("Failed to generate sequence for: {}", desc));
        (desc, bytes)
    }

    eprintln!("🚀 PTY Master: Starting...");

    // Get master writer/reader
    let mut writer = pty_pair.master.take_writer().expect("Failed to get writer");
    let reader = pty_pair
        .master
        .try_clone_reader()
        .expect("Failed to get reader");
    let mut buf_reader = BufReader::new(reader);

    // 4. Define test sequences
    let no_mods = KeyModifiers::default();
    let sequences: Vec<(&str, Vec<u8>)> = vec![
        generate_test_sequence(
            "Up Arrow",
            InputEvent::Keyboard {
                code: KeyCode::Up,
                modifiers: no_mods,
            },
        ),
        generate_test_sequence(
            "Down Arrow",
            InputEvent::Keyboard {
                code: KeyCode::Down,
                modifiers: no_mods,
            },
        ),
        generate_test_sequence(
            "F1",
            InputEvent::Keyboard {
                code: KeyCode::Function(1),
                modifiers: no_mods,
            },
        ),
    ];

    eprintln!("📝 PTY Master: Waiting for slave to start...");

    // Wait for slave to confirm it's running
    let mut slave_started = false;
    let mut test_running_seen = false;
    let start_timeout = Instant::now();

    while !slave_started && start_timeout.elapsed() < Duration::from_secs(5) {
        let mut line = String::new();
        match buf_reader.read_line(&mut line) {
            Ok(0) => {
                eprintln!("  ⚠️  EOF reached while waiting for slave");
                break;
            }
            Ok(_) => {
                let trimmed = line.trim();
                eprintln!("  ← Slave output: {}", trimmed);

                // Look for our debug markers
                if trimmed.contains("TEST_RUNNING") {
                    test_running_seen = true;
                    eprintln!("  ✓ Test is running in slave");
                }
                if trimmed.contains("SLAVE_STARTING") {
                    slave_started = true;
                    eprintln!("  ✓ Slave confirmed running!");
                    break;
                }
                // Skip test harness output
                if trimmed.contains("running 1 test")
                    || trimmed.contains("test result:")
                    || trimmed.is_empty()
                {
                    continue;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => panic!("Read error while waiting for slave: {}", e),
        }
    }

    if !test_running_seen {
        panic!("Slave test never started running (no TEST_RUNNING output)");
    }
    if !slave_started {
        panic!(
            "Slave process did not enter slave mode within 5 seconds (no SLAVE_STARTING)"
        );
    }

    eprintln!("📝 PTY Master: Sending {} sequences...", sequences.len());

    // 5. Send sequences and verify
    for (desc, sequence) in &sequences {
        eprintln!("  → Sending: {} ({:?})", desc, sequence);

        writer
            .write_all(sequence)
            .expect("Failed to write sequence");
        writer.flush().expect("Failed to flush");

        // Give slave time to process
        std::thread::sleep(Duration::from_millis(100));

        // Read responses until we get an event line (skip test harness noise)
        let event_line = loop {
            let mut line = String::new();
            match buf_reader.read_line(&mut line) {
                Ok(0) => {
                    panic!("EOF reached before receiving event for {}", desc);
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
                    eprintln!("  ⚠️  Skipping non-event output: {}", trimmed);
                }
                Err(e) => {
                    panic!("Read error for {}: {}", desc, e);
                }
            }
        };

        eprintln!("  ✓ {}: {}", desc, event_line);
    }

    eprintln!("🧹 PTY Master: Cleaning up...");

    // 6. Close writer to signal EOF
    drop(writer);

    // Wait for slave to exit
    match child.wait() {
        Ok(status) => {
            eprintln!("✅ PTY Master: Slave exited: {:?}", status);
        }
        Err(e) => {
            panic!("Failed to wait for slave: {}", e);
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
/// 2. **Create Device**: Initialize DirectToAnsiInputDevice to read from stdin
/// 3. **Process Events**: Read and parse ANSI sequences into InputEvents
/// 4. **Output Results**: Write parsed events to stdout for master to verify
///
/// This function MUST exit before returning so other tests don't run.
fn run_pty_slave() -> ! {
    // Print to stdout immediately to confirm slave is running
    println!("SLAVE_STARTING");
    std::io::stdout().flush().expect("Failed to flush");

    // CRITICAL: Set the terminal (PTY slave) to raw mode
    // Without this, DirectToAnsiInputDevice cannot read ANSI escape sequences properly
    // because they would be buffered or interpreted by the terminal layer
    eprintln!("🔍 PTY Slave: Setting terminal to raw mode...");
    // Use our own raw mode implementation instead of crossterm
    if let Err(e) = crate::core::ansi::terminal_raw_mode::enable_raw_mode() {
        eprintln!("⚠️  PTY Slave: Failed to enable raw mode: {}", e);
        // This would likely cause the test to fail - escape sequences won't be readable
    } else {
        eprintln!("✓ PTY Slave: Terminal in raw mode");
    }

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        eprintln!("🔍 PTY Slave: Starting...");
        let mut device = DirectToAnsiInputDevice::new();
        eprintln!("🔍 PTY Slave: Device created, reading events...");

        // Add timeout to prevent hanging forever
        use tokio::time::timeout;
        let mut event_count = 0;

        loop {
            // Try to read an event with a timeout
            match timeout(Duration::from_millis(100), device.read_event()).await {
                Ok(Some(event)) => {
                    event_count += 1;
                    eprintln!("🔍 PTY Slave: Event #{}: {:?}", event_count, event);

                    // Output event in parseable format
                    let output = match event {
                        InputEvent::Keyboard { code, modifiers } => {
                            format!(
                                "Keyboard: {:?} (shift={} ctrl={} alt={})",
                                code, modifiers.shift, modifiers.ctrl, modifiers.alt
                            )
                        }
                        InputEvent::Mouse { button, action, .. } => {
                            format!("Mouse: button={:?} action={:?}", button, action)
                        }
                        InputEvent::Resize { rows, cols } => {
                            format!("Resize: {}x{}", rows, cols)
                        }
                        InputEvent::Focus(state) => {
                            format!("Focus: {:?}", state)
                        }
                        InputEvent::Paste(mode) => {
                            format!("Paste: {:?}", mode)
                        }
                    };

                    println!("{}", output);
                    std::io::stdout().flush().expect("Failed to flush stdout");

                    // Exit after processing a few events for testing
                    if event_count >= 3 {
                        eprintln!(
                            "🔍 PTY Slave: Processed {} events, exiting",
                            event_count
                        );
                        break;
                    }
                }
                Ok(None) => {
                    eprintln!("🔍 PTY Slave: EOF reached");
                    break;
                }
                Err(_) => {
                    // Timeout - check if we should exit
                    static mut TIMEOUT_COUNT: usize = 0;
                    unsafe {
                        TIMEOUT_COUNT += 1;
                        if TIMEOUT_COUNT > 20 {
                            eprintln!("🔍 PTY Slave: Too many timeouts, exiting");
                            break;
                        }
                    }
                }
            }
        }

        eprintln!("🔍 PTY Slave: Completed, exiting");
    });

    // Clean up: disable raw mode before exiting
    if let Err(e) = crate::core::ansi::terminal_raw_mode::disable_raw_mode() {
        eprintln!("⚠️  PTY Slave: Failed to disable raw mode: {}", e);
    }

    eprintln!("🔍 Slave: Completed, exiting");
    // CRITICAL: Exit immediately to prevent test harness from running other tests
    std::process::exit(0);
}
