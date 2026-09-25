// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// XMARK: pty test faking terminal emulator (in controller)

//! [`PTY`]-based integration test for [`Kitty`] Keyboard Protocol (`CSI u`) sequences.
//!
//! Validates that [`DirectToAnsiInputDevice`] correctly parses `CSI u` sequences in a
//! real pseudoterminal environment:
//! - Disambiguated key combinations (`Alt+[`, `Shift+Enter`, `Ctrl+Tab`, `Alt+Escape`,
//!   `Shift+Backspace`, `Ctrl+Backspace`)
//! - [`Kitty`] Private Use Area ([`PUA`]) functional keys (`Shift+Home`, `Ctrl+Delete`,
//!   `Shift+Insert`, `PageUp`, `PageDown`, arrow keys)
//! - Function keys (`F1`, `Shift+F12`)
//! - Omitted and alternate key modifiers (`ESC [ 91 u`, `ESC [ 91 : 93 ; 3 u`)
//! - Event types (press `:1`, repeat `:2`, and absorption of release `:3` without
//!   spurious events)
//!
//! # Architecture and Process Roles
//!
//! In this test, the controller process simulates a host terminal emulator while the
//! controlled child runs our TUI input engine. For the overall architecture and testing
//! philosophy, see the [process roles documentation].
//!
//! 1. **[`PTY`] Controller (Simulated Terminal Emulator)**:
//!    - Simulates an advanced host terminal emulator (such as [`Kitty`], Ghostty, or
//!      [`WezTerm`]) with progressive keyboard enhancement enabled.
//!    - Synthesizes and writes raw `CSI u` byte sequences directly into the controller
//!      side of the [`PTY`], which the operating system delivers to the controlled
//!      process's [`stdin`].
//!    - Exercises disambiguated key sequences that traditionally collide in legacy
//!      [`ANSI`] mode (such as `Alt+[` vs [`CSI`] prefix, and `Shift+Enter` vs carriage
//!    - Sends [`PUA`] functional and navigation keys defined in the [[`Kitty`] Keyboard
//!      Protocol] (such as `Shift+Home`, where `Home` is assigned Unicode PUA codepoint
//!      `57366` / `0xE016`, represented by [`KITTY_PUA_HOME`]).
//!    - Injects release events (`:3`) to verify that the controlled child absorbs them
//!      without leaking phantom keypresses, immediately followed by marker keystroke
//!      `'Z'` to confirm stream responsiveness.
//!    - Reads line-by-line responses from the controlled child's [`stdout`] and verifies
//!      exact decoded [`KeyPress`] values.
//!
//! 2. **[`PTY`] Controlled (System Under Test / TUI Input Engine)**:
//!    - Represents the real TUI application running inside the child subprocess connected
//!      to the controlled side of the [`PTY`].
//!    - Initializes [`DirectToAnsiInputDevice`].
//!    - Behind the scenes, the background [`mio`] polling thread watches [`stdin`],
//!      feeding raw bytes into [`ChunkFramer`] and the Sans-IO [`vt_100`] parser.
//!    - Runs a Tokio asynchronous event loop consuming from `input_device.next().await`.
//!    - Maps parsed intermediate representation events into domain
//!      [`InputEvent::Keyboard`] structures with complete [`ModifierKeysMask`] fidelity.
//!    - Formats and echoes each received event over [`stdout`] so the controller can
//!      verify exact parse accuracy.
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_pty_csi_u_keyboard -- --nocapture
//! ```
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`ChunkFramer`]: crate::core::ansi::vt_100_terminal_input_parser::ChunkFramer
//! [`CSI`]: crate::CsiSequence
//! [`DirectToAnsiInputDevice`]: crate::direct_to_ansi::DirectToAnsiInputDevice
//! [`InputEvent::Keyboard`]: crate::InputEvent::Keyboard
//! [`KeyPress`]: crate::KeyPress
//! [`KITTY_PUA_HOME`]: crate::KITTY_PUA_HOME
//! [`Kitty`]: https://sw.kovidgoyal.net/kitty/
//! [`mio`]: https://crates.io/crates/mio
//! [`ModifierKeysMask`]: crate::ModifierKeysMask
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`PUA`]: https://en.wikipedia.org/wiki/Private_Use_Areas
//! [`stdin`]: std::io::stdin
//! [`stdout`]: std::io::stdout
//! [`vt_100`]: crate::core::ansi::vt_100_terminal_input_parser
//! [`WezTerm`]: https://wezfurlong.org/wezterm/
//! [Kitty Keyboard Protocol]: https://sw.kovidgoyal.net/kitty/keyboard-protocol/
//! [process roles documentation]: mod@super#architecture-and-process-roles

