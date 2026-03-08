// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{
    AsyncDebouncedDeadline,
    CONTROLLED_READY,
    CONTROLLED_STARTING,
    DebouncedState,
    PtyTestMode,
    TEST_RUNNING,
    core::test_fixtures::StdoutMock,
    readline_async::readline_async_impl::LineState,
    PtyTestContext,
};
use std::{io::{BufRead, Write},
          sync::{Arc, Mutex as StdMutex},
          time::Duration};

/// Prefix for line state output.
const LINE_PREFIX: &str = "Line:";

generate_pty_test! {
    /// [`PTY`]-based integration test for Ctrl+U line clearing behavior.
    ///
    /// Validates that Ctrl+U correctly clears from the start of the line to the cursor position.
    ///
    /// Run with:
    /// ```bash
    /// cargo test -p r3bl_tui --lib test_pty_ctrl_u -- --nocapture
    /// ```
    ///
    /// ## Test Cases
    ///
    /// 1. **Cursor at position 0**: Ctrl+U deletes nothing (0 to 0)
    /// 2. **Cursor at the end**: Ctrl+U deletes entire line (start to cursor at end)
    ///
    /// Note: We don't test "cursor in middle" as that would require navigation commands
    /// (Alt+B, Ctrl+Left, arrow keys, etc.) which violates Separation of Concerns.
    /// The two cases above cover the boundary conditions for Ctrl+U behavior.
    ///
    /// ## Test Protocol (Request-Response Pattern)
    ///
    /// This test uses a **request-response protocol** between controller and controlled:
    ///
    /// 1. **Controller sends input** (text and Ctrl+U sequences)
    /// 2. **Controller flushes** and blocks reading controlled stdout until it sees
    ///    "Line: ..."
    /// 3. **Controller makes assertion** on the line state
    /// 4. **Repeat** for next test case
    ///
    /// The ([`LineState`]) is checked in the test to make assertions against.
    ///
    /// [`LineState`]: crate::readline_async::readline_async_impl::LineState
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    test_fn: test_pty_ctrl_u,
    controller: pty_controller_entry_point,
    controlled: pty_controlled_entry_point,
    mode: PtyTestMode::Raw,
}

/// [`PTY`] Controller: Send Ctrl+U sequences and verify line clearing behavior
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
#[allow(clippy::too_many_lines)]
fn pty_controller_entry_point(context: PtyTestContext) {
    let PtyTestContext {
        pty_pair,
        child,
        mut buf_reader,
        mut writer,
    } = context;

    eprintln!("🚀 PTY Controller: Starting Ctrl+U test...");

    eprintln!("📝 PTY Controller: Waiting for controlled process to start...");

    // Wait for controlled to confirm it's running and ready. The controlled process sends
    // TEST_RUNNING, CONTROLLED_STARTING, and CONTROLLED_READY on startup.
    let mut test_running_seen = false;
    // Note: controlled_ready_seen will be assigned in the loop before being read
    let controlled_ready_seen;

    // Blocking reads work reliably because controlled process responds immediately.
    loop {
        let mut line = String::new();
        match buf_reader.read_line(&mut line) {
            Ok(0) => panic!("EOF reached before controlled started"),
            Ok(_) => {
                let trimmed = line.trim();
                eprintln!("  ← Controlled output: {trimmed}");

                if trimmed.contains(TEST_RUNNING) {
                    test_running_seen = true;
                    eprintln!("  ✓ Test is running in controlled");
                }
                if trimmed.contains(CONTROLLED_STARTING) {
                    eprintln!("  ✓ Controlled process confirmed running!");
                }
                if trimmed.contains(CONTROLLED_READY) {
                    controlled_ready_seen = true;
                    eprintln!(
                        "  ✓ Controlled is ready (input device created)"
                    );
                    break;
                }
            }
            Err(e) => panic!("Read error while waiting for controlled: {e}"),
        }
    }

    assert!(
        test_running_seen,
        "Controlled test never started running (no {TEST_RUNNING} output)"
    );
    assert!(
        controlled_ready_seen,
        "Controlled never signaled ready (no {CONTROLLED_READY} output)"
    );

    // Helper function to read line state, skipping debug output.
    // Blocking reads work reliably because controlled process responds immediately.
    let mut read_line_state = || -> String {
        loop {
            let mut line = String::new();
            match buf_reader.read_line(&mut line) {
                Ok(0) => panic!("EOF reached before getting line state"),
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.starts_with(LINE_PREFIX) {
                        return trimmed.to_string();
                    }
                    eprintln!("  ⚠️  Skipping: {trimmed}");
                }
                Err(e) => panic!("Read error: {e}"),
            }
        }
    };

    // Test Case 1: Ctrl+U with cursor at the end (deletes entire line)
    eprintln!("📝 PTY Controller: Test Case 1 - Ctrl+U with cursor at end...");

    // Type "hello world" which naturally leaves cursor at end
    writer
        .write_all(b"hello world")
        .expect("Failed to write text");
    writer.flush().expect("Failed to flush");

    let result = read_line_state();
    eprintln!("  ← Line with cursor at end: {result}");
    assert_eq!(result, "Line: hello world, Cursor: 11");

    // Ctrl+U at end should delete entire line
    writer.write_all(&[0x15]).expect("Failed to write Ctrl+U");
    writer.flush().expect("Failed to flush");

    let result = read_line_state();
    eprintln!("  ← After Ctrl+U (cursor at end): {result}");
    assert_eq!(
        result, "Line: , Cursor: 0",
        "Ctrl+U at end should delete entire line"
    );

    // Test Case 2: Ctrl+U with cursor at position 0 (deletes nothing)
    eprintln!("📝 PTY Controller: Test Case 2 - Ctrl+U with cursor at position 0...");

    // Now line is empty and cursor is at position 0
    // Ctrl+U at position 0 should still delete nothing
    writer.write_all(&[0x15]).expect("Failed to write Ctrl+U");
    writer.flush().expect("Failed to flush");

    let result = read_line_state();
    eprintln!("  ← After Ctrl+U on empty line: {result}");
    assert_eq!(
        result, "Line: , Cursor: 0",
        "Ctrl+U on empty line should delete nothing"
    );

    eprintln!("✅ PTY Controller: All Ctrl+U test cases passed!");

    // Clean shutdown
    eprintln!("🧹 PTY Controller: Cleaning up...");
    drop(writer);
    child.drain_and_wait(buf_reader, pty_pair);
    eprintln!("✅ PTY Controller: Test passed!");
}

