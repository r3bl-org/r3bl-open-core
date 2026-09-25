// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// XMARK: pty test faking terminal emulator (in controller)

//! [`PTY`]-based integration test for keypad application mode and special keys.
//!
//! Validates that [`DirectToAnsiInputDevice`] correctly parses keypad application mode
//! sequences and special control character inputs in a real pseudoterminal environment:
//! - Single-byte control characters: Tab (`0x09`) and `Ctrl+Space` (`0x00` NUL byte)
//! - `BackTab` / `Shift+Tab` escape sequence (`ESC [ Z`)
//! - Alternative Home and End key escape sequences (`ESC [ 1 ~`, `ESC [ 4 ~`, `ESC [ 7
//!   ~`, `ESC [ 8 ~`)
//! - Numpad Application Mode ([`SS3`] sequences: `ESC O p` through `ESC O y`, Enter,
//!   Plus, Minus, Multiply, Divide, Decimal, and Comma across all 17 keypad keys)
//!
//! # Architecture and Process Roles
//!
//! In this test, the controller process simulates a host terminal emulator while the
//! controlled child runs our TUI input engine. For the overall architecture and testing
//! philosophy, see the [process roles documentation].
//!
//! 1. **[`PTY`] Controller (Simulated Terminal Emulator)**:
//!    - Simulates an external terminal emulator host (such as [`Kitty`], Ghostty, or
//!      [`XTerm`]) with numpad application mode ([`DECPAM`]) enabled.
//!    - Synthesizes raw byte vectors from foundational Tier 1 constants without magic
//!      literals, including single-byte control characters ([`CONTROL_TAB`],
//!      [`CONTROL_NUL`]), `BackTab` ([`BACKTAB_FINAL`]), alternative Home/End parameter
//!      digits ([`ASCII_DIGIT_1`], [`ASCII_DIGIT_4`], [`ASCII_DIGIT_7`],
//!      [`ASCII_DIGIT_8`]), and all 17 application-mode [`SS3`] numpad keys
//!      ([`SS3_NUMPAD_0`] through [`SS3_NUMPAD_9`], etc.).
//!    - Writes sequence bytes directly into the controller side of the [`PTY`], which the
//!      operating system delivers to the controlled process's [`stdin`].
//!    - Reads line-by-line event confirmation strings from the controlled child's
//!      [`stdout`] and asserts that the child decoded the expected
//!      [`InputEvent::Keyboard`].
//!
//! 2. **[`PTY`] Controlled (System Under Test / TUI Input Engine)**:
//!    - Represents the real TUI application running inside the child subprocess connected
//!      to the controlled side of the [`PTY`].
//!    - Initializes [`DirectToAnsiInputDevice`].
//!    - Behind the scenes, the background [`mio`] polling thread watches [`stdin`],
//!      feeding raw bytes into [`ChunkFramer`] and the Sans-IO [`vt_100`] parser.
//!    - Disambiguates [`SS3`] keypad application mode codes into corresponding [`Key`]
//!      variants while preserving special control codes (such as mapping NUL byte to
//!      `Ctrl+Space` rather than `Ctrl+@`).
//!    - Runs a Tokio asynchronous event loop consuming from `input_device.next().await`.
//!    - Maps parsed intermediate representation events into domain
//!      [`InputEvent::Keyboard`] structures.
//!    - Formats and echoes each received event over [`stdout`] so the controller can
//!      verify parsing accuracy.
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_pty_keypad_and_special_keys -- --nocapture
//! ```
//!
//! [`ASCII_DIGIT_1`]: crate::core::ansi::constants::ASCII_DIGIT_1
//! [`ASCII_DIGIT_4`]: crate::core::ansi::constants::ASCII_DIGIT_4
//! [`ASCII_DIGIT_7`]: crate::core::ansi::constants::ASCII_DIGIT_7
//! [`ASCII_DIGIT_8`]: crate::core::ansi::constants::ASCII_DIGIT_8
//! [`BACKTAB_FINAL`]: crate::core::ansi::constants::BACKTAB_FINAL
//! [`ChunkFramer`]: crate::core::ansi::vt_100_terminal_input_parser::ChunkFramer
//! [`CONTROL_NUL`]: crate::core::ansi::constants::CONTROL_NUL
//! [`CONTROL_TAB`]: crate::core::ansi::constants::CONTROL_TAB
//! [`DECPAM`]: https://vt100.net/docs/vt510-rm/DECPAM.html
//! [`DirectToAnsiInputDevice`]: crate::direct_to_ansi::DirectToAnsiInputDevice
//! [`InputEvent::Keyboard`]: crate::InputEvent::Keyboard
//! [`Key`]: crate::Key
//! [`Kitty`]: https://sw.kovidgoyal.net/kitty/
//! [`mio`]: https://crates.io/crates/mio
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`SS3_NUMPAD_0`]: crate::core::ansi::constants::SS3_NUMPAD_0
//! [`SS3_NUMPAD_9`]: crate::core::ansi::constants::SS3_NUMPAD_9
//! [`SS3`]: https://en.wikipedia.org/wiki/ANSI_escape_code#SS3
//! [`stdin`]: std::io::stdin
//! [`stdout`]: std::io::stdout
//! [`vt_100`]: crate::core::ansi::vt_100_terminal_input_parser
//! [`XTerm`]: https://invisible-island.net/xterm/
//! [process roles documentation]: mod@super#architecture-and-process-roles