use crate::{ANSI_CSI_BRACKET, ANSI_CSI_U_CHAR, ANSI_OSC_CLOSE_BRACKET,
            ANSI_PARAM_SEPARATOR_CHAR, ASCII_DEL, CONTROL_BACKSPACE, CONTROL_ENTER,
            CONTROL_ESC, CONTROL_TAB, CSI_START, DirectToAnsiInputDevice,
            GLYPH_CONTROLLED, GLYPH_CONTROLLER, GLYPH_CONTROLLER_CLEANUP, GLYPH_SUCCESS,
            GLYPH_WAITING, InputEvent, KITTY_EVENT_PRESS, KITTY_EVENT_RELEASE,
            KITTY_EVENT_REPEAT, KITTY_PUA_DELETE, KITTY_PUA_DOWN, KITTY_PUA_END,
            KITTY_PUA_F1, KITTY_PUA_F12, KITTY_PUA_HOME, KITTY_PUA_INSERT,
            KITTY_PUA_LEFT, KITTY_PUA_PAGE_DOWN, KITTY_PUA_PAGE_UP, KITTY_PUA_RIGHT,
            KITTY_PUA_UP, MODIFIER_ALT, MODIFIER_CTRL, MODIFIER_NONE, MODIFIER_SHIFT,
            MSG_CONTROLLED_READY, MSG_CONTROLLED_STARTING, MSG_TEST_RUNNING,
            PtyTestContext, PtyTestMode, csi_u, csi_u_codepoint,
            csi_u_with_alternate_key, csi_u_with_event_type, generate_pty_test};
use std::{io::Write, time::Duration};

generate_pty_test! {
    test_fn: test_pty_csi_u_keyboard,
    controller: controller,
    controlled: controlled,
    mode: PtyTestMode::Raw,
}

