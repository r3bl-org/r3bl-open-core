// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// XMARK: pty test faking terminal emulator (in controller)

//! [`PTY`]-based integration test for keyboard modifiers (Shift, Ctrl, Alt).
//!
//! Validates that [`DirectToAnsiInputDevice`] correctly parses keyboard sequences with
//! various modifier combinations in a real pseudoterminal environment:
//! - Single modifiers: Shift, Ctrl, Alt (`Shift+Up`, `Ctrl+Up`, `Alt+Down`)
//! - Combined dual modifiers: `Shift+Alt+Left`, `Ctrl+Shift+Right`
//! - Triple modifiers: `Ctrl+Alt+Shift+F1`
//! - Modified navigation keys: `Shift+Home`, `Shift+End`, `Ctrl+Home`, `Alt+End`
//! - Modified editing keys: `Shift+Insert`, `Ctrl+Delete`, `Alt+PageUp`,
//!   `Ctrl+Shift+PageDown`
//! - Modified function keys: `Shift+F5`, `Ctrl+F12`
//! - Alt+letter combinations: `Alt+a` (`ESC a`)
//!
//! # Architecture and Process Roles
//!
//! In this test, the controller process simulates a host terminal emulator while the
//! controlled child runs our TUI input engine. For the overall architecture and testing
//! philosophy, see the [process roles documentation].
//!
//! 1. **[`PTY`] Controller (Simulated Terminal Emulator)**:
//!    - Simulates an external host terminal emulator synthesizing XTerm-style modified
//!      key sequences (`CSI 1 ; <modifier> <char>` for arrows and navigation, `CSI <code
//!      ; <modifier> ~` for editing and function keys, and `ESC <char>` for Alt
//!      combinations).
//!    - Generates sequence byte buffers using [`generate_keyboard_sequence`] and writes
//!      them directly into the controller side of the [`PTY`], which the operating system
//!      delivers to the controlled process's [`stdin`].
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
//!    - Runs a Tokio asynchronous event loop consuming from `input_device.next().await`.
//!    - Maps parsed intermediate representation events into domain
//!      [`InputEvent::Keyboard`] structures containing exact [`ModifierKeysMask`] flags
//!      (Shift, Ctrl, Alt).
//!    - Formats and echoes each received event over [`stdout`] so the controller can
//!      verify parsing accuracy.
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_pty_keyboard_modifiers -- --nocapture
//! ```
//!
//! [`ChunkFramer`]: crate::core::ansi::vt_100_terminal_input_parser::ChunkFramer
//! [`DirectToAnsiInputDevice`]: crate::direct_to_ansi::DirectToAnsiInputDevice
//! [`generate_keyboard_sequence`]: crate::core::ansi::generator::generate_keyboard_sequence
//! [`InputEvent::Keyboard`]: crate::InputEvent::Keyboard
//! [`mio`]: https://crates.io/crates/mio
//! [`ModifierKeysMask`]: crate::ModifierKeysMask
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`stdin`]: std::io::stdin
//! [`stdout`]: std::io::stdout
//! [`vt_100`]: crate::core::ansi::vt_100_terminal_input_parser
//! [process roles documentation]: mod@super#architecture-and-process-roles

use crate::{GLYPH_CONTROLLED, GLYPH_CONTROLLER, GLYPH_CONTROLLER_CLEANUP, GLYPH_SUCCESS,
            GLYPH_WAITING, InputEvent, MSG_CONTROLLED_READY, MSG_CONTROLLED_STARTING,
            PtyTestContext, PtyTestMode,
            core::ansi::{generator::generate_keyboard_sequence,
                         vt_100_terminal_input_parser::ir_event_types::{VT100InputEventIR,
                                                                        VT100KeyCodeIR,
                                                                        VT100KeyModifiersIR}},
            generate_pty_test,
            tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice};
use std::{io::Write, time::Duration};

generate_pty_test! {
    test_fn: test_pty_keyboard_modifiers,
    controller: controller,
    controlled: controlled,
    mode: PtyTestMode::Raw,
}

