// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{AsyncDebouncedDeadline, ControlledChild, Deadline, DebouncedState, Pair,
            core::test_fixtures::StdoutMock,
            generate_pty_test,
            readline_async::readline_async_impl::LineState};
use std::{io::{BufRead, BufReader, Write},
          sync::{Arc, Mutex as StdMutex},
          time::Duration};

generate_pty_test! {
    /// PTY-based integration test for Alt+D and Alt+Backspace word killing.
    ///
    /// Validates that Alt+D (kill word forward) and Alt+Backspace (kill word backward)
    /// correctly delete words at word boundaries.
    ///
    /// Run with: `cargo test -p r3bl_tui --lib test_pty_alt_kill -- --nocapture`
    ///
    /// Tests:
    /// 1. Alt+D: Delete word forward from cursor
    /// 2. Alt+Backspace: Delete word backward from cursor
    /// 3. Word deletion with punctuation boundaries
    ///
    /// ## Test Protocol (Request-Response Pattern)
    ///
    /// This test uses a **request-response protocol** between master and slave:
    ///
    /// 1. **Master sends input** (e.g., "hello-world" or Alt+D sequences)
    /// 2. **Master flushes** and waits ~200ms for slave to process
    /// 3. **Master blocks** reading slave stdout until it sees "Line: ..."
    /// 4. **Master makes assertion** on the line state
    /// 5. **Repeat** for next input sequence
    ///
    /// **Critical requirement**: Slave must output line state **only once** after
    /// processing all available input, not after every character. Otherwise, master
    /// will read intermediate states.
    ///
    /// The ([`LineState`]) is checked in the tests to make assertions against.
    ///
    /// [`LineState`]: crate::readline_async::readline_async_impl::LineState
    test_fn: test_pty_alt_kill,
    master: pty_master_entry_point,
    slave: pty_slave_entry_point
}

