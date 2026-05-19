// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{AsyncDebouncedDeadline, CONTROL_C, DebouncedState, GLYPH_CONTROLLED,
            GLYPH_CONTROLLER_CLEANUP, GLYPH_SUCCESS, KeyState, MSG_CONTROLLED_READY,
            MSG_LINE_PREFIX, PtyTestContext, SegIndex, Size,
            core::test_fixtures::StdoutMock, direct_to_ansi::DirectToAnsiInputDevice,
            height, readline_async::readline_async_impl::LineState, width};
use std::{io::{Write, stdout},
          sync::{Arc, Mutex as StdMutex},
          time::Duration};

/// Helper for the `controlled` process in [`readline_async`] [`PTY`] tests. This sets up
/// the standard event loop with:
/// 1. An [inactivity watchdog] (5s).
/// 2. Debounced state reporting.
/// 3. Immediate exit on [`CONTROL_C`] signal from the controller.
///
/// # Test Protocol (Request-Response Pattern)
///
/// This loop implements the "controlled" side of a **request-response protocol** used by
/// all [`readline_async`] [`PTY`] integration tests:
///
/// 1. **Controller sends input**: The `controller` sends bytes (text or control
///    sequences) via the [`PTY`] writer.
/// 2. **Controller blocks**: The `controller` blocks reading the `controlled` [`stdout`]
///    until it sees a state output prefixed with [`MSG_LINE_PREFIX`].
/// 3. **Controlled processes**: This loop reads the input, applies it to the
///    [`LineState`], and debounces the state reporting.
/// 4. **Controlled responds**: This loop outputs the final debounced line state, allowing
///    the `controller` to unblock.
/// 5. **Controller asserts**: The `controller` verifies the final state.
///
/// **Critical requirement**: The `controlled` process must output line state **only
/// once** after processing all available rapid input, not after every character.
/// Otherwise, the `controller` will read intermediate states (e.g., "Line: h, Cursor: 1"
/// instead of "Line: hello world, Cursor: 11") and fail its assertions. This is achieved
/// via [`DebouncedState`].
///
/// # Panics
/// Panics if it fails to initialize the input device or flush [`stdout`].
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`readline_async`]: crate::readline_async
/// [`stdout`]: std::io::stdout
/// [inactivity watchdog]: crate::AsyncDebouncedDeadline
pub fn readline_async_controlled_loop(initial_text: &str, initial_cursor: SegIndex) {
    let mut line_state = LineState::new(
        initial_text.to_string(),
        Size::new((width(100), height(100))),
    );
    line_state.line_cursor_grapheme = initial_cursor;

    let mut input_device = DirectToAnsiInputDevice::new()
        .expect("Failed to initialize DirectToAnsiInputDevice");

    // Signal readiness
    println!("{MSG_CONTROLLED_READY}");
    stdout().flush().expect("Failed to flush");

    let mut inactivity_watchdog = AsyncDebouncedDeadline::new(Duration::from_secs(5));
    let mut debounced_state = DebouncedState::new(Duration::from_millis(10));
    let safe_output_terminal = Arc::new(StdMutex::new(StdoutMock::default()));
    let (history, _) = crate::readline_async::readline_async_impl::History::new();
    let safe_history = Arc::new(StdMutex::new(history));

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    runtime.block_on(async {
        loop {
            tokio::select! {
                // Inactivity watchdog
                () = inactivity_watchdog.sleep_until() => {
                    break;
                }

                // Debounced state reporting
                () = debounced_state.sleep_until(), if debounced_state.should_poll() => {
                    if let Some(result) = debounced_state.take() {
                        println!("{MSG_LINE_PREFIX}{result}");
                        stdout().flush().expect("Failed to flush stdout");
                    }
                }

                // Input event handling
                event_result = input_device.next() => {
                    match event_result {
                        Some(event) => {
                            // Reset inactivity watchdog on each event
                            inactivity_watchdog.reset();
                            println!("{GLYPH_CONTROLLED} PTY Controlled: Event: {event:?}");

                            // Exit on Ctrl+C signal from controller
                            if matches!(
                                &event,
                                crate::InputEvent::Keyboard(crate::KeyPress::WithModifiers {
                                    key: crate::Key::Character('c'),
                                    mask,
                                }) if mask.ctrl_key_state == KeyState::Pressed
                            ) {
                                println!(
                                    "{GLYPH_CONTROLLED} PTY Controlled: Ctrl+C received, exiting"
                                );
                                break;
                            }

                            // Apply event
                            let result = line_state.apply_event_and_render(
                                &event,
                                &mut *safe_output_terminal.lock().unwrap(),
                                &safe_history,
                            );

                            match result {
                                Ok(None) => {
                                    debounced_state.set(format!(
                                        " {}, Cursor: {}",
                                        line_state.line,
                                        line_state.line_cursor_grapheme
                                    ));
                                }
                                Ok(Some(readline_event)) => {
                                    println!("{GLYPH_CONTROLLED} PTY Controlled: ReadlineEvent: {readline_event:?}");
                                    // For EOF, print it and exit
                                    if matches!(readline_event, crate::ReadlineEvent::Eof) {
                                        println!("EOF");
                                        stdout().flush().expect("Failed to flush");
                                        break;
                                    }
                                }
                                Err(e) => {
                                    println!("{GLYPH_CONTROLLED} PTY Controlled: Error: {e:?}");
                                }
                            }
                        }
                        None => {
                            break;
                        }
                    }
                }
            }
        }
    });
}

/// Helper for the `controller` process cleanup in [`readline_async`] [`PTY`] tests. This
/// sends the [`CONTROL_C`] signal to the controlled process and then drains the [`PTY`].
///
/// # Panics
/// Panics if it fails to send the [`CONTROL_C`] signal or flush the writer.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`readline_async`]: crate::readline_async
pub fn readline_async_controller_exit(mut context: PtyTestContext) {
    eprintln!("{GLYPH_CONTROLLER_CLEANUP} PTY Controller: Cleaning up...");
    // Signal controlled process to exit immediately
    context
        .writer
        .write_all(&[CONTROL_C])
        .expect("Failed to send Ctrl+C");
    context.writer.flush().expect("Failed to flush");
    drop(context.writer);
    context
        .child
        .drain_and_wait(context.buf_reader, context.pty_pair);
    eprintln!("{GLYPH_SUCCESS} PTY Controller: Test passed!");
}