/// [`PTY`] Controller: Send [`Kitty`] keyboard protocol sequences and verify parsing.
///
/// [`Kitty`]: https://sw.kovidgoyal.net/kitty/
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
fn controller(context: PtyTestContext) {
    let PtyTestContext {
        pty_pair,
        child,
        mut buf_reader,
        mut writer,
    } = context;

    eprintln!("{GLYPH_CONTROLLER} PTY Controller: Starting Kitty CSI u keyboard test...");

    eprintln!(
        "{GLYPH_WAITING} PTY Controller: Waiting for controlled process to start..."
    );

    // Wait for controlled process to be ready.
    child
        .wait_for_ready(&mut buf_reader, MSG_CONTROLLED_READY)
        .expect("Failed to wait for MSG_CONTROLLED_READY");
    eprintln!("  {GLYPH_SUCCESS} Controlled is ready (input device created)");

    let test_cases = generate_test_cases();

    eprintln!(
        "{} PTY Controller: Sending {} standard Kitty CSI u test sequences...",
        GLYPH_WAITING,
        test_cases.len()
    );

    for test_case in &test_cases {
        let TestCase {
            description,
            sequence,
            expected_pattern,
        } = test_case;

        eprintln!("  → Sending: {description} ({sequence:?})");

        writer
            .write_all(sequence)
            .expect("Failed to write sequence");
        writer.flush().expect("Failed to flush");

        let event_line =
            child.read_line_state(&mut buf_reader, |line| line.starts_with("Keyboard:"));

        assert!(
            event_line.contains(expected_pattern),
            "Assertion failed for {description}: expected output containing '{expected_pattern}', got: '{event_line}'"
        );
        eprintln!("  {GLYPH_SUCCESS} {description}: {event_line}");
    }

    // 6. Release event (:3) absorption verification.
    // Send release event followed by marker keystroke 'Z'. The release event must be
    // absorbed as Ignored and not produce an InputEvent.
    eprintln!(
        "{GLYPH_WAITING} PTY Controller: Verifying release event (:3) absorption..."
    );
    writer
        .write_all(&csi_u_with_event_type(
            OPEN_BRACKET_CODEPOINT,
            MODIFIER_ALT,
            KITTY_EVENT_RELEASE,
        ))
        .expect("Failed to write release event");
    writer
        .write_all(b"Z")
        .expect("Failed to write marker keystroke");
    writer.flush().expect("Failed to flush");

    let event_line =
        child.read_line_state(&mut buf_reader, |line| line.starts_with("Keyboard:"));
    assert!(
        event_line.contains("Character('Z')"),
        "Release event was not ignored! Got: '{event_line}'"
    );
    eprintln!("  {GLYPH_SUCCESS} Release event absorbed, received marker: {event_line}");

    eprintln!("{GLYPH_CONTROLLER_CLEANUP} PTY Controller: Cleaning up...");

    drop(writer);
    child.drain_and_wait(buf_reader, pty_pair);

    eprintln!("{GLYPH_SUCCESS} PTY Controller: Kitty CSI u keyboard test passed!");
}

/// [`PTY`] Controlled: Read and parse keyboard events.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
fn controlled() {
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
                            eprintln!(
                                "{GLYPH_CONTROLLED} PTY Controlled: Event #{event_count}: {event:?}"
                            );

                            let output = match event {
                                InputEvent::Keyboard(ref key_press) => {
                                    format!("Keyboard: {key_press:?}")
                                }
                                _ => {
                                    format!("Event: {event:?}")
                                }
                            };

                            println!("{output}");
                            std::io::stdout().flush().expect("Failed to flush stdout");

                            if event_count >= TOTAL_EXPECTED_EVENTS {
                                eprintln!(
                                    "{GLYPH_CONTROLLED} PTY Controlled: Processed {event_count} events, exiting"
                                );
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
                    eprintln!(
                        "{GLYPH_CONTROLLED} PTY Controlled: Inactivity timeout (5 seconds with no events), exiting"
                    );
                    break;
                }
            }
        }

        eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Completed, exiting");
    });
}

/// A single test case for [`Kitty`] `CSI u` sequence verification.
///
/// [`Kitty`]: https://sw.kovidgoyal.net/kitty/
struct TestCase {
    description: &'static str,
    sequence: Vec<u8>,
    expected_pattern: &'static str,
}

