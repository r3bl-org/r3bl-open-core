// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// XMARK: pty test faking terminal emulator (in controller)

//! [`PTY`]-based integration test for [`SGR`] mouse tracking sequences.
//!
//! Validates that [`DirectToAnsiInputDevice`] correctly parses [`SGR`] mouse tracking
//! sequences in a real pseudoterminal environment:
//! - Button clicks and releases (Left, Right, and Middle mouse buttons)
//! - Mouse drag operations (Left Drag, Right Drag, Middle Drag)
//! - Motion tracking (`MouseMove` hover tracking without buttons depressed)
//! - Modifier combinations with mouse clicks (Shift, Ctrl, Alt, and Ctrl+Shift)
//! - Scroll wheel events (Scroll Up, Scroll Down)
//! - [`SGR`] 1006 coordinate encoding (`ESC [ < B ; C ; R M` / `m`) with 1-based
//!   coordinates
//!
//! # Architecture and Process Roles
//!
//! In this test, the controller process simulates a host terminal emulator while the
//! controlled child runs our TUI input engine. For the overall architecture and testing
//! philosophy, see the [process roles documentation].
//!
//! 1. **[`PTY`] Controller (Simulated Terminal Emulator)**:
//!    - Simulates an external terminal emulator host (such as [`Kitty`], Ghostty, or
//!      [`XTerm`]) with [`SGR`] 1006 extended mouse mode enabled.
//!    - Synthesizes raw [`SGR`] mouse sequences (`ESC [ < B ; C ; R M` for
//!      press/drag/scroll, `ESC [ < B ; C ; R m` for release) directly into the
//!      controller side of the [`PTY`], which the operating system delivers to the
//!      controlled process's [`stdin`].
//!    - Exercises diverse mouse interactions including button presses, releases,
//!      coordinate drags, pure motion hover events, modifier key masks, and vertical
//!      scroll wheel events.
//!    - Reads line-by-line event confirmation strings from the controlled child's
//!      [`stdout`] and validates decoded [`MouseInputKind`] and [`ModifierKeysMask`]
//!      output.
//!
//! 2. **[`PTY`] Controlled (System Under Test / TUI Input Engine)**:
//!    - Represents the real TUI application running inside the child subprocess connected
//!      to the controlled side of the [`PTY`].
//!    - Initializes [`DirectToAnsiInputDevice`].
//!    - Behind the scenes, the background [`mio`] polling thread watches [`stdin`],
//!      feeding raw bytes into [`ChunkFramer`] and the Sans-IO [`vt_100`] parser.
//!    - Runs a Tokio asynchronous event loop consuming from `input_device.next().await`.
//!    - Maps parsed intermediate representation mouse events into domain
//!      [`InputEvent::Mouse`] structures containing [`MouseInput`].
//!    - Formats and echoes each received mouse event over [`stdout`] so the controller
//!      can verify parsing accuracy.
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_pty_mouse_events -- --nocapture
//! ```
//!
//! [`ChunkFramer`]: crate::core::ansi::vt_100_terminal_input_parser::ChunkFramer
//! [`DirectToAnsiInputDevice`]: crate::direct_to_ansi::DirectToAnsiInputDevice
//! [`InputEvent::Mouse`]: crate::InputEvent::Mouse
//! [`Kitty`]: https://sw.kovidgoyal.net/kitty/
//! [`mio`]: https://crates.io/crates/mio
//! [`ModifierKeysMask`]: crate::ModifierKeysMask
//! [`MouseInput`]: crate::MouseInput
//! [`MouseInputKind`]: crate::MouseInputKind
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`SGR`]: crate::SgrCode
//! [`stdin`]: std::io::stdin
//! [`stdout`]: std::io::stdout
//! [`vt_100`]: crate::core::ansi::vt_100_terminal_input_parser
//! [`XTerm`]: https://invisible-island.net/xterm/
//! [process roles documentation]: mod@super#architecture-and-process-roles

