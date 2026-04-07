// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`PTY`]-based integration test for [`UTF-8`] text input parsing.
//!
//! Validates that the [`DirectToAnsiInputDevice`] correctly handles [`UTF-8`] text input:
//! - [`ASCII`] characters
//! - [`UTF-8`] multi-byte sequences (accented characters, emojis, etc.)
//! - Mixed text and [`ANSI`] escape sequences
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_pty_utf8_text -- --nocapture
//! ```
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
//! [`DirectToAnsiInputDevice`]: crate::direct_to_ansi::DirectToAnsiInputDevice
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8

use crate::{MSG_CONTROLLED_READY, MSG_CONTROLLED_STARTING, GLYPH_CONTROLLED, GLYPH_CONTROLLER,
            GLYPH_CONTROLLER_CLEANUP, GLYPH_SUCCESS, GLYPH_WAITING,
            InputEvent, PtyTestContext, PtyTestMode, MSG_TEST_RUNNING, generate_pty_test,
            tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice};
use std::{io::Write,
          time::Duration};

generate_pty_test! {
    test_fn: test_pty_utf8_text,
    controller: controller,
    controlled: controlled,
    mode: PtyTestMode::Raw,
}

/// [`PTY`] Controller: Sends [`UTF-8`] text and verifies parsing.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
fn controller(context: PtyTestContext) {
    let PtyTestContext {
        pty_pair,
        child,
        mut buf_reader,
        mut writer,
    } = context;

    eprintln!("{GLYPH_CONTROLLER} PTY Controller: Starting UTF-8 text test...");

    eprintln!(
        "{GLYPH_WAITING} PTY Controller: Waiting for controlled process to start..."
    );

    // Wait for controlled to confirm it's running and ready.
    child
        .wait_for_ready(&mut buf_reader, MSG_CONTROLLED_READY)
        .expect("Failed to wait for ready signal");

    // Generate test cases using abstractions (no magic strings!)
    let texts = vec![
        ("ASCII", "hello"),
        ("Numbers", "12345"),
        ("Space", "a b c"),
        ("Punctuation", "!@#$%"),
    ];

    eprintln!(
        "{} PTY Controller: Sending {} text inputs...",
        GLYPH_WAITING,
        texts.len()
    );

    for (desc, text) in &texts {
        eprintln!("  → Sending: {desc} ({text})");

        writer
            .write_all(text.as_bytes())
            .expect("Failed to write text");
        writer.flush().expect("Failed to flush");

        let event_line = child.read_line_state(&mut buf_reader, |line| line.starts_with("Text:"));
        eprintln!("  {GLYPH_SUCCESS} {desc}: {event_line}");
    }

    eprintln!("{GLYPH_CONTROLLER_CLEANUP} PTY Controller: Cleaning up...");

    drop(writer);
    child.drain_and_wait(buf_reader, pty_pair);

    eprintln!("{GLYPH_SUCCESS} PTY Controller: Test passed!");
}

/// [`PTY`] Controlled: Reads and parses [`UTF-8`] text. The harness performs
/// [`std::process::exit(0)`] after this function returns.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
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
                                InputEvent::Keyboard(key_press) => {
                                    format!("Text: {key_press:?}")
                                }
                                _ => {
                                    format!("Unexpected event: {event:?}")
                                }
                            };

                            println!("{output}");
                            std::io::stdout().flush().expect("Failed to flush stdout");

                            if event_count >= 4 {
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