/// PTY Master: Send Alt+D/Backspace sequences and verify word deletion
fn pty_master_entry_point(pty_pair: Pair, mut child: ControlledChild) {
    eprintln!("🚀 PTY Master: Starting Alt+D/Backspace test...");

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
        assert!(
            deadline.has_time_remaining(),
            "Timeout: slave did not start within 5 seconds"
        );

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

    assert!(
        test_running_seen,
        "Slave test never started running (no TEST_RUNNING output)"
    );

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

    // Test 1: Alt+D to delete word forward
    eprintln!("📝 PTY Master: Test 1 - Alt+D to delete word forward...");

    // Send "hello world test"
    writer
        .write_all(b"hello world test")
        .expect("Failed to write text");
    writer.flush().expect("Failed to flush");
    std::thread::sleep(Duration::from_millis(200));

    let result = read_line_state();
    eprintln!("  ← Initial line: {result}");
    assert_eq!(result, "Line: hello world test, Cursor: 16");

    // Move to start with Ctrl+A
    writer.write_all(&[0x01]).expect("Failed to write Ctrl+A");
    writer.flush().expect("Failed to flush");
    std::thread::sleep(Duration::from_millis(100));

    let result = read_line_state();
    eprintln!("  ← After Ctrl+A: {result}");
    assert_eq!(result, "Line: hello world test, Cursor: 0");

    // Alt+D to delete "hello"
    // Alt+D is ESC d
    writer.write_all(b"\x1bd").expect("Failed to write Alt+D");
    writer.flush().expect("Failed to flush");
    std::thread::sleep(Duration::from_millis(100));

    let result = read_line_state();
    eprintln!("  ← After Alt+D: {result}");
    assert_eq!(result, "Line:  world test, Cursor: 0");

    // Test 2: Alt+Backspace to delete word backward
    eprintln!("📝 PTY Master: Test 2 - Alt+Backspace to delete word backward...");

    // Clear and send "one two three"
    writer.write_all(&[0x15]).expect("Failed to write Ctrl+U");
    writer.flush().expect("Failed to flush");
    std::thread::sleep(Duration::from_millis(100));

    let result = read_line_state();
    eprintln!("  ← After clear: {result}");

    writer
        .write_all(b"one two three")
        .expect("Failed to write text");
    writer.flush().expect("Failed to flush");
    std::thread::sleep(Duration::from_millis(200));

    let result = read_line_state();
    eprintln!("  ← New line: {result}");
    assert_eq!(result, "Line: one two three, Cursor: 13");

    // Alt+Backspace to delete "three"
    // Alt+Backspace is ESC DEL (0x7f)
    writer
        .write_all(b"\x1b\x7f")
        .expect("Failed to write Alt+Backspace");
    writer.flush().expect("Failed to flush");
    std::thread::sleep(Duration::from_millis(100));

    let result = read_line_state();
    eprintln!("  ← After Alt+Backspace: {result}");
    assert_eq!(result, "Line: one two , Cursor: 8");

    // Test 3: Another Alt+Backspace to delete "two"
    eprintln!("📝 PTY Master: Test 3 - Another Alt+Backspace...");
    writer
        .write_all(b"\x1b\x7f")
        .expect("Failed to write Alt+Backspace");
    writer.flush().expect("Failed to flush");
    std::thread::sleep(Duration::from_millis(100));

    let result = read_line_state();
    eprintln!("  ← After Alt+Backspace: {result}");
    assert_eq!(result, "Line: one , Cursor: 4");

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

    println!("🔍 PTY Slave: Setting terminal to raw mode...");
    if let Err(e) = crate::core::ansi::terminal_raw_mode::enable_raw_mode() {
        println!("⚠️  PTY Slave: Failed to enable raw mode: {e}");
    } else {
        println!("✓ PTY Slave: Terminal in raw mode");
    }

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        println!("🔍 PTY Slave: Starting...");

        let mut line_state = LineState::new(String::new(), (100, 100));
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = crate::readline_async::readline_async_impl::History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        println!("🔍 PTY Slave: LineState created, reading input...");

        let mut input_device = DirectToAnsiInputDevice::new();

        // ==================== Timing Configuration ====================
        //
        // Inactivity watchdog: Exit if no events arrive for 2 seconds
        // Pattern: "Exit if this operation takes too long"
        let mut inactivity_watchdog = AsyncDebouncedDeadline::new(Duration::from_secs(2));
        inactivity_watchdog.reset(); // Start the watchdog

        // Debounced state: Buffer line state and print after 10ms of no events
        // Pattern: "Do X after Y ms of no activity"
        // This batches rapid input (e.g., "one two three" arrives as 13 chars
        // within ~1-2ms, all processed before first print at ~12ms)
        let mut buffered_state = DebouncedState::new(Duration::from_millis(10));

        // ==================== Event Loop ====================
        loop {
            tokio::select! {
                // -------- Branch 1: Read next input event --------
                event_result = input_device.read_event() => {
                    match event_result {
                        Some(event) => {
                            // Reset inactivity watchdog on each event
                            inactivity_watchdog.reset();
                            println!("🔍 PTY Slave: Event: {event:?}");

                            let result = line_state.apply_event_and_render(
                                &event,
                                &mut *safe_output_terminal.lock().unwrap(),
                                &safe_history,
                            );

                            match result {
                                Ok(Some(readline_event)) => {
                                    println!("🔍 PTY Slave: ReadlineEvent: {readline_event:?}");
                                }
                                Ok(None) => {
                                    // Buffer the current line state and reset debounce timer.
                                    // If another event arrives before 10ms, we update the buffered
                                    // state and reset the timer again (batching rapid input).
                                    buffered_state.set(format!(
                                        "Line: {}, Cursor: {}",
                                        line_state.line,
                                        line_state.line_cursor_grapheme
                                    ));
                                }
                                Err(e) => {
                                    println!("🔍 PTY Slave: Error: {e:?}");
                                }
                            }
                        }
                        None => {
                            println!("🔍 PTY Slave: EOF reached");
                            break;
                        }
                    }
                }
                // -------- Branch 2: Print buffered state after debounce delay --------
                // If we should poll the debounced state, then sleep until the debounce timer expires, and when it fires, execute this code.
                () = buffered_state.sleep_until(), if buffered_state.should_poll() => {
                    // No new events arrived within 10ms, print the buffered line state
                    if let Some(state) = buffered_state.take() {
                        println!("{state}");
                        std::io::stdout().flush().expect("Failed to flush");
                    }
                }

                // -------- Branch 3: Exit on inactivity timeout --------
                () = inactivity_watchdog.sleep_until() => {
                    println!("🔍 PTY Slave: Inactivity timeout hit, exiting");
                    break;
                }
            }
        }

        println!("🔍 PTY Slave: Completed, exiting");
    });

    if let Err(e) = crate::core::ansi::terminal_raw_mode::disable_raw_mode() {
        println!("⚠️  PTY Slave: Failed to disable raw mode: {e}");
    }

    println!("🔍 Slave: Completed, exiting");
    std::process::exit(0);
}
