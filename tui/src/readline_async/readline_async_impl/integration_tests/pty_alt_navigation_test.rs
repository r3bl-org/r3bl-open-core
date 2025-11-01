// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{generate_pty_test, Deadline, readline_async::readline_async_impl::LineState, core::test_fixtures::StdoutMock};
use std::{io::{BufRead, BufReader, Write}, time::Duration, sync::{Arc, Mutex as StdMutex}};

generate_pty_test! {
    /// PTY-based integration test for Alt+B/F word navigation.
    ///
    /// Validates that Alt+B (backward) and Alt+F (forward) correctly move the cursor
    /// to word boundaries, providing bash-compatible word navigation.
    ///
    /// Tests:
    /// 1. Alt+B: Move backward one word
    /// 2. Alt+F: Move forward one word
    /// 3. Multiple navigations across word boundaries
    ///
    /// Uses the coordinator-worker pattern with two processes ([`LineState`]).
    ///
    /// [`LineState`]: crate::readline_async::readline_async_impl::LineState
    test_fn: test_pty_alt_navigation,
    master: pty_master_entry_point,
    slave: pty_slave_entry_point
}

/// PTY Master: Send Alt+B/F sequences and verify navigation
fn pty_master_entry_point(
    pty_pair: portable_pty::PtyPair,
    mut child: Box<dyn portable_pty::Child + Send + Sync>,
) {
    eprintln!("🚀 PTY Master: Starting Alt+B/F test...");

    let mut writer = pty_pair.master.take_writer().expect("Failed to get writer");
    let reader_non_blocking = pty_pair
        .master
        .try_clone_reader()
        .expect("Failed to get reader");
    let mut buf_reader_non_blocking = BufReader::new(reader_non_blocking);

    eprintln!("📝 PTY Master: Waiting for slave to start...");

    // Wait for slave to confirm it's running
    let mut test_running_seen = false;
    let deadline = Deadline::default();

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

    // Helper function to read line state, skipping debug output
    let mut read_line_state = || -> String {
        loop {
            let mut line = String::new();
            match buf_reader_non_blocking.read_line(&mut line) {
                Ok(0) => panic!("EOF reached before getting line state"),
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.starts_with("Line:") || trimmed.contains("EOF") {
                        return trimmed.to_string();
                    }
                    eprintln!("  ⚠️  Skipping: {trimmed}");
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) => panic!("Read error: {e}"),
            }
        }
    };

    // Setup: Send "one two three"
    eprintln!("📝 PTY Master: Setting up line...");
    writer.write_all(b"one two three").expect("Failed to write text");
    writer.flush().expect("Failed to flush");
    std::thread::sleep(Duration::from_millis(200));

    let result = read_line_state();
    eprintln!("  ← Initial line: {result}");
    assert_eq!(result, "Line: one two three, Cursor: 13");

    // Test 1: Alt+B to move backward to "two"
    eprintln!("📝 PTY Master: Test 1 - Alt+B to start of 'three'...");

    // Alt+B is ESC b
    writer.write_all(b"\x1bb").expect("Failed to write Alt+B");
    writer.flush().expect("Failed to flush");
    std::thread::sleep(Duration::from_millis(100));

    let result = read_line_state();
    eprintln!("  ← After Alt+B: {result}");
    assert_eq!(result, "Line: one two three, Cursor: 8");

    // Test 2: Another Alt+B to move to "one"
    eprintln!("📝 PTY Master: Test 2 - Alt+B to start of 'two'...");
    writer.write_all(b"\x1bb").expect("Failed to write Alt+B");
    writer.flush().expect("Failed to flush");
    std::thread::sleep(Duration::from_millis(100));

    let result = read_line_state();
    eprintln!("  ← After Alt+B: {result}");
    assert_eq!(result, "Line: one two three, Cursor: 4");

    // Test 3: Alt+F to move forward to "two"
    eprintln!("📝 PTY Master: Test 3 - Alt+F to start of 'three'...");

    // Alt+F is ESC f
    writer.write_all(b"\x1bf").expect("Failed to write Alt+F");
    writer.flush().expect("Failed to flush");
    std::thread::sleep(Duration::from_millis(100));

    let result = read_line_state();
    eprintln!("  ← After Alt+F: {result}");
    assert_eq!(result, "Line: one two three, Cursor: 8");

    eprintln!("🧹 PTY Master: Cleaning up...");
    drop(writer);

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

/// PTY Slave: Process readline input and report line state
fn pty_slave_entry_point() -> ! {
    use crate::tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice;

    println!("SLAVE_STARTING");
    std::io::stdout().flush().expect("Failed to flush");

    eprintln!("🔍 PTY Slave: Setting terminal to raw mode...");
    if let Err(e) = crate::core::ansi::terminal_raw_mode::enable_raw_mode() {
        eprintln!("⚠️  PTY Slave: Failed to enable raw mode: {e}");
    } else {
        eprintln!("✓ PTY Slave: Terminal in raw mode");
    }

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        eprintln!("🔍 PTY Slave: Starting...");

        let mut line_state = LineState::new(String::new(), (100, 100));
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = crate::readline_async::readline_async_impl::History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        eprintln!("🔍 PTY Slave: LineState created, reading input...");

        let mut input_device = DirectToAnsiInputDevice::new();

        let inactivity_timeout = Duration::from_secs(2);
        let mut inactivity_deadline = tokio::time::Instant::now() + inactivity_timeout;

        loop {
            tokio::select! {
                event_result = input_device.read_event() => {
                    match event_result {
                        Some(event) => {
                            inactivity_deadline = tokio::time::Instant::now() + inactivity_timeout;
                            eprintln!("🔍 PTY Slave: Event: {event:?}");

                            let result = line_state.apply_event_and_render(
                                &event,
                                &mut *safe_output_terminal.lock().unwrap(),
                                &safe_history,
                            );

                            match result {
                                Ok(Some(readline_event)) => {
                                    eprintln!("🔍 PTY Slave: ReadlineEvent: {readline_event:?}");
                                }
                                Ok(None) => {
                                    // Normal event, output line state
                                    let output = format!("Line: {}, Cursor: {}",
                                        line_state.line,
                                        line_state.line_cursor_grapheme
                                    );
                                    println!("{output}");
                                    std::io::stdout().flush().expect("Failed to flush");
                                }
                                Err(e) => {
                                    eprintln!("🔍 PTY Slave: Error: {e:?}");
                                }
                            }
                        }
                        None => {
                            eprintln!("🔍 PTY Slave: EOF reached");
                            break;
                        }
                    }
                }
                () = tokio::time::sleep_until(inactivity_deadline) => {
                    eprintln!("🔍 PTY Slave: Inactivity timeout, exiting");
                    break;
                }
            }
        }

        eprintln!("🔍 PTY Slave: Completed, exiting");
    });

    if let Err(e) = crate::core::ansi::terminal_raw_mode::disable_raw_mode() {
        eprintln!("⚠️  PTY Slave: Failed to disable raw mode: {e}");
    }

    eprintln!("🔍 Slave: Completed, exiting");
    std::process::exit(0);
}
