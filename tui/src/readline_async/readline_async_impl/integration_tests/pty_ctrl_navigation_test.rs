// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{AsyncDebouncedDeadline, ControlledChild, DebouncedState, KeyState, PtyPair,
            core::{ansi::{generator::generate_keyboard_sequence,
                          vt_100_terminal_input_parser::{VT100InputEventIR,
                                                         VT100KeyCodeIR,
                                                         VT100KeyModifiersIR}},
                   test_fixtures::StdoutMock},
            generate_pty_test,
            readline_async::readline_async_impl::LineState};
use std::{io::{BufRead, BufReader, Write},
          sync::{Arc, Mutex as StdMutex},
          time::Duration};

// ==================== Test Input Sequences ====================
//
// These helper functions generate ANSI escape sequences using the VT100 input generator.
// This ensures the test sends the exact same sequences that the parser expects.

/// Ctrl+Left: Move cursor one word backward
/// Generates: ESC [ 1 ; 5 D
fn ctrl_left() -> Vec<u8> {
    generate_keyboard_sequence(&VT100InputEventIR::Keyboard {
        code: VT100KeyCodeIR::Left,
        modifiers: VT100KeyModifiersIR {
            ctrl: KeyState::Pressed,
            shift: KeyState::NotPressed,
            alt: KeyState::NotPressed,
        },
    })
    .expect("Ctrl+Left should generate valid sequence")
}

/// Ctrl+Right: Move cursor one word forward
/// Generates: ESC [ 1 ; 5 C
fn ctrl_right() -> Vec<u8> {
    generate_keyboard_sequence(&VT100InputEventIR::Keyboard {
        code: VT100KeyCodeIR::Right,
        modifiers: VT100KeyModifiersIR {
            ctrl: KeyState::Pressed,
            shift: KeyState::NotPressed,
            alt: KeyState::NotPressed,
        },
    })
    .expect("Ctrl+Right should generate valid sequence")
}

generate_pty_test! {
    /// PTY-based integration test for Ctrl+Left/Right word navigation.
    ///
    /// Validates that Ctrl+Left and Ctrl+Right correctly move the cursor
    /// to word boundaries, respecting whitespace and punctuation.
    ///
    /// Run with: `cargo test -p r3bl_tui --lib test_pty_ctrl_navigation -- --nocapture`
    ///
    /// Tests:
    /// 1. Ctrl+Left: Move to start of previous word
    /// 2. Ctrl+Right: Move to start of next word
    /// 3. Multiple navigations across word boundaries
    ///
    /// ## Test Protocol (Request-Response Pattern)
    ///
    /// This test uses a **request-response protocol** between controller and controlled processes:
    ///
    /// 1. **Controller sends input** (e.g., "hello world test" or ESC sequences)
    /// 2. **Controller flushes** and waits ~200ms for controlled process to process
    /// 3. **Controller blocks** reading controlled stdout until it sees "Line: ..."
    /// 4. **Controller makes assertion** on the line state
    /// 5. **Repeat** for next input sequence
    ///
    /// **Critical requirement**: Controlled process must output line state **only once** after
    /// processing all available input, not after every character. Otherwise, controller
    /// will read intermediate states (e.g., "Line: h, Cursor: 1" instead of
    /// "Line: hello world test, Cursor: 16").
    ///
    /// The ([`LineState`]) is checked in the tests to make assertions against.
    ///
    /// [`LineState`]: crate::readline_async::readline_async_impl::LineState
    test_fn: test_pty_ctrl_navigation,
    controller: pty_controller_entry_point,
    controlled: pty_controlled_entry_point
}