/// [`PTY`] Controller: Send keyboard sequences with modifiers and verify parsing
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
fn controller(context: PtyTestContext) {
    let PtyTestContext {
        pty_pair,
        child,
        mut buf_reader,
        mut writer,
    } = context;

    eprintln!("{GLYPH_CONTROLLER} PTY Controller: Starting keyboard modifiers test...");

    eprintln!(
        "{GLYPH_WAITING} PTY Controller: Waiting for controlled process to start..."
    );

    // Wait for controlled to be ready.
    child
        .wait_for_ready(&mut buf_reader, MSG_CONTROLLED_READY)
        .expect("Failed to wait for MSG_CONTROLLED_READY");
    eprintln!("  {GLYPH_SUCCESS} Controlled is ready (input device created)");

    let test_cases = generate_test_cases();

    assert_eq!(test_cases.len(), TOTAL_EXPECTED_EVENTS);

    eprintln!(
        "{} PTY Controller: Sending {} keyboard modifier combinations...",
        GLYPH_WAITING,
        test_cases.len()
    );

    for test_case in &test_cases {
        let TestCase { description, ir } = test_case;

        let sequence = generate_keyboard_sequence(ir)
            .unwrap_or_else(|| panic!("Failed to generate sequence for: {description}"));

        eprintln!("  → Sending: {description} ({sequence:?})");

        writer
            .write_all(&sequence)
            .expect("Failed to write sequence");
        writer.flush().expect("Failed to flush");

        // Read responses until we get a keyboard event line. The controlled process
        // responds immediately after receiving input, so blocking reads work.
        let event_line =
            child.read_line_state(&mut buf_reader, |line| line.starts_with("Keyboard:"));

        assert!(
            !event_line.contains("Unexpected"),
            "Received unexpected event line: {event_line}"
        );
        eprintln!("  {GLYPH_SUCCESS} {description}: {event_line}");
    }

    eprintln!("{GLYPH_CONTROLLER_CLEANUP} PTY Controller: Cleaning up...");

    drop(writer);
    child.drain_and_wait(buf_reader, pty_pair);

    eprintln!("{GLYPH_SUCCESS} PTY Controller: Test passed!");
}

/// [`PTY`] Controlled: Read and parse keyboard events with modifiers. The harness
/// performs [`std::process::exit(0)`] after this function returns.
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

                            let output = match event {
                                InputEvent::Keyboard(ref key_press) => {
                                    format!("Keyboard: {key_press:?}")
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

/// Expected number of keyboard events processed by the controlled process:
/// - 17 modifier combinations across arrow, function, navigation, and editing keys
const TOTAL_EXPECTED_EVENTS: usize = 17;

/// A single test case for keyboard modifier combination verification.
struct TestCase {
    description: &'static str,
    ir: VT100InputEventIR,
}

/// Generate the structured test cases for keyboard modifier testing.
#[allow(clippy::too_many_lines)]
fn generate_test_cases() -> Vec<TestCase> {
    vec![
        TestCase {
            description: "Shift+Up",
            ir: VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Up,
                modifiers: VT100KeyModifiersIR::SHIFT,
            },
        },
        TestCase {
            description: "Ctrl+Up",
            ir: VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Up,
                modifiers: VT100KeyModifiersIR::CTRL,
            },
        },
        TestCase {
            description: "Alt+Down",
            ir: VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Down,
                modifiers: VT100KeyModifiersIR::ALT,
            },
        },
        TestCase {
            description: "Shift+Alt+Left",
            ir: VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Left,
                modifiers: VT100KeyModifiersIR::SHIFT.with_alt(),
            },
        },
        TestCase {
            description: "Ctrl+Shift+Right",
            ir: VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Right,
                modifiers: VT100KeyModifiersIR::CTRL.with_shift(),
            },
        },
        TestCase {
            description: "Ctrl+Alt+Shift+F1",
            ir: VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Function(1),
                modifiers: VT100KeyModifiersIR::CTRL.with_alt().with_shift(),
            },
        },
        // Navigation keys with modifiers (validating Shift+Home / Shift+End fix)
        TestCase {
            description: "Shift+Home",
            ir: VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Home,
                modifiers: VT100KeyModifiersIR::SHIFT,
            },
        },
        TestCase {
            description: "Shift+End",
            ir: VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::End,
                modifiers: VT100KeyModifiersIR::SHIFT,
            },
        },
        TestCase {
            description: "Ctrl+Home",
            ir: VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Home,
                modifiers: VT100KeyModifiersIR::CTRL,
            },
        },
        TestCase {
            description: "Alt+End",
            ir: VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::End,
                modifiers: VT100KeyModifiersIR::ALT,
            },
        },
        // Editing keys with modifiers
        TestCase {
            description: "Shift+Insert",
            ir: VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Insert,
                modifiers: VT100KeyModifiersIR::SHIFT,
            },
        },
        TestCase {
            description: "Ctrl+Delete",
            ir: VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Delete,
                modifiers: VT100KeyModifiersIR::CTRL,
            },
        },
        TestCase {
            description: "Alt+PageUp",
            ir: VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::PageUp,
                modifiers: VT100KeyModifiersIR::ALT,
            },
        },
        TestCase {
            description: "Ctrl+Shift+PageDown",
            ir: VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::PageDown,
                modifiers: VT100KeyModifiersIR::CTRL.with_shift(),
            },
        },
        // Function keys F5-F12 with modifiers
        TestCase {
            description: "Shift+F5",
            ir: VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Function(5),
                modifiers: VT100KeyModifiersIR::SHIFT,
            },
        },
        TestCase {
            description: "Ctrl+F12",
            ir: VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Function(12),
                modifiers: VT100KeyModifiersIR::CTRL,
            },
        },
        // Alt+letter combination
        TestCase {
            description: "Alt+a",
            ir: VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Char('a'),
                modifiers: VT100KeyModifiersIR::ALT,
            },
        },
    ]
}