use crate::{GLYPH_CONTROLLED, GLYPH_CONTROLLER, GLYPH_CONTROLLER_CLEANUP, GLYPH_SUCCESS,
            GLYPH_WAITING, InputEvent, MSG_CONTROLLED_READY, MSG_CONTROLLED_STARTING,
            MSG_TEST_RUNNING, PtyTestContext, PtyTestMode, TermPos,
            core::ansi::{generator::generate_keyboard_sequence,
                         vt_100_terminal_input_parser::ir_event_types::{VT100InputEventIR,
                                                                        VT100KeyModifiersIR,
                                                                        VT100MouseActionIR,
                                                                        VT100MouseButtonIR,
                                                                        VT100ScrollDirectionIR}},
            generate_pty_test,
            tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice};
use std::{io::Write, time::Duration};

generate_pty_test! {
    test_fn: test_pty_mouse_events,
    controller: controller,
    controlled: controlled,
    mode: PtyTestMode::Raw,
}

/// [`PTY`] Controller: Send mouse event sequences and verify parsing
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

    eprintln!("{GLYPH_CONTROLLER} PTY Controller: Starting mouse events test...");
    eprintln!(
        "{GLYPH_WAITING} PTY Controller: Waiting for controlled process to start..."
    );

    // Wait for controlled to confirm it's running and ready.
    child
        .wait_for_ready(&mut buf_reader, MSG_CONTROLLED_READY)
        .expect("Failed to wait for ready signal");

    let test_cases = generate_test_cases();

    eprintln!(
        "{GLYPH_WAITING} PTY Controller: Sending {} mouse event sequences...",
        test_cases.len()
    );

    for test_case in &test_cases {
        let desc = test_case.description;
        let event = &test_case.ir;
        let expected_pattern = test_case.expected_pattern;
        let sequence = generate_keyboard_sequence(event)
            .unwrap_or_else(|| panic!("Failed to generate sequence for: {desc}"));
        eprintln!("  → Sending: {desc} ({sequence:?})");

        writer
            .write_all(&sequence)
            .expect("Failed to write sequence");
        writer.flush().expect("Failed to flush");
        // Read responses until we get a mouse event line. The controlled process
        // responds immediately after receiving input, so blocking reads work.
        let event_line =
            child.read_line_state(&mut buf_reader, |line| line.starts_with("Mouse:"));
        assert!(
            event_line.contains(expected_pattern),
            "Assertion failed for {desc}: expected output containing '{expected_pattern}', got: '{event_line}'"
        );
        eprintln!("  {GLYPH_SUCCESS} {desc}: {event_line}");
    }

    eprintln!("{GLYPH_CONTROLLER_CLEANUP} PTY Controller: Cleaning up...");

    drop(writer);
    child.drain_and_wait(buf_reader, pty_pair);

    eprintln!("{GLYPH_SUCCESS} PTY Controller: Test passed!");
}