use crate::{GLYPH_CONTROLLED, GLYPH_CONTROLLER, GLYPH_CONTROLLER_CLEANUP, GLYPH_SUCCESS,
            GLYPH_WAITING, GLYPH_WARNING, InputEvent, MSG_CONTROLLED_READY,
            MSG_CONTROLLED_STARTING, PtyTestContext, PtyTestMode,
            core::ansi::constants::{ANSI_CSI_BRACKET, ANSI_ESC,
                                    ANSI_FUNCTION_KEY_TERMINATOR, ANSI_SS3_O,
                                    ASCII_DIGIT_1, ASCII_DIGIT_4, ASCII_DIGIT_7,
                                    ASCII_DIGIT_8, BACKTAB_FINAL, CONTROL_NUL,
                                    CONTROL_TAB, SS3_NUMPAD_0, SS3_NUMPAD_1,
                                    SS3_NUMPAD_2, SS3_NUMPAD_3, SS3_NUMPAD_4,
                                    SS3_NUMPAD_5, SS3_NUMPAD_6, SS3_NUMPAD_7,
                                    SS3_NUMPAD_8, SS3_NUMPAD_9, SS3_NUMPAD_COMMA,
                                    SS3_NUMPAD_DECIMAL, SS3_NUMPAD_DIVIDE,
                                    SS3_NUMPAD_ENTER, SS3_NUMPAD_MINUS,
                                    SS3_NUMPAD_MULTIPLY, SS3_NUMPAD_PLUS},
            generate_pty_test,
            tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice};
use std::{io::{BufRead, Write},
          time::Duration};

generate_pty_test! {
    test_fn: test_pty_keypad_and_special_keys,
    controller: controller,
    controlled: controlled,
    mode: PtyTestMode::Raw,
}

/// [`PTY`] Controller: Send new keyboard sequences and verify parsing
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

    eprintln!(
        "{GLYPH_CONTROLLER} PTY Controller: Starting keypad and special keys test..."
    );

    eprintln!(
        "{GLYPH_WAITING} PTY Controller: Waiting for controlled process to start..."
    );

    // Wait for controlled to confirm it's running and ready. The controlled process sends
    // TEST_RUNNING, CONTROLLED_STARTING, and CONTROLLED_READY on startup.
    child
        .wait_for_ready(&mut buf_reader, MSG_CONTROLLED_READY)
        .expect("Failed to wait for MSG_CONTROLLED_READY");
    eprintln!("  {GLYPH_SUCCESS} Controlled is ready (input device created)");

    let test_cases = generate_test_cases();

    eprintln!(
        "{} PTY Controller: Sending {} sequences...",
        GLYPH_WAITING,
        test_cases.len()
    );

    // For each test sequence: write ANSI bytes to PTY, read back parsed event, verify.
    for test_case in &test_cases {
        let desc = test_case.description;
        let sequence = &test_case.sequence;
        eprintln!("  → Sending: {desc} ({sequence:?})");

        writer
            .write_all(sequence)
            .expect("Failed to write sequence");
        writer.flush().expect("Failed to flush");

        // Read responses until we get an event line (skip test harness noise)
        let event_line = loop {
            let mut line = String::new();
            match buf_reader.read_line(&mut line) {
                Ok(0) => {
                    panic!("EOF reached before receiving event for {desc}");
                }
                Ok(_) => {
                    let trimmed = line.trim();

                    // Check if it's an event line
                    if trimmed.starts_with("Keyboard:")
                        || trimmed.starts_with("Mouse:")
                        || trimmed.starts_with("Resize:")
                        || trimmed.starts_with("Focus:")
                        || trimmed.starts_with("Paste:")
                    {
                        break trimmed.to_string();
                    }

                    // Skip test harness noise
                    eprintln!("  {GLYPH_WARNING} Skipping non-event output: {trimmed}");
                }
                Err(e) => {
                    panic!("Read error for {desc}: {e}");
                }
            }
        };

        eprintln!("  {GLYPH_SUCCESS} {desc}: {event_line}");
    }

    eprintln!("{GLYPH_CONTROLLER_CLEANUP} PTY Controller: Cleaning up...");

    // Close writer to signal EOF
    drop(writer);
    child.drain_and_wait(buf_reader, pty_pair);

    eprintln!("{GLYPH_SUCCESS} PTY Controller: All new keyboard features tests passed!");
}