/// [`PTY`] Controlled: Process readline input and report line state
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
fn pty_controlled_entry_point() {
    use crate::direct_to_ansi::DirectToAnsiInputDevice;

    println!("{CONTROLLED_STARTING}");
    std::io::stdout().flush().expect("Failed to flush");

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

        // Signal to controller that we're ready to receive input. MUST be after
        // DirectToAnsiInputDevice::new() so the mio poller thread is already
        // watching stdin before the controller sends any input through the PTY.
        println!("{CONTROLLED_READY}");
        std::io::stdout().flush().expect("Failed to flush");

        // ==================== Timing Configuration ====================
        //
        // Inactivity watchdog: Exit if no events arrive for 5 seconds.
        // Needs headroom for parallel test execution where CPU scheduling
        // delays can cause input events to arrive late.
        // Pattern: "Exit if this operation takes too long"
        let mut inactivity_watchdog = AsyncDebouncedDeadline::new(Duration::from_secs(5));
        inactivity_watchdog.reset(); // Start the watchdog

        // Debounced state: Buffer line state and print after 10ms of no events
        // Pattern: "Do X after Y ms of no activity"
        // This batches rapid input (e.g., "hello world" arrives as 11 chars
        // within ~1-2ms, all processed before first print at ~12ms)
        let mut buffered_state = DebouncedState::new(Duration::from_millis(10));

        // ==================== Event Loop ====================
        loop {
            tokio::select! {
                // -------- Branch 1: Read next input event --------
                event_result = input_device.next() => {
                    match event_result {
                        Some(event) => {
                            // Reset inactivity watchdog on each event
                            inactivity_watchdog.reset();
                            println!("🔍 PTY Controlled: Event: {event:?}");

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
                                        "{LINE_PREFIX} {}, Cursor: {}",
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

                // -------- Branch 2: Debounce timer expired, print buffered state --------
                // If we should poll the debounced state, then sleep until the debounce timer expires, and when it fires, execute this code.
                () = buffered_state.sleep_until(), if buffered_state.should_poll() => {
                    // No new events arrived within 10ms, print the buffered line state
                    if let Some(state) = buffered_state.take() {
                        println!("{state}");
                        std::io::stdout().flush().expect("Failed to flush");
                    }
                }

                // -------- Branch 3: Inactivity timeout - exit test --------
                () = inactivity_watchdog.sleep_until() => {
                    println!("🔍 PTY Controlled: Inactivity timeout - exiting");
                    break;
                }
            }
        }

        println!("🔍 PTY Controlled: Completed, exiting");
        std::io::stdout().flush().expect("Failed to flush");
    });

}
