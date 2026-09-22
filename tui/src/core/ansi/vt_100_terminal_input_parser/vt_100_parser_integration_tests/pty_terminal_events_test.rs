// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// XMARK: pty test faking terminal emulator (in controller)

//! [`PTY`]-based integration test for terminal event parsing.
//!
//! Validates that [`DirectToAnsiInputDevice`] correctly parses terminal status and state
//! escape sequences in a real pseudoterminal environment:
//! - Window resize sequence notifications (`CSI 8 ; <rows> ; <cols> t`)
//! - Terminal window focus gained notifications (`CSI I`)
//! - Terminal window focus lost notifications (`CSI O`)
//!
//! Note that bracketed paste events are validated separately in
//! [`pty_bracketed_paste_test`], and kernel-level window resizing via signals is
//! validated in [`pty_sigwinch_test`].
//!
//! # Architecture and Process Roles
//!
//! In this test, the controller process simulates a host terminal emulator while the
//! controlled child runs our TUI input engine. For the overall architecture and testing
//! philosophy, see the [process roles documentation].
//!
//! 1. **[`PTY`] Controller (Simulated Terminal Emulator)**:
//!    - Simulates an external terminal emulator host (such as [`Kitty`], Ghostty, or
//!      [`XTerm`]) notifying the running application of window dimension and focus state
//!      changes.
//!    - Synthesizes terminal control escape sequences (`CSI 8 ; 24 ; 80 t`, `CSI I`, `CSI
//!      O`) directly into the controller side of the [`PTY`], which the operating system
//!      delivers to the controlled process's [`stdin`].
//!    - Exercises terminal window resize reporting and window focus gain and loss
//!      transitions.
//!    - Reads line-by-line event confirmation strings from the controlled child's
//!      [`stdout`] and validates decoded `Resize:` and `Focus:` lines.
//!
//! 2. **[`PTY`] Controlled (System Under Test / TUI Input Engine)**:
//!    - Represents the real TUI application running inside the child subprocess connected
//!      to the controlled side of the [`PTY`].
//!    - Initializes [`DirectToAnsiInputDevice`].
//!    - Behind the scenes, the background [`mio`] polling thread watches [`stdin`],
//!      feeding raw bytes into [`ChunkFramer`] and the Sans-IO [`vt_100`] parser.
//!    - Runs a Tokio asynchronous event loop consuming from `input_device.next().await`.
//!    - Maps parsed intermediate representation events into domain [`InputEvent::Resize`]
//!      and [`InputEvent::Focus`] ([`FocusEvent::Gained`] and [`FocusEvent::Lost`]).
//!    - Formats and echoes each received event over [`stdout`] so the controller can
//!      verify parsing accuracy.
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_pty_terminal_events -- --nocapture
//! ```
//!
//! [`ChunkFramer`]: crate::core::ansi::vt_100_terminal_input_parser::ChunkFramer
//! [`DirectToAnsiInputDevice`]: crate::direct_to_ansi::DirectToAnsiInputDevice
//! [`FocusEvent::Gained`]: crate::FocusEvent::Gained
//! [`FocusEvent::Lost`]: crate::FocusEvent::Lost
//! [`InputEvent::Focus`]: crate::InputEvent::Focus
//! [`InputEvent::Resize`]: crate::InputEvent::Resize
//! [`Kitty`]: https://sw.kovidgoyal.net/kitty/
//! [`mio`]: mio
//! [`pty_bracketed_paste_test`]:
//!     mod@crate::core::ansi::vt_100_terminal_input_parser::vt_100_parser_integration_tests::pty_bracketed_paste_test
//! [`pty_sigwinch_test`]:
//!     mod@crate::core::ansi::vt_100_terminal_input_parser::vt_100_parser_integration_tests::pty_sigwinch_test
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`stdin`]: std::io::stdin
//! [`stdout`]: std::io::stdout
//! [`vt_100`]: crate::core::ansi::vt_100_terminal_input_parser
//! [`XTerm`]: https://invisible-island.net/xterm/
//! [process roles documentation]: mod@super#architecture-and-process-roles

use crate::{GLYPH_CONTROLLED, GLYPH_CONTROLLER, GLYPH_CONTROLLER_CLEANUP, GLYPH_SUCCESS,
            GLYPH_WAITING, InputEvent, MSG_CONTROLLED_READY, MSG_CONTROLLED_STARTING,
            MSG_TEST_RUNNING, PtyTestContext, PtyTestMode, VPHeight, VPWidth,
            core::ansi::{generator::generate_keyboard_sequence,
                         vt_100_terminal_input_parser::ir_event_types::{VT100FocusStateIR,
                                                                        VT100InputEventIR}},
            generate_pty_test,
            tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice};
use std::{io::Write, time::Duration};

generate_pty_test! {
    test_fn: test_pty_terminal_events,
    controller: controller,
    controlled: controlled,
    mode: PtyTestMode::Raw,
}