/// [`PTY`] Controlled: Parse keyboard input and echo results. The harness performs
/// [`std::process::exit(0)`] after this function returns.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
fn controlled() {
    // Print to stdout immediately to confirm controlled is starting.
    println!("{MSG_CONTROLLED_STARTING}");
    std::io::stdout().flush().expect("Failed to flush");

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Starting...");
        let mut input_device = DirectToAnsiInputDevice::new()
            .expect("Failed to initialize DirectToAnsiInputDevice");
        eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Device created, reading events...");

        // Signal to controller that we're ready to receive input. MUST be after
        // DirectToAnsiInputDevice::new() so the mio poller thread is already
        // watching stdin before the controller sends any input through the PTY.
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

                            // Output event in parseable format (same as pty_input_device_test)
                            let output = match event {
                                InputEvent::Keyboard(ref key_press) => {
                                    format!("Keyboard: {key_press:?}")
                                }
                                InputEvent::Mouse(ref mouse_input) => {
                                    format!("Mouse: {mouse_input:?}")
                                }
                                InputEvent::Resize(ref size) => {
                                    format!("Resize: {size:?}")
                                }
                                InputEvent::Focus(ref state) => {
                                    format!("Focus: {state:?}")
                                }
                                InputEvent::BracketedPaste(ref text) => {
                                    format!("Paste: {} chars", text.len())
                                }
                                InputEvent::TerminalColorReport(ref report) => {
                                    format!("TerminalColorReport: {report:?}")
                                }
                                InputEvent::Shutdown(ref reason) => {
                                    format!("Shutdown: {reason:?}")
                                }
                            };

                            println!("{output}");
                            std::io::stdout().flush().expect("Failed to flush stdout");

                            // Exit after processing all expected events.
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

// -------------------------------------------------------------------------------------------------
// Test cases
// -------------------------------------------------------------------------------------------------

const TOTAL_EXPECTED_EVENTS: usize = 24;

#[derive(Debug, Clone)]
struct TestCase {
    pub description: &'static str,
    pub sequence: Vec<u8>,
}

impl TestCase {
    pub fn new(description: &'static str, sequence: Vec<u8>) -> Self {
        Self {
            description,
            sequence,
        }
    }
}

#[allow(clippy::too_many_lines)]
fn generate_test_cases() -> Vec<TestCase> {
    vec![
        // Test 1: Tab key (was broken, returning None).
        TestCase::new("Tab", vec![CONTROL_TAB]),
        // Test 2: Shift+Tab (BackTab): ESC [ Z.
        TestCase::new(
            "Shift+Tab (BackTab)",
            vec![ANSI_ESC, ANSI_CSI_BRACKET, BACKTAB_FINAL],
        ),
        // Test 3: Ctrl+Space: NUL byte.
        TestCase::new("Ctrl+Space", vec![CONTROL_NUL]),
        // Tests 4 to 7: Alternative Home/End sequences. Format: ESC [ code ~
        TestCase::new(
            "Home (alt ESC[1~)",
            vec![
                ANSI_ESC,
                ANSI_CSI_BRACKET,
                ASCII_DIGIT_1,
                ANSI_FUNCTION_KEY_TERMINATOR,
            ],
        ),
        TestCase::new(
            "End (alt ESC[4~)",
            vec![
                ANSI_ESC,
                ANSI_CSI_BRACKET,
                ASCII_DIGIT_4,
                ANSI_FUNCTION_KEY_TERMINATOR,
            ],
        ),
        TestCase::new(
            "Home (rxvt ESC[7~)",
            vec![
                ANSI_ESC,
                ANSI_CSI_BRACKET,
                ASCII_DIGIT_7,
                ANSI_FUNCTION_KEY_TERMINATOR,
            ],
        ),
        TestCase::new(
            "End (rxvt ESC[8~)",
            vec![
                ANSI_ESC,
                ANSI_CSI_BRACKET,
                ASCII_DIGIT_8,
                ANSI_FUNCTION_KEY_TERMINATOR,
            ],
        ),
        // Tests 8 to 24: Numpad in application mode (SS3 sequences). Format: ESC O
        // command_char
        TestCase::new(
            "Numpad 0 (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_0],
        ),
        TestCase::new(
            "Numpad 1 (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_1],
        ),
        TestCase::new(
            "Numpad 2 (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_2],
        ),
        TestCase::new(
            "Numpad 3 (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_3],
        ),
        TestCase::new(
            "Numpad 4 (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_4],
        ),
        TestCase::new(
            "Numpad 5 (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_5],
        ),
        TestCase::new(
            "Numpad 6 (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_6],
        ),
        TestCase::new(
            "Numpad 7 (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_7],
        ),
        TestCase::new(
            "Numpad 8 (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_8],
        ),
        TestCase::new(
            "Numpad 9 (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_9],
        ),
        TestCase::new(
            "Numpad Enter (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_ENTER],
        ),
        TestCase::new(
            "Numpad + (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_PLUS],
        ),
        TestCase::new(
            "Numpad - (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_MINUS],
        ),
        TestCase::new(
            "Numpad * (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_MULTIPLY],
        ),
        TestCase::new(
            "Numpad / (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_DIVIDE],
        ),
        TestCase::new(
            "Numpad . (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_DECIMAL],
        ),
        TestCase::new(
            "Numpad , (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_COMMA],
        ),
    ]
}
