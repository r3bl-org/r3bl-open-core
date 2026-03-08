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
//! Run with:
//! ```bash
//! cargo test -p r3bl_tui --lib test_pty_terminal_events -- --nocapture
//! ```
//!
//! [`DirectToAnsiInputDevice`]: crate::direct_to_ansi::DirectToAnsiInputDevice
//! [`pty_bracketed_paste_test`]:
//!     mod@crate::vt_100_terminal_input_parser::integration_tests::pty_bracketed_paste_test
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal

use crate::{CONTROLLED_READY, CONTROLLED_STARTING, InputEvent, PtyTestMode,
            PtyTestContext, TEST_RUNNING,
            core::ansi::{generator::generate_keyboard_sequence,
                         vt_100_terminal_input_parser::ir_event_types::{VT100FocusStateIR,
                                                                        VT100InputEventIR}},
            tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice};
use std::{io::{BufRead, Write},
          time::Duration};

// XMARK: Process isolated test with PTY.

generate_pty_test! {
    test_fn: test_pty_terminal_events,
    controller: pty_controller_entry_point,
    controlled: pty_controlled_entry_point,
    mode: PtyTestMode::Raw,
}

/// [`PTY`] Controller: Send terminal event sequences and verify parsing
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

    eprintln!("🚀 PTY Controller: Starting terminal events test...");

    eprintln!("📝 PTY Controller: Waiting for controlled process to start...");

    // Wait for controlled to confirm it's running and ready.
    child.wait_for_ready(&mut buf_reader, CONTROLLED_READY)
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
        "📝 PTY Controller: Sending {} terminal events...",
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
        let event_line = loop {
            let mut line = String::new();
            match buf_reader.read_line(&mut line) {
                Ok(0) => panic!("EOF reached before receiving event for {desc}"),
                Ok(_) => {
                    let trimmed = line.trim();

                    // Check if it's an event line
                    if trimmed.starts_with("Resize:")
                        || trimmed.starts_with("Focus:")
                        || trimmed.starts_with("Paste:")
                    {
                        break trimmed.to_string();
                    }

                    // Skip test harness noise
                    eprintln!("  ⚠️  Skipping non-event output: {trimmed}");
                }
                Err(e) => panic!("Read error for {desc}: {e}"),
            }
        };

        eprintln!("  ✓ {desc}: {event_line}");
    }

    eprintln!("🧹 PTY Controller: Cleaning up...");

    drop(writer);
    child.drain_and_wait(buf_reader, pty_pair);

    eprintln!("✅ PTY Controller: Test passed!");
}

/// [`PTY`] Controlled: Read and parse terminal events
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
fn pty_controlled_entry_point() {
    // Print to stdout immediately to confirm controlled is starting.
    println!("{TEST_RUNNING}");
    println!("{CONTROLLED_STARTING}");
    std::io::stdout().flush().expect("Failed to flush");

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        eprintln!("🔍 PTY Controlled: Starting...");
        let mut input_device = DirectToAnsiInputDevice::new();
        eprintln!("🔍 PTY Controlled: Device created, reading events...");

        // Signal to controller that we're ready to receive input.
        println!("{CONTROLLED_READY}");
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
                            eprintln!("🔍 PTY Controlled: Event #{event_count}: {event:?}");

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
                                eprintln!("🔍 PTY Controlled: Processed {event_count} events, exiting");
                                break;
                            }
                        }
                        None => {
                            eprintln!("🔍 PTY Controlled: EOF reached");
                            break;
                        }
                    }
                }
                () = tokio::time::sleep_until(inactivity_deadline) => {
                    eprintln!("🔍 PTY Controlled: Inactivity timeout (5 seconds with no events), exiting");
                    break;
                }
            }
        }

        eprintln!("🔍 PTY Controlled: Completed, exiting");
    });

}
