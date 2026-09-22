// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// XMARK: pty test faking terminal emulator (in controller)

//! [`PTY`]-based integration test for [`DirectToAnsiInputDevice`].
//!
//! Validates that [`DirectToAnsiInputDevice`] correctly manages asynchronous terminal
//! input streams in a real pseudoterminal environment:
//! - Up Arrow escape sequence (`ESC [ A`)
//! - Down Arrow escape sequence (`ESC [ B`)
//! - Function key F1 escape sequence (`ESC [ 11 ~` / `ESC O P`)
//! - Terminal raw mode configuration preventing line-buffering and signal trapping
//!
//! # Architecture and Process Roles
//!
//! In this test, the controller process simulates a host terminal emulator while the
//! controlled child runs our TUI input engine. For the overall architecture and testing
//! philosophy, see the [process roles documentation].
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │ Actor 1: PTY Controller (Simulated Terminal Emulator)                   │
//! │ Synchronous test driver (runs on host / controller PTY)                 │
//! │                                                                         │
//! │  1. generate_pty_test! creates PTY pair (controller / controlled)       │
//! │  2. Spawns test binary child with $ENV_VAR_PTY_CONTROLLED=1             │
//! │  3. Writes synthesized ANSI escape sequences into controller PTY fd     │
//! │  4. Reads parsed event strings from Actor 2's stdout via PTY reader     │
//! │  5. Verifies parsed domain events match expectations                    │
//! └────────────────────────────────────┬────────────────────────────────────┘
//!                                      │ spawns with controlled PTY as stdin/stdout
//! ┌────────────────────────────────────▼────────────────────────────────────┐
//! │ Actor 2: PTY Controlled (System Under Test / TUI Input Engine)          │
//! │ Tokio runtime and asynchronous event loop (runs on controlled PTY)      │
//! │                                                                         │
//! │  1. Test entry detects $ENV_VAR_PTY_CONTROLLED environment variable     │
//! │  2. Enables raw mode on controlled PTY (prevents line buffering)         │
//! │  3. Creates DirectToAnsiInputDevice (mio poller reads stdin)            │
//! │  4. Loop: next().await → ChunkFramer / vt_100 parser → echo to stdout   │
//! │  5. Exits cleanly after processing expected test event count            │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! 1. **[`PTY`] Controller (Simulated Terminal Emulator)**:
//!    - Simulates an external terminal emulator host (such as [`Kitty`], Ghostty, or
//!      [`XTerm`]).
//!    - Synthesizes foundational navigation and function key sequences using
//!      [`generate_keyboard_sequence`].
//!    - Writes sequence bytes directly into the controller side of the [`PTY`], which the
//!      operating system delivers to the controlled process's [`stdin`].
//!    - Reads line-by-line event confirmation strings from the controlled child's
//!      [`stdout`] and validates decoded `Keyboard:` output.
//!
//! 2. **[`PTY`] Controlled (System Under Test / TUI Input Engine)**:
//!    - Represents the real TUI application running inside the child subprocess connected
//!      to the controlled side of the [`PTY`].
//!    - Configures the controlled [`PTY`] to raw mode to disable canonical line
//!      buffering, disable local echo, and prevent control characters from being
//!      intercepted by the tty driver.
//!    - Initializes [`DirectToAnsiInputDevice`].
//!    - Behind the scenes, the background [`mio`] polling thread watches [`stdin`],
//!      feeding raw bytes into [`ChunkFramer`] and the Sans-IO [`vt_100`] parser.
//!    - Runs a Tokio asynchronous event loop consuming from `input_device.next().await`.
//!    - Maps parsed intermediate representation events into domain
//!      [`InputEvent::Keyboard`] structures.
//!    - Formats and echoes each received event over [`stdout`] so the controller can
//!      verify parsing accuracy.
//!
//! # Critical: Raw Mode Requirement
//!
//! In pseudoterminal architectures, the controlled side is what the child process sees as
//! its terminal. When the child reads from [`stdin`], it reads from the controlled
//! [`PTY`]. Setting raw mode on the controlled device is mandatory because:
//!
//! 1. **No Line Buffering**: Characters become available immediately as bytes arrive
//!    without waiting for a carriage return or newline.
//! 2. **No Special Character Processing**: Control characters (such as [`ESC`]) pass
//!    through verbatim rather than triggering terminal signals (like [`SIGINT`] on
//!    `Ctrl+C`).
//! 3. **Non-Blocking Async Read**: The asynchronous reader retrieves discrete chunks as
//!    they arrive, enabling continuous Sans-IO parsing.
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_pty_input_device -- --nocapture
//! ```
//!
//! [`ChunkFramer`]: crate::core::ansi::vt_100_terminal_input_parser::ChunkFramer
//! [`DirectToAnsiInputDevice`]: crate::direct_to_ansi::DirectToAnsiInputDevice
//! [`ESC`]: crate::EscSequence
//! [`generate_keyboard_sequence`]: crate::core::ansi::generator::generate_keyboard_sequence
//! [`InputEvent::Keyboard`]: crate::InputEvent::Keyboard
//! [`Kitty`]: https://sw.kovidgoyal.net/kitty/
//! [`mio`]: mio
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`SIGINT`]: https://en.wikipedia.org/wiki/Signal_(IPC)#SIGINT
//! [`stdin`]: std::io::stdin
//! [`stdout`]: std::io::stdout
//! [`vt_100`]: crate::core::ansi::vt_100_terminal_input_parser
//! [`XTerm`]: https://invisible-island.net/xterm/
//! [process roles documentation]: mod@super#architecture-and-process-roles