/// [`PTY`] Controller: Send terminal event sequences and verify parsing
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
fn controller(context: PtyTestContext) {
    let PtyTestContext {
        pty_pair,
        child,
        mut buf_reader,
        mut writer,
    } = context;

    eprintln!("{GLYPH_CONTROLLER} PTY Controller: Starting terminal events test...");

    eprintln!(
        "{GLYPH_WAITING} PTY Controller: Waiting for controlled process to start..."
    );

    // Wait for controlled to confirm it's running and ready.
    child
        .wait_for_ready(&mut buf_reader, MSG_CONTROLLED_READY)
        .expect("Failed to wait for ready signal");

    let test_cases = generate_test_cases();

    eprintln!(
        "{} PTY Controller: Sending {} terminal events...",
        GLYPH_WAITING,
        test_cases.len()
    );

    for test_case in &test_cases {
        let TestCase {
            description,
            ir,
            expected_line_prefix,
        } = test_case;

        let sequence = generate_keyboard_sequence(ir)
            .unwrap_or_else(|| panic!("Failed to generate sequence for: {description}"));

        eprintln!("  → Sending: {description} ({sequence:?})");

        writer
            .write_all(&sequence)
            .expect("Failed to write sequence");
        writer.flush().expect("Failed to flush after {description}");

        // Read responses until we get the expected event line.
        let event_line = child.read_line_state(&mut buf_reader, |line| {
            line.starts_with(expected_line_prefix)
        });

        eprintln!("  {GLYPH_SUCCESS} {description}: {event_line}");
    }

    eprintln!("{GLYPH_CONTROLLER_CLEANUP} PTY Controller: Cleaning up...");

    drop(writer);
    child.drain_and_wait(buf_reader, pty_pair);

    eprintln!("{GLYPH_SUCCESS} PTY Controller: Test passed!");
}

/// [`PTY`] Controlled: Read and parse terminal events. The harness performs
/// [`std::process::exit(0)`] after this function returns.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
fn controlled() {
    // Print to stdout immediately to confirm controlled is starting.
    println!("{MSG_TEST_RUNNING}");
    println!("{MSG_CONTROLLED_STARTING}");
    std::io::stdout().flush().expect("Failed to flush");

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Starting...");
        let mut input_device = DirectToAnsiInputDevice::new()
            .expect("Failed to initialize DirectToAnsiInputDevice");
        eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Device created, reading events...");

        // Signal to controller that we're ready to receive input.
        println!("{MSG_CONTROLLED_READY}");
        std::io::stdout().flush().expect("Failed to flush");

        let inactivity_timeout = Duration::from_secs(5);
        let mut inactivity_deadline = tokio::time::Instant::now() + inactivity_timeout;
        let mut event_count = 0;

        loop {
            tokio::select! {
                event_result = input_device.next() => {
                    match event_result {
                        Some(event) => {
                            event_count += 1;
                            inactivity_deadline = tokio::time::Instant::now() + inactivity_timeout;
                            eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Event #{event_count}: {event:?}");

                            let output = match event {
                                InputEvent::Resize(ref size) => {
                                    format!("Resize: {size:?}")
                                }
                                InputEvent::Focus(ref state) => {
                                    format!("Focus: {state:?}")
                                }
                                InputEvent::BracketedPaste(ref text) => {
                                    format!("Paste: {} chars", text.len())
                                }
                                _ => {
                                    format!("Unexpected event: {event:?}")
                                }
                            };

                            println!("{output}");
                            std::io::stdout().flush().expect("Failed to flush stdout");

                            // Exit after processing the expected number of test events
                            if event_count >= TOTAL_EXPECTED_EVENTS {
                                eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Processed {event_count} events, exiting");
                                break;
                            }
                        }
                        None => {
                            eprintln!("{GLYPH_CONTROLLED} PTY Controlled: EOF reached");
                            break;
                        }
                    }
                }
                () = tokio::time::sleep_until(inactivity_deadline) => {
                    eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Inactivity timeout (5 seconds with no events), exiting");
                    break;
                }
            }
        }

        eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Completed, exiting");
    });
}

/// Expected number of terminal events processed by the controlled process:
/// - 1 resize event
/// - 2 focus events (Gained and Lost)
const TOTAL_EXPECTED_EVENTS: usize = 3;

/// A single test case for terminal event verification.
struct TestCase {
    description: &'static str,
    ir: VT100InputEventIR,
    expected_line_prefix: &'static str,
}

/// Generate the structured test cases for terminal event testing.
fn generate_test_cases() -> Vec<TestCase> {
    vec![
        TestCase {
            description: "Window Resize",
            ir: VT100InputEventIR::Resize {
                row_height: VPHeight::from(24),
                col_width: VPWidth::from(80),
            },
            expected_line_prefix: "Resize:",
        },
        TestCase {
            description: "Focus Gained",
            ir: VT100InputEventIR::Focus(VT100FocusStateIR::Gained),
            expected_line_prefix: "Focus:",
        },
        TestCase {
            description: "Focus Lost",
            ir: VT100InputEventIR::Focus(VT100FocusStateIR::Lost),
            expected_line_prefix: "Focus:",
        },
    ]
}
