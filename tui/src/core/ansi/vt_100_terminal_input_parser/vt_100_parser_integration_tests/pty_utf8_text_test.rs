// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// XMARK: pty test faking terminal emulator (in controller)

//! [`PTY`]-based integration test for [`UTF-8`] text input parsing.
//!
//! Validates that [`DirectToAnsiInputDevice`] correctly handles [`UTF-8`] text input in a
//! real pseudoterminal environment:
//! - Phase 1 (Isolated single characters across all variable-length [`UTF-8`] byte
//!   lengths):
//!   - [`ASCII`] characters (1-byte: `'a'`, `'Z'`, `'9'`, `'!'`)
//!   - Latin-1 and Cyrillic characters (2-byte: `'é'`, `'ñ'`, `'ü'`, `'д'`)
//!   - CJK ideographs (3-byte: `'中'`, `'文'`, `'日'`, `'本'`)
//!   - Astral Plane characters and emojis (4-byte: `'🦀'`, `'🚀'`, `'🎉'`, `'✨'`)
//! - Phase 2 (Multi-character strings streamed in contiguous chunks):
//!   - Accented text (`"café"`)
//!   - Cyrillic text (`"мир"`)
//!   - CJK text (`"你好"`)
//!   - Multi-byte emoji sequence (`"🦀🚀"`)
//!
//! # Architecture and Process Roles
//!
//! In this test, the controller process simulates a host terminal emulator while the
//! controlled child runs our TUI input engine. For the overall architecture and testing
//! philosophy, see the [process roles documentation].
//!
//! 1. **[`PTY`] Controller (Simulated Terminal Emulator)**:
//!    - Simulates an external terminal emulator host (such as [`Kitty`], Ghostty, or
//!      [`XTerm`]) transmitting raw [`UTF-8`] byte streams to the terminal application.
//!    - Encodes single [`char`] scalars into variable-length [`UTF-8`] byte slices and
//!      streams multi-character byte arrays directly into the controller side of the
//!      [`PTY`], which the operating system delivers to the controlled process's
//!      [`stdin`].
//!    - Exercises 1-byte, 2-byte, 3-byte, and 4-byte boundaries as well as multi-byte
//!      streaming sequences.
//!    - Reads line-by-line event confirmation strings from the controlled child's
//!      [`stdout`] and asserts that decoded [`Unicode`] characters and hex values match
//!      expected values.
//!
//! 2. **[`PTY`] Controlled (System Under Test / TUI Input Engine)**:
//!    - Represents the real TUI application running inside the child subprocess connected
//!      to the controlled side of the [`PTY`].
//!    - Initializes [`DirectToAnsiInputDevice`].
//!    - Behind the scenes, the background [`mio`] polling thread watches [`stdin`],
//!      feeding raw bytes into [`ChunkFramer`] and the Sans-IO [`vt_100`] parser.
//!    - Accurately reassembles multi-byte [`UTF-8`] sequences into discrete [`Unicode`]
//!      scalar values without splitting codepoints or dropping bytes.
//!    - Runs a Tokio asynchronous event loop consuming from `input_device.next().await`.
//!    - Maps parsed intermediate representation text events into domain
//!      [`InputEvent::Keyboard`] structures with [`Key::Character`].
//!    - Formats and echoes each received character line over [`stdout`] so the controller
//!      can verify parsing accuracy.
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_pty_utf8_text -- --nocapture
//! ```
//!
//! [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
//! [`ChunkFramer`]: crate::core::ansi::vt_100_terminal_input_parser::ChunkFramer
//! [`DirectToAnsiInputDevice`]: crate::direct_to_ansi::DirectToAnsiInputDevice
//! [`InputEvent::Keyboard`]: crate::InputEvent::Keyboard
//! [`Key::Character`]: crate::Key::Character
//! [`Kitty`]: https://sw.kovidgoyal.net/kitty/
//! [`mio`]: mio
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`stdin`]: std::io::stdin
//! [`stdout`]: std::io::stdout
//! [`Unicode`]: https://en.wikipedia.org/wiki/Unicode
//! [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
//! [`vt_100`]: crate::core::ansi::vt_100_terminal_input_parser
//! [`XTerm`]: https://invisible-island.net/xterm/
//! [process roles documentation]: mod@super#architecture-and-process-roles

use crate::{GLYPH_CONTROLLED, GLYPH_CONTROLLER, GLYPH_CONTROLLER_CLEANUP, GLYPH_SUCCESS,
            GLYPH_WAITING, InputEvent, MSG_CONTROLLED_READY, MSG_CONTROLLED_STARTING,
            MSG_TEST_RUNNING, PtyTestContext, PtyTestMode, generate_pty_test,
            tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice};