/// PTY Controller: Send Ctrl+Left/Right sequences and verify navigation
#[allow(clippy::too_many_lines)]
fn pty_controller_entry_point(pty_pair: PtyPair, mut child: ControlledChild) {
    eprintln!("🚀 PTY Controller: Starting Ctrl+Left/Right test...");

    let mut writer = pty_pair.controller().take_writer().expect("Failed to get writer");
    let reader = pty_pair
        .controller()
        .try_clone_reader()
        .expect("Failed to get reader");
    let mut buf_reader = BufReader::new(reader);

    eprintln!("📝 PTY Controller: Waiting for controlled process to start...");

    // Wait for controlled process to confirm it's running. The controlled process sends
    // TEST_RUNNING and CONTROLLED_STARTING immediately on startup.
    let mut test_running_seen = false;

    loop {
        let mut line = String::new();
        match buf_reader.read_line(&mut line) {
            Ok(0) => panic!("EOF reached before controlled process started"),
            Ok(_) => {
                let trimmed = line.trim();
                eprintln!("  ← Controlled output: {trimmed}");

                if trimmed.contains("TEST_RUNNING") {
                    test_running_seen = true;
                    eprintln!("  ✓ Test is running in controlled process");
                }
                if trimmed.contains("CONTROLLED_STARTING") {
                    eprintln!("  ✓ Controlled process confirmed running!");
                    break;
                }
            }
            Err(e) => panic!("Read error while waiting for controlled process: {e}"),
        }
    }

    assert!(
        test_running_seen,
        "Controlled process test never started running (no TEST_RUNNING output)"
    );

    // ==================== Helper: Read Line State from Controlled ====================
    //
    // Blocks until controlled process prints "Line: ..." output, skipping debug messages.
    // Blocking reads work reliably because controlled process responds immediately.
    let mut read_line_state = || -> String {
        loop {
            let mut line = String::new();
            match buf_reader.read_line(&mut line) {
                Ok(0) => panic!("EOF reached before getting line state"),
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.starts_with("Line:") || trimmed.contains("EOF") {
                        return trimmed.to_string();
                    }
                    eprintln!("  ⚠️  Skipping: {trimmed}");
                }
                Err(e) => panic!("Read error: {e}"),
            }
        }
    };

    // ==================== Setup: Send "hello world test" ====================
    eprintln!("📝 PTY Controller: Setting up line...");
    writer
        .write_all(b"hello world test")
        .expect("Failed to write text");
    writer.flush().expect("Failed to flush");

    // Wait 200ms for controlled process to process all 16 characters and print line state.
    // Controlled process's 10ms debounce ensures all chars are batched before printing.
    std::thread::sleep(Duration::from_millis(200));

    let result = read_line_state();
    eprintln!("  ← Initial line: {result}");
    assert_eq!(result, "Line: hello world test, Cursor: 16");

    // ==================== Test 1: Ctrl+Left to move to start of "test"
    // ====================
    eprintln!("📝 PTY Controller: Test 1 - Ctrl+Left to start of 'test'...");
    writer
        .write_all(&ctrl_left())
        .expect("Failed to write Ctrl+Left");
    writer.flush().expect("Failed to flush");

    // Wait 100ms for controlled process to process single key sequence and print line state
    std::thread::sleep(Duration::from_millis(100));

    let result = read_line_state();
    eprintln!("  ← After Ctrl+Left: {result}");
    assert_eq!(result, "Line: hello world test, Cursor: 12");

    // ==================== Test 2: Another Ctrl+Left to move to start of "world"
    // ====================
    eprintln!("📝 PTY Controller: Test 2 - Ctrl+Left to start of 'world'...");
    writer
        .write_all(&ctrl_left())
        .expect("Failed to write Ctrl+Left");
    writer.flush().expect("Failed to flush");
    std::thread::sleep(Duration::from_millis(100));

    let result = read_line_state();
    eprintln!("  ← After Ctrl+Left: {result}");
    assert_eq!(result, "Line: hello world test, Cursor: 6");

    // ==================== Test 3: Ctrl+Right to move to start of "test"
    // ====================
    eprintln!("📝 PTY Controller: Test 3 - Ctrl+Right to start of 'test'...");
    writer
        .write_all(&ctrl_right())
        .expect("Failed to write Ctrl+Right");
    writer.flush().expect("Failed to flush");
    std::thread::sleep(Duration::from_millis(100));

    let result = read_line_state();
    eprintln!("  ← After Ctrl+Right: {result}");
    assert_eq!(result, "Line: hello world test, Cursor: 12");

    eprintln!("🧹 PTY Controller: Cleaning up...");
    drop(writer);

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

