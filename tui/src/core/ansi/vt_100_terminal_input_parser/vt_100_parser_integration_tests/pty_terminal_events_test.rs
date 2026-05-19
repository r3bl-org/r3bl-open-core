// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`PTY`]-based integration test for terminal event parsing.
//!
//! Validates that the [`DirectToAnsiInputDevice`] correctly parses terminal events:
//! - Window resize notifications (`CSI 8;rows;cols t`)
//! - Focus gained/lost events (`CSI I/O`)
//!
//! Note: Bracketed paste events are tested in [`pty_bracketed_paste_test`] because they
//! require special state machine handling (Start + text + End).
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_pty_terminal_events -- --nocapture
//! ```
//!
//! [`DirectToAnsiInputDevice`]: crate::direct_to_ansi::DirectToAnsiInputDevice
//! [`pty_bracketed_paste_test`]:
//!     mod@crate::vt_100_terminal_input_parser::vt_100_parser_integration_tests::pty_bracketed_paste_test
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal

use crate::{GLYPH_CONTROLLED, MSG_CONTROLLED_READY, MSG_CONTROLLED_STARTING,
            GLYPH_CONTROLLER_CLEANUP, GLYPH_CONTROLLER, InputEvent,
            PtyTestContext, PtyTestMode, GLYPH_SUCCESS, MSG_TEST_RUNNING, GLYPH_WAITING,
            core::ansi::{generator::generate_keyboard_sequence,
                         vt_100_terminal_input_parser::ir_event_types::{VT100FocusStateIR,
                                                                        VT100InputEventIR}},
            generate_pty_test,
            tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice};
use std::{io::Write,
          time::Duration};

generate_pty_test! {
    test_fn: test_pty_terminal_events,
    controller: controller,
    controlled: controlled,
    mode: PtyTestMode::Raw,
}

/// [`PTY`] Controller: Send terminal event sequences and verify parsing
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
#[allow(clippy::too_many_lines)]
fn controller(context: PtyTestContext) {
    let PtyTestContext {
        pty_pair,
        child,
        mut buf_reader,
        mut writer,
    } = context;

    eprintln!("{GLYPH_CONTROLLER} PTY Controller: Starting terminal events test...");

    eprintln!("{GLYPH_WAITING} PTY Controller: Waiting for controlled process to start...");

    // Wait for controlled to confirm it's running and ready.
    child
        .wait_for_ready(&mut buf_reader, MSG_CONTROLLED_READY)
        .expect("Failed to wait for ready signal");

    // Generate test cases using abstractions (no magic strings!)
    // Note: Paste events are tested separately in pty_bracketed_paste_test.rs
    // because they require special handling (Start + text + End = single event)
    let events = vec![
        (
            "Window Resize",
            VT100InputEventIR::Resize {
                row_height: crate::RowHeight::from(24),
                col_width: crate::ColWidth::from(80),
            },
        ),
        (
            "Focus Gained",
            VT100InputEventIR::Focus(VT100FocusStateIR::Gained),
        ),
        (
            "Focus Lost",
            VT100InputEventIR::Focus(VT100FocusStateIR::Lost),
        ),
    ];

    eprintln!(
        "{} PTY Controller: Sending {} terminal events...",
        GLYPH_WAITING,
        events.len()
    );

    for (desc, event) in &events {
        let sequence = generate_keyboard_sequence(event)
            .unwrap_or_else(|| panic!("Failed to generate sequence for: {desc}"));

        eprintln!("  → Sending: {desc} ({sequence:?})");

        writer
            .write_all(&sequence)
            .expect("Failed to write sequence");
        writer.flush().expect("Failed to flush after {desc}");

        // Read responses until we get an event line. The controlled process
        // responds immediately after receiving input, so blocking reads work.
        let event_line = child.read_line_state(&mut buf_reader, |line| {
            line.starts_with("Resize:")
                || line.starts_with("Focus:")
                || line.starts_with("Paste:")
        });

        eprintln!("  {GLYPH_SUCCESS} {desc}: {event_line}");
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

                            // Exit after processing the expected number of test events (3)
                            if event_count >= 3 {
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