/// Generate the structured test cases for [`Kitty`] `CSI u` keyboard protocol testing.
///
/// [`Kitty`]: https://sw.kovidgoyal.net/kitty/
#[allow(clippy::too_many_lines)]
fn generate_test_cases() -> Vec<TestCase> {
    vec![
        // 1. Disambiguated key combinations
        TestCase {
            description: "Alt+[ (CSI collision resolution)",
            sequence: csi_u('[', MODIFIER_ALT),
            expected_pattern: "Character('['), mask: ModifierKeysMask { shift_key_state: NotPressed, ctrl_key_state: NotPressed, alt_key_state: Pressed }",
        },
        TestCase {
            description: "Shift+Enter",
            sequence: csi_u_codepoint(u32::from(CONTROL_ENTER), MODIFIER_SHIFT),
            expected_pattern: "SpecialKey(Enter), mask: ModifierKeysMask { shift_key_state: Pressed, ctrl_key_state: NotPressed, alt_key_state: NotPressed }",
        },
        TestCase {
            description: "Ctrl+Tab",
            sequence: csi_u_codepoint(u32::from(CONTROL_TAB), MODIFIER_CTRL),
            expected_pattern: "SpecialKey(Tab), mask: ModifierKeysMask { shift_key_state: NotPressed, ctrl_key_state: Pressed, alt_key_state: NotPressed }",
        },
        TestCase {
            description: "Alt+Escape",
            sequence: csi_u_codepoint(u32::from(CONTROL_ESC), MODIFIER_ALT),
            expected_pattern: "SpecialKey(Esc), mask: ModifierKeysMask { shift_key_state: NotPressed, ctrl_key_state: NotPressed, alt_key_state: Pressed }",
        },
        TestCase {
            description: "Shift+Backspace (codepoint 127)",
            sequence: csi_u_codepoint(u32::from(ASCII_DEL), MODIFIER_SHIFT),
            expected_pattern: "SpecialKey(Backspace), mask: ModifierKeysMask { shift_key_state: Pressed, ctrl_key_state: NotPressed, alt_key_state: NotPressed }",
        },
        TestCase {
            description: "Ctrl+Backspace (codepoint 8)",
            sequence: csi_u_codepoint(u32::from(CONTROL_BACKSPACE), MODIFIER_CTRL),
            expected_pattern: "SpecialKey(Backspace), mask: ModifierKeysMask { shift_key_state: NotPressed, ctrl_key_state: Pressed, alt_key_state: NotPressed }",
        },
        // 2. Kitty PUA functional and navigation keys
        TestCase {
            description: "Shift+Home (PUA 57366)",
            sequence: csi_u_codepoint(KITTY_PUA_HOME, MODIFIER_SHIFT),
            expected_pattern: "SpecialKey(Home), mask: ModifierKeysMask { shift_key_state: Pressed, ctrl_key_state: NotPressed, alt_key_state: NotPressed }",
        },
        TestCase {
            description: "Ctrl+Delete (PUA 57359)",
            sequence: csi_u_codepoint(KITTY_PUA_DELETE, MODIFIER_CTRL),
            expected_pattern: "SpecialKey(Delete), mask: ModifierKeysMask { shift_key_state: NotPressed, ctrl_key_state: Pressed, alt_key_state: NotPressed }",
        },
        TestCase {
            description: "Shift+Insert (PUA 57358)",
            sequence: csi_u_codepoint(KITTY_PUA_INSERT, MODIFIER_SHIFT),
            expected_pattern: "SpecialKey(Insert), mask: ModifierKeysMask { shift_key_state: Pressed, ctrl_key_state: NotPressed, alt_key_state: NotPressed }",
        },
        TestCase {
            description: "PageUp (PUA 57364)",
            sequence: csi_u_codepoint(KITTY_PUA_PAGE_UP, MODIFIER_NONE),
            expected_pattern: "Plain { key: SpecialKey(PageUp) }",
        },
        TestCase {
            description: "PageDown (PUA 57365)",
            sequence: csi_u_codepoint(KITTY_PUA_PAGE_DOWN, MODIFIER_NONE),
            expected_pattern: "Plain { key: SpecialKey(PageDown) }",
        },
        TestCase {
            description: "Up Arrow (PUA 57362)",
            sequence: csi_u_codepoint(KITTY_PUA_UP, MODIFIER_NONE),
            expected_pattern: "Plain { key: SpecialKey(Up) }",
        },
        TestCase {
            description: "Down Arrow (PUA 57363)",
            sequence: csi_u_codepoint(KITTY_PUA_DOWN, MODIFIER_NONE),
            expected_pattern: "Plain { key: SpecialKey(Down) }",
        },
        TestCase {
            description: "Left Arrow (PUA 57360)",
            sequence: csi_u_codepoint(KITTY_PUA_LEFT, MODIFIER_NONE),
            expected_pattern: "Plain { key: SpecialKey(Left) }",
        },
        TestCase {
            description: "Right Arrow (PUA 57361)",
            sequence: csi_u_codepoint(KITTY_PUA_RIGHT, MODIFIER_NONE),
            expected_pattern: "Plain { key: SpecialKey(Right) }",
        },
        TestCase {
            description: "End (PUA 57367)",
            sequence: csi_u_codepoint(KITTY_PUA_END, MODIFIER_NONE),
            expected_pattern: "Plain { key: SpecialKey(End) }",
        },
        // 3. Kitty function keys
        TestCase {
            description: "F1 (PUA 57376)",
            sequence: csi_u_codepoint(KITTY_PUA_F1, MODIFIER_NONE),
            expected_pattern: "Plain { key: FunctionKey(F1) }",
        },
        TestCase {
            description: "Shift+F12 (PUA 57387)",
            sequence: csi_u_codepoint(KITTY_PUA_F12, MODIFIER_SHIFT),
            expected_pattern: "FunctionKey(F12), mask: ModifierKeysMask { shift_key_state: Pressed, ctrl_key_state: NotPressed, alt_key_state: NotPressed }",
        },
        // 4. Omitted modifiers and subparam alternate keys
        TestCase {
            description: "Plain '[' with omitted modifier",
            sequence: format!("{CSI_START}{OPEN_BRACKET_CODEPOINT}{ANSI_CSI_U_CHAR}").into_bytes(),
            expected_pattern: "Plain { key: Character('[') }",
        },
        TestCase {
            description: "Plain '[' with trailing semicolon",
            sequence: format!("{CSI_START}{OPEN_BRACKET_CODEPOINT}{ANSI_PARAM_SEPARATOR_CHAR}{ANSI_CSI_U_CHAR}").into_bytes(),
            expected_pattern: "Plain { key: Character('[') }",
        },
        TestCase {
            description: "Alt+'[' with alternate key subparameter",
            sequence: csi_u_with_alternate_key(OPEN_BRACKET_CODEPOINT, CLOSE_BRACKET_CODEPOINT, MODIFIER_ALT),
            expected_pattern: "Character('['), mask: ModifierKeysMask { shift_key_state: NotPressed, ctrl_key_state: NotPressed, alt_key_state: Pressed }",
        },
        // 5. Event types
        TestCase {
            description: "Press event type (:1)",
            sequence: csi_u_with_event_type(OPEN_BRACKET_CODEPOINT, MODIFIER_ALT, KITTY_EVENT_PRESS),
            expected_pattern: "Character('['), mask: ModifierKeysMask { shift_key_state: NotPressed, ctrl_key_state: NotPressed, alt_key_state: Pressed }",
        },
        TestCase {
            description: "Repeat event type (:2)",
            sequence: csi_u_with_event_type(OPEN_BRACKET_CODEPOINT, MODIFIER_ALT, KITTY_EVENT_REPEAT),
            expected_pattern: "Character('['), mask: ModifierKeysMask { shift_key_state: NotPressed, ctrl_key_state: NotPressed, alt_key_state: Pressed }",
        },
    ]
}

/// Expected number of keyboard events processed by the controlled process:
/// - 23 standard `CSI u` test cases
/// - 1 marker keystroke `'Z'` following absorbed release event `:3`
const TOTAL_EXPECTED_EVENTS: usize = 24;

// XMARK: Intentional numeric casting using as.
#[allow(clippy::as_conversions)]
const OPEN_BRACKET_CODEPOINT: u32 = ANSI_CSI_BRACKET as u32;

// XMARK: Intentional numeric casting using as.
#[allow(clippy::as_conversions)]
const CLOSE_BRACKET_CODEPOINT: u32 = ANSI_OSC_CLOSE_BRACKET as u32;
