// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words ello

use crate::{AsyncDebouncedDeadline, CONTROLLED_READY, DebouncedState, LINE_PREFIX, PtyTestMode, PtyTestContext,
            core::test_fixtures::StdoutMock, direct_to_ansi::DirectToAnsiInputDevice,
            readline_async::readline_async_impl::LineState};
use std::{io::Write,
          sync::{Arc, Mutex as StdMutex},
          time::Duration};

generate_pty_test! {
    /// [`PTY`]-based integration test for Ctrl+D delete character behavior.
    ///
    /// Validates that Ctrl+D on a non-empty line deletes the character at cursor position.
    ///
    /// Run with:
    /// ```bash
    /// cargo test -p r3bl_tui --lib test_pty_ctrl_d_delete -- --nocapture
    /// ```
    ///
    /// ## Test Protocol (Request-Response Pattern)
    ///
    /// This test uses a **request-response protocol** between controller and controlled:
    ///
    /// 1. **Controller sends input** (text and Ctrl+D sequences)
    /// 2. **Controller flushes** and blocks reading controlled stdout until it sees
    ///    "Line: ..."
    /// 3. **Controller makes assertion** on the line state
    /// 4. **Repeat** for next input sequence
    ///
    /// The ([`LineState`]) is checked in the test to make assertions against.
    ///
    /// [`LineState`]: crate::readline_async::readline_async_impl::LineState
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    test_fn: test_pty_ctrl_d_delete,
    controller: pty_controller_entry_point,
    controlled: pty_controlled_entry_point,
    mode: PtyTestMode::Raw,
}

/// [`PTY`] Controller: Send Ctrl+D on non-empty line and verify delete behavior
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

    eprintln!("🚀 PTY Controller: Starting Ctrl+D delete test...");

    eprintln!("📝 PTY Controller: Waiting for controlled process to start...");

    // Wait for controlled to confirm it's ready.
    child
        .wait_for_ready(&mut buf_reader, CONTROLLED_READY)
        .expect("Controlled never signaled ready");

    eprintln!("  ✓ Controlled is ready (input device created)");

    // Test: Ctrl+D on non-empty line → delete character at cursor
    eprintln!("📝 PTY Controller: Sending 'hello'...");
    writer.write_all(b"hello").expect("Failed to write text");
    writer.flush().expect("Failed to flush");

    let result = child.read_line_state(&mut buf_reader, LINE_PREFIX);
    eprintln!("  ← Line state: {result}");
    assert_eq!(result, "Line: hello, Cursor: 5");

    // Move cursor to beginning with Ctrl+A
    eprintln!("📝 PTY Controller: Sending Ctrl+A (move to beginning)...");
    writer.write_all(&[0x01]).expect("Failed to write Ctrl+A");
    writer.flush().expect("Failed to flush");

    let result = child.read_line_state(&mut buf_reader, LINE_PREFIX);
    eprintln!("  ← After Ctrl+A: {result}");
    assert_eq!(result, "Line: hello, Cursor: 0");

    // Send Ctrl+D to delete 'h'
    eprintln!("📝 PTY Controller: Sending Ctrl+D (delete character at cursor)...");
    writer.write_all(&[0x04]).expect("Failed to write Ctrl+D");
    writer.flush().expect("Failed to flush");

    let result = child.read_line_state(&mut buf_reader, LINE_PREFIX);
    eprintln!("  ← After Ctrl+D: {result}");
    assert_eq!(result, "Line: ello, Cursor: 0");

    eprintln!("✅ PTY Controller: Ctrl+D delete test passed!");

    // Clean shutdown - close writer to signal controlled to exit
    eprintln!("🧹 PTY Controller: Cleaning up...");
    drop(writer);
    child.drain_and_wait(buf_reader, pty_pair);
    eprintln!("✅ PTY Controller: Test passed!");
}

/// [`PTY`] Controlled: Process readline input and report line state
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
fn pty_controlled_entry_point() {
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
        // This batches rapid input (e.g., "hello" arrives as 5 chars
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

                                    // For this test, we don't exit on EOF
                                    // We only test delete character behavior
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