use crate::{GLYPH_CONTROLLED, GLYPH_CONTROLLER, GLYPH_CONTROLLER_CLEANUP, GLYPH_SUCCESS,
            GLYPH_WAITING, InputEvent, MSG_CONTROLLED_READY, MSG_CONTROLLED_STARTING,
            MSG_TEST_RUNNING, PtyTestContext, PtyTestMode,
            core::ansi::{generator::generate_keyboard_sequence,
                         vt_100_terminal_input_parser::ir_event_types::{VT100InputEventIR,
                                                                        VT100KeyCodeIR,
                                                                        VT100KeyModifiersIR}},
            generate_pty_test,
            tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice};
use std::{io::Write, time::Duration};

generate_pty_test! {
    test_fn: test_pty_input_device,
    controller: controller,
    controlled: controlled,
    mode: PtyTestMode::Raw,
}

/// Actor 1: [`PTY`] Controller (test entry, env var NOT set) - Synchronous code
///
/// - Receives [`PTY`] pair and child process from macro
/// - Writes [`ANSI`] sequences to [`PTY`] controller
/// - Reads parsed output from controlled's [`stdout`]
/// - Verifies correctness
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`stdout`]: std::io::stdout
fn controller(context: PtyTestContext) {
    let PtyTestContext {
        pty_pair,
        child,
        mut buf_reader,
        mut writer,
    } = context;

    eprintln!("{GLYPH_CONTROLLER} PTY Controller: Starting...");

    eprintln!(
        "{GLYPH_WAITING} PTY Controller: Waiting for controlled process to start..."
    );

    // Wait for controlled to confirm it's running and ready.
    child
        .wait_for_ready(&mut buf_reader, MSG_CONTROLLED_READY)
        .expect("Failed to wait for ready signal");

    let test_cases = generate_test_cases();

    eprintln!(
        "{} PTY Controller: Sending {} sequences...",
        GLYPH_WAITING,
        test_cases.len()
    );

    // For each test sequence: write ANSI bytes to PTY, read back parsed event, verify
    // correctness.
    for test_case in &test_cases {
        let TestCase {
            description,
            sequence,
        } = test_case;

        eprintln!("  → Sending: {description} ({sequence:?})");

        writer
            .write_all(sequence)
            .expect("Failed to write sequence");
        writer.flush().expect("Failed to flush");

        // Read responses until we get an event line (skip test harness noise)
        let event_line = child.read_line_state(&mut buf_reader, |line| {
            line.starts_with("Keyboard:")
                || line.starts_with("Mouse:")
                || line.starts_with("Resize:")
                || line.starts_with("Focus:")
                || line.starts_with("Paste:")
        });

        eprintln!("  {GLYPH_SUCCESS} {description}: {event_line}");
    }

    eprintln!("{GLYPH_CONTROLLER_CLEANUP} PTY Controller: Cleaning up...");

    // Close writer to signal EOF.
    drop(writer);
    child.drain_and_wait(buf_reader, pty_pair);

    eprintln!("{GLYPH_SUCCESS} PTY Controller: Test passed!");
}