/// PTY Controlled: Process readline input and report line state
fn pty_controlled_entry_point() -> ! {
    use crate::tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice;

    println!("CONTROLLED_STARTING");
    std::io::stdout().flush().expect("Failed to flush");

    println!("🔍 PTY Controlled: Setting terminal to raw mode...");
    if let Err(e) = crate::core::ansi::terminal_raw_mode::enable_raw_mode() {
        println!("⚠️  PTY Controlled: Failed to enable raw mode: {e}");
    } else {
        println!("✓ PTY Controlled: Terminal in raw mode");
    }

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        println!("🔍 PTY Controlled: Starting...");

        let mut line_state = LineState::new(String::new(), (100, 100));
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = crate::readline_async::readline_async_impl::History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        println!("🔍 PTY Controlled: LineState created, reading input...");

        let mut input_device = DirectToAnsiInputDevice::new();

        // ==================== Timing Configuration ====================
        //
        // Inactivity watchdog: Exit if no events arrive for 2 seconds
        // Pattern: "Exit if this operation takes too long"
        let mut inactivity_watchdog = AsyncDebouncedDeadline::new(Duration::from_secs(2));
        inactivity_watchdog.reset(); // Start the watchdog

        // Debounced state: Buffer line state and print after 10ms of no events
        // Pattern: "Do X after Y ms of no activity"
        // This batches rapid input (e.g., "hello world test" arrives as 16 chars
        // within ~1-2ms, all processed before first print at ~12ms)
        let mut buffered_state = DebouncedState::new(Duration::from_millis(10));

        // ==================== Event Loop ====================
        loop {
            tokio::select! {
                // -------- Branch 1: Read next input event --------
                event_result = input_device.try_read_event() => {
                    match event_result {
                        Some(event) => {
                            // Reset inactivity watchdog on each event
                            inactivity_watchdog.reset();
                            println!("🔍 PTY Controlled: Event: {event:?}");

                            // Process the input event
                            let result = line_state.apply_event_and_render(
                                &event,
                                &mut *safe_output_terminal.lock().unwrap(),
                                &safe_history,
                            );

                            match result {
                                Ok(Some(readline_event)) => {
                                    println!("🔍 PTY Controlled: ReadlineEvent: {readline_event:?}");
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
                                    println!("🔍 PTY Controlled: Error: {e:?}");
                                }
                            }
                        }
                        None => {
                            println!("🔍 PTY Controlled: EOF reached");
                            break;
                        }
                    }
                }

                // -------- Branch 2: Print buffered state after debounce delay --------
                // If we should poll the debounced state, then sleep until the debounce
                // timer expires, and when it fires, execute this code.
                () = buffered_state.sleep_until(), if buffered_state.should_poll() => {
                    // No new events arrived within 10ms, print the buffered line state
                    if let Some(state) = buffered_state.take() {
                        println!("{state}");
                        std::io::stdout().flush().expect("Failed to flush");
                    }
                }

                // -------- Branch 3: Exit on inactivity timeout --------
                () = inactivity_watchdog.sleep_until() => {
                    println!("🔍 PTY Controlled: Inactivity timeout hit, exiting");
                    break;
                }
            }
        }

        println!("🔍 PTY Controlled: Completed, exiting");
    });

    if let Err(e) = crate::core::ansi::terminal_raw_mode::disable_raw_mode() {
        println!("⚠️  PTY Controlled: Failed to disable raw mode: {e}");
    }

    println!("🔍 Controlled: Completed, exiting");
    std::process::exit(0);
}