/// [`PTY`] Controlled: Read and parse mouse events. The harness performs
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
                                InputEvent::Mouse(ref mouse_input) => {
                                    format!("Mouse: {mouse_input:?}")
                                }
                                _ => {
                                    format!("Event: {event:?}")
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

// -------------------------------------------------------------------------------------------------
// Test cases
// -------------------------------------------------------------------------------------------------

const TOTAL_EXPECTED_EVENTS: usize = 18;

#[derive(Debug, Clone)]
struct TestCase {
    pub description: &'static str,
    pub ir: VT100InputEventIR,
    pub expected_pattern: &'static str,
}

impl TestCase {
    pub fn new(
        description: &'static str,
        ir: VT100InputEventIR,
        expected_pattern: &'static str,
    ) -> Self {
        Self {
            description,
            ir,
            expected_pattern,
        }
    }
}

#[allow(clippy::too_many_lines)]
fn generate_test_cases() -> Vec<TestCase> {
    vec![
        TestCase::new(
            "Left Click",
            VT100InputEventIR::Mouse {
                button: VT100MouseButtonIR::Left,
                pos: TermPos::from_one_based(10, 5),
                action: VT100MouseActionIR::Press,
                modifiers: VT100KeyModifiersIR::default(),
            },
            "kind: MouseDown(Left), maybe_modifier_keys: None",
        ),
        TestCase::new(
            "Left Release",
            VT100InputEventIR::Mouse {
                button: VT100MouseButtonIR::Left,
                pos: TermPos::from_one_based(10, 5),
                action: VT100MouseActionIR::Release,
                modifiers: VT100KeyModifiersIR::default(),
            },
            "kind: MouseUp(Left), maybe_modifier_keys: None",
        ),
        TestCase::new(
            "Right Click",
            VT100InputEventIR::Mouse {
                button: VT100MouseButtonIR::Right,
                pos: TermPos::from_one_based(20, 10),
                action: VT100MouseActionIR::Press,
                modifiers: VT100KeyModifiersIR::default(),
            },
            "kind: MouseDown(Right), maybe_modifier_keys: None",
        ),
        TestCase::new(
            "Right Release",
            VT100InputEventIR::Mouse {
                button: VT100MouseButtonIR::Right,
                pos: TermPos::from_one_based(20, 10),
                action: VT100MouseActionIR::Release,
                modifiers: VT100KeyModifiersIR::default(),
            },
            "kind: MouseUp(Right), maybe_modifier_keys: None",
        ),
        TestCase::new(
            "Middle Click",
            VT100InputEventIR::Mouse {
                button: VT100MouseButtonIR::Middle,
                pos: TermPos::from_one_based(30, 15),
                action: VT100MouseActionIR::Press,
                modifiers: VT100KeyModifiersIR::default(),
            },
            "kind: MouseDown(Middle), maybe_modifier_keys: None",
        ),
        TestCase::new(
            "Middle Release",
            VT100InputEventIR::Mouse {
                button: VT100MouseButtonIR::Middle,
                pos: TermPos::from_one_based(30, 15),
                action: VT100MouseActionIR::Release,
                modifiers: VT100KeyModifiersIR::default(),
            },
            "kind: MouseUp(Middle), maybe_modifier_keys: None",
        ),
        TestCase::new(
            "Left Drag",
            VT100InputEventIR::Mouse {
                button: VT100MouseButtonIR::Left,
                pos: TermPos::from_one_based(10, 5),
                action: VT100MouseActionIR::Drag,
                modifiers: VT100KeyModifiersIR::default(),
            },
            "kind: MouseDrag(Left), maybe_modifier_keys: None",
        ),
        TestCase::new(
            "Right Drag",
            VT100InputEventIR::Mouse {
                button: VT100MouseButtonIR::Right,
                pos: TermPos::from_one_based(20, 10),
                action: VT100MouseActionIR::Drag,
                modifiers: VT100KeyModifiersIR::default(),
            },
            "kind: MouseDrag(Right), maybe_modifier_keys: None",
        ),
        TestCase::new(
            "Middle Drag",
            VT100InputEventIR::Mouse {
                button: VT100MouseButtonIR::Middle,
                pos: TermPos::from_one_based(30, 15),
                action: VT100MouseActionIR::Drag,
                modifiers: VT100KeyModifiersIR::default(),
            },
            "kind: MouseDrag(Middle), maybe_modifier_keys: None",
        ),
        TestCase::new(
            "Mouse Motion",
            VT100InputEventIR::Mouse {
                button: VT100MouseButtonIR::Unknown,
                pos: TermPos::from_one_based(15, 8),
                action: VT100MouseActionIR::Motion,
                modifiers: VT100KeyModifiersIR::default(),
            },
            "kind: MouseMove, maybe_modifier_keys: None",
        ),
        TestCase::new(
            "Shift+Left Click",
            VT100InputEventIR::Mouse {
                button: VT100MouseButtonIR::Left,
                pos: TermPos::from_one_based(10, 5),
                action: VT100MouseActionIR::Press,
                modifiers: VT100KeyModifiersIR::SHIFT,
            },
            "kind: MouseDown(Left), maybe_modifier_keys: Some(ModifierKeysMask { shift_key_state: Pressed, ctrl_key_state: NotPressed, alt_key_state: NotPressed })",
        ),
        TestCase::new(
            "Ctrl+Left Click",
            VT100InputEventIR::Mouse {
                button: VT100MouseButtonIR::Left,
                pos: TermPos::from_one_based(10, 5),
                action: VT100MouseActionIR::Press,
                modifiers: VT100KeyModifiersIR::CTRL,
            },
            "kind: MouseDown(Left), maybe_modifier_keys: Some(ModifierKeysMask { shift_key_state: NotPressed, ctrl_key_state: Pressed, alt_key_state: NotPressed })",
        ),
        TestCase::new(
            "Alt+Right Click",
            VT100InputEventIR::Mouse {
                button: VT100MouseButtonIR::Right,
                pos: TermPos::from_one_based(20, 10),
                action: VT100MouseActionIR::Press,
                modifiers: VT100KeyModifiersIR::ALT,
            },
            "kind: MouseDown(Right), maybe_modifier_keys: Some(ModifierKeysMask { shift_key_state: NotPressed, ctrl_key_state: NotPressed, alt_key_state: Pressed })",
        ),
        TestCase::new(
            "Ctrl+Shift+Middle Click",
            VT100InputEventIR::Mouse {
                button: VT100MouseButtonIR::Middle,
                pos: TermPos::from_one_based(30, 15),
                action: VT100MouseActionIR::Press,
                modifiers: VT100KeyModifiersIR::CTRL.with_shift(),
            },
            "kind: MouseDown(Middle), maybe_modifier_keys: Some(ModifierKeysMask { shift_key_state: Pressed, ctrl_key_state: Pressed, alt_key_state: NotPressed })",
        ),
        TestCase::new(
            "Scroll Up",
            VT100InputEventIR::Mouse {
                button: VT100MouseButtonIR::Left,
                pos: TermPos::from_one_based(25, 12),
                action: VT100MouseActionIR::Scroll(VT100ScrollDirectionIR::Up),
                modifiers: VT100KeyModifiersIR::default(),
            },
            "kind: ScrollUp, maybe_modifier_keys: None",
        ),
        TestCase::new(
            "Scroll Down",
            VT100InputEventIR::Mouse {
                button: VT100MouseButtonIR::Left,
                pos: TermPos::from_one_based(25, 12),
                action: VT100MouseActionIR::Scroll(VT100ScrollDirectionIR::Down),
                modifiers: VT100KeyModifiersIR::default(),
            },
            "kind: ScrollDown, maybe_modifier_keys: None",
        ),
        TestCase::new(
            "Scroll Left",
            VT100InputEventIR::Mouse {
                button: VT100MouseButtonIR::Left,
                pos: TermPos::from_one_based(25, 12),
                action: VT100MouseActionIR::Scroll(VT100ScrollDirectionIR::Left),
                modifiers: VT100KeyModifiersIR::default(),
            },
            "kind: ScrollLeft, maybe_modifier_keys: None",
        ),
        TestCase::new(
            "Scroll Right",
            VT100InputEventIR::Mouse {
                button: VT100MouseButtonIR::Left,
                pos: TermPos::from_one_based(25, 12),
                action: VT100MouseActionIR::Scroll(VT100ScrollDirectionIR::Right),
                modifiers: VT100KeyModifiersIR::default(),
            },
            "kind: ScrollRight, maybe_modifier_keys: None",
        ),
    ]
}