use std::{io::Write, time::Duration};

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

    // Phase 1: Isolated single characters across 1, 2, 3, and 4 byte UTF-8 encodings.
    let single_char_cases = generate_single_char_test_cases();

    eprintln!(
        "{} PTY Controller: Sending {} single UTF-8 characters...",
        GLYPH_WAITING,
        single_char_cases.len()
    );

    for test_case in &single_char_cases {
        let SingleCharTestCase {
            description,
            character,
        } = test_case;
        let mut utf8_buf = [0u8; 4];
        let utf8_bytes = character.encode_utf8(&mut utf8_buf).as_bytes();
        eprintln!("  → Sending: {description} ({utf8_bytes:?})");

        writer
            .write_all(utf8_bytes)
            .expect("Failed to write character");
        writer.flush().expect("Failed to flush");

        let event_line =
            child.read_line_state(&mut buf_reader, |line| line.starts_with("Text:"));
        assert!(
            event_line.contains(*character),
            "Expected event line for '{description}' to contain '{character}', got '{event_line}'"
        );
        eprintln!("  {GLYPH_SUCCESS} {description}: {event_line}");
    }

    // Phase 2: Multi-character strings streamed in contiguous chunks.
    let multi_char_cases = generate_multi_char_test_cases();

    let total_streamed_chars: usize = multi_char_cases
        .iter()
        .map(|case| case.text.chars().count())
        .sum();

    assert_eq!(
        single_char_cases.len() + total_streamed_chars,
        TOTAL_EXPECTED_EVENTS
    );

    eprintln!(
        "{} PTY Controller: Sending {} multi-character UTF-8 strings ({} total chars)...",
        GLYPH_WAITING,
        multi_char_cases.len(),
        total_streamed_chars
    );

    for test_case in &multi_char_cases {
        let MultiCharTestCase { description, text } = test_case;
        eprintln!("  → Sending: {description} ({text})");

        writer
            .write_all(text.as_bytes())
            .expect("Failed to write text string");
        writer.flush().expect("Failed to flush");

        for expected_ch in text.chars() {
            let event_line =
                child.read_line_state(&mut buf_reader, |line| line.starts_with("Text:"));
            assert!(
                event_line.contains(expected_ch),
                "Expected event line in '{description}' for character '{expected_ch}', got '{event_line}'"
            );
            eprintln!(
                "  {GLYPH_SUCCESS} {description} char '{expected_ch}': {event_line}"
            );
        }
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

/// Expected number of keyboard text events processed by the controlled process:
/// - 16 single [`UTF-8`] characters
/// - 11 multi-character string characters (café: 4, мир: 3, 你好: 2, 🦀🚀: 2)
///
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
const TOTAL_EXPECTED_EVENTS: usize = 27;

/// A single-character test case for [`UTF-8`] encoding verification.
///
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
struct SingleCharTestCase {
    description: &'static str,
    character: char,
}

/// A multi-character text test case for streaming [`UTF-8`] verification.
///
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
struct MultiCharTestCase {
    description: &'static str,
    text: &'static str,
}

/// Generate the structured single-character test cases across 1, 2, 3, and 4 byte
/// [`UTF-8`] encodings.
///
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
fn generate_single_char_test_cases() -> Vec<SingleCharTestCase> {
    vec![
        // ASCII (1-byte)
        SingleCharTestCase {
            description: "ASCII lowercase 'a'",
            character: 'a',
        },
        SingleCharTestCase {
            description: "ASCII uppercase 'Z'",
            character: 'Z',
        },
        SingleCharTestCase {
            description: "ASCII digit '9'",
            character: '9',
        },
        SingleCharTestCase {
            description: "ASCII punctuation '!'",
            character: '!',
        },
        // Latin-1 / Cyrillic (2-byte)
        SingleCharTestCase {
            description: "Accented 'é'",
            character: 'é',
        },
        SingleCharTestCase {
            description: "Accented 'ñ'",
            character: 'ñ',
        },
        SingleCharTestCase {
            description: "Accented 'ü'",
            character: 'ü',
        },
        SingleCharTestCase {
            description: "Cyrillic 'д'",
            character: 'д',
        },
        // CJK (3-byte)
        SingleCharTestCase {
            description: "CJK '中'",
            character: '中',
        },
        SingleCharTestCase {
            description: "CJK '文'",
            character: '文',
        },
        SingleCharTestCase {
            description: "CJK '日'",
            character: '日',
        },
        SingleCharTestCase {
            description: "CJK '本'",
            character: '本',
        },
        // Astral Plane / Emojis (4-byte)
        SingleCharTestCase {
            description: "Crab emoji '🦀'",
            character: '🦀',
        },
        SingleCharTestCase {
            description: "Rocket emoji '🚀'",
            character: '🚀',
        },
        SingleCharTestCase {
            description: "Party Popper emoji '🎉'",
            character: '🎉',
        },
        SingleCharTestCase {
            description: "Sparkles emoji '✨'",
            character: '✨',
        },
    ]
}

/// Generate the structured multi-character string test cases streamed in contiguous
/// chunks.
fn generate_multi_char_test_cases() -> Vec<MultiCharTestCase> {
    vec![
        MultiCharTestCase {
            description: "Accented text",
            text: "café",
        },
        MultiCharTestCase {
            description: "Cyrillic text",
            text: "мир",
        },
        MultiCharTestCase {
            description: "CJK text",
            text: "你好",
        },
        MultiCharTestCase {
            description: "Emoji sequence",
            text: "🦀🚀",
        },
    ]
}