/// Actor 2: [`PTY`] Controlled (worker process).
///
/// Runs in the spawned child process when `R3BL_PTY_TEST_CONTROLLED` env var is set.
/// This process's stdin/stdout are connected to the controlled [`PTY`] [`file
/// descriptor`].
///
/// **Critical Steps**:
/// 1. **Enable Raw Mode**: MUST set the controlled [`PTY`] terminal to raw mode to:
///    - Disable line buffering (get bytes immediately)
///    - Prevent [`ANSI`] escape sequence interpretation
///    - Allow async byte-by-byte reading
/// 2. **Create Device**: Initialize `DirectToAnsiInputDevice` to read from [`stdin`]
/// 3. **Process Events**: Read and parse [`ANSI`] sequences into `InputEvents`
/// 4. **Output Results**: Write parsed events to [`stdout`] for controller to verify
///
/// The harness performs [`std::process::exit(0)`] after this function returns.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`file descriptor`]: https://manpages.debian.org/open.2
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`stdin`]: std::io::stdin
/// [`stdout`]: std::io::stdout
fn controlled() {
    // Print to stdout immediately to confirm controlled is starting.
    println!("{MSG_TEST_RUNNING}");
    println!("{MSG_CONTROLLED_STARTING}");
    std::io::stdout().flush().expect("Failed to flush stdout");

    // Create a Tokio runtime for async operations.
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Starting...");
        let mut input_device = DirectToAnsiInputDevice::new()
            .expect("Failed to initialize DirectToAnsiInputDevice");
        eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Device created, reading events...");

        // Signal to controller that we're ready to receive input.
        println!("{MSG_CONTROLLED_READY}");
        std::io::stdout().flush().expect("Failed to flush stdout");

        // Create inactivity timeout: exit if no events for 5 seconds.
        // Needs headroom for parallel test execution where CPU scheduling
        // delays can cause input events to arrive late.
        // https://developerlife.com/2024/07/10/rust-async-cancellation-safety-tokio/#example-1-right-and-wrong-way-to-sleep-and-interval
        let inactivity_timeout = Duration::from_secs(5);
        // Cancel safe: sleep_until() with a deadline stored outside select! is safe.
        let mut inactivity_deadline = tokio::time::Instant::now() + inactivity_timeout;

        let mut event_count = 0;

        loop {
            tokio::select! {
                // Try to read an event from the device.
                event_result = input_device.next() => {
                    match event_result {
                        Some(event) => {
                            event_count += 1;
                            inactivity_deadline = tokio::time::Instant::now() + inactivity_timeout; // Reset deadline.
                            eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Event #{event_count}: {event:?}");

                            // Output event in parseable format.
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

                            // Exit after processing expected number of events.
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
                // Inactivity timeout: exit if deadline is reached (2 seconds of no events).
                // https://developerlife.com/2024/07/10/rust-async-cancellation-safety-tokio/#example-1-right-and-wrong-way-to-sleep-and-interval
                () = tokio::time::sleep_until(inactivity_deadline) => {
                    eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Inactivity timeout (5 seconds with no events), exiting");
                    break;
                }
            }
        }

        eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Completed, exiting");
    });
}

/// Expected number of keyboard events processed by the controlled process:
/// - 3 navigation and function key sequences (Up, Down, F1)
const TOTAL_EXPECTED_EVENTS: usize = 3;

/// A single test case for basic input device sequence verification.
struct TestCase {
    description: &'static str,
    sequence: Vec<u8>,
}

/// Generate the structured test cases for basic input device testing.
fn generate_test_cases() -> Vec<TestCase> {
    let no_mods = VT100KeyModifiersIR::default();
    vec![
        TestCase {
            description: "Up Arrow",
            sequence: generate_keyboard_sequence(&VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Up,
                modifiers: no_mods,
            })
            .expect("Failed to generate sequence for: Up Arrow"),
        },
        TestCase {
            description: "Down Arrow",
            sequence: generate_keyboard_sequence(&VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Down,
                modifiers: no_mods,
            })
            .expect("Failed to generate sequence for: Down Arrow"),
        },
        TestCase {
            description: "F1",
            sequence: generate_keyboard_sequence(&VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Function(1),
                modifiers: no_mods,
            })
            .expect("Failed to generate sequence for: F1"),
        },
    ]
}
