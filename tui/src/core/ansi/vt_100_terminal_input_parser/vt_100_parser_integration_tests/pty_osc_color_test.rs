// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// XMARK: pty test faking terminal emulator (in controller)

//! [`PTY`]-based integration test for [`OSC`] color queries and reports.
//!
//! Validates end-to-end bidirectional [`OSC`] color query and response handling in a
//! real pseudoterminal environment:
//! - Queries all 7 supported terminal color roles: [`Foreground`], [`Background`],
//!   [`Cursor`], [`MouseForeground`], [`MouseBackground`], [`Highlight`], and
//!   [`HighlightForeground`]
//! - Alternating string terminators: `BEL` (`0x07`) and `ST` (`ESC \`)
//! - Multi-depth RGB hex formats: 4-digit (`rgb:aaaa/bbbb/cccc`), 2-digit
//!   (`rgb:12/34/56`), 1-digit (`rgb:f/0/8`), and hash hex (`#rrggbb`)
//! - Modern terminal emulator security: Inbound [`OSC 52`] clipboard payloads are safely
//!   absorbed as [`VT100InputEventIR::Ignored`] without leaking phantom keystrokes
//!
//! # Architecture and Process Roles
//!
//! In this test, the controller process simulates a host terminal emulator while the
//! controlled child runs our TUI input engine. For the overall architecture and testing
//! philosophy, see the [process roles documentation].
//!
//! 1. **[`PTY`] Controller (Simulated Terminal Emulator)**:
//!    - Simulates an external host terminal emulator (such as Ghostty, [`Kitty`], or
//!      [`Alacritty`]) responding to client application environment queries.
//!    - Reads query strings emitted by the controlled child over [`stdout`], asserting
//!      that the query format matches the expected role code and [`OSC_QUERY`] character
//!      (`'?'`).
//!    - Synthesizes and writes wire color responses back into the controller side of the
//!      [`PTY`], which the operating system delivers to the child's [`stdin`].
//!    - Injects an inbound [`OSC 52`] clipboard response followed immediately by marker
//!      keystroke `'Z'` to confirm stream responsiveness and verify that clipboard reads
//!      are safely absorbed.
//!    - Reads line-by-line confirmation signals from the child's [`stdout`].
//!
//! 2. **[`PTY`] Controlled (System Under Test / TUI Input Engine)**:
//!    - Represents the real TUI application running inside the child subprocess connected
//!      to the controlled side of the [`PTY`].
//!    - Requests colors via [`OscSender::send_color_query`], which writes query bytes to
//!      [`stdout`].
//!    - Initializes [`DirectToAnsiInputDevice`].
//!    - Behind the scenes, the background [`mio`] polling thread watches [`stdin`],
//!      feeding raw bytes into [`ChunkFramer`] and the Sans-IO [`vt_100`] parser.
//!    - Decodes incoming color report sequences into [`InputEvent::TerminalColorReport`]
//!      using [`OscSequence::try_parse()`].
//!    - Asserts that decoded color values match the exact expected [`RgbValue`] across
//!      all 7 roles.
//!    - Safely absorbs inbound [`OSC 52`] payloads as [`VT100InputEventIR::Ignored`]
//!      without emitting spurious keystroke events.
//!
//! # Security Callout: Why Absorb [`OSC 52`]?
//!
//! Modern terminal emulators forbid clipboard read queries by default because allowing
//! untrusted terminal applications to silently inspect the host clipboard creates severe
//! security vulnerabilities (such as exfiltrating passwords, tokens, and private keys).
//! Because clipboard queries are untrusted, [`DirectToAnsiInputDevice`] mimics the
//! security posture of modern terminal emulators and safely discards inbound [`OSC 52`]
//! payloads. Bracketed paste is used for intentional clipboard input instead.
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui --lib test_pty_osc_color -- --nocapture
//! ```
//!
//! [`Alacritty`]: https://alacritty.org/
//! [`Background`]: crate::TerminalColorRole::Background
//! [`ChunkFramer`]: crate::core::ansi::vt_100_terminal_input_parser::ChunkFramer
//! [`Cursor`]: crate::TerminalColorRole::Cursor
//! [`DirectToAnsiInputDevice`]: crate::direct_to_ansi::DirectToAnsiInputDevice
//! [`Foreground`]: crate::TerminalColorRole::Foreground
//! [`Highlight`]: crate::TerminalColorRole::Highlight
//! [`HighlightForeground`]: crate::TerminalColorRole::HighlightForeground
//! [`InputEvent::TerminalColorReport`]: crate::InputEvent::TerminalColorReport
//! [`Kitty`]: https://sw.kovidgoyal.net/kitty/
//! [`mio`]: https://crates.io/crates/mio
//! [`MouseBackground`]: crate::TerminalColorRole::MouseBackground
//! [`MouseForeground`]: crate::TerminalColorRole::MouseForeground
//! [`OSC 52`]: crate::core::ansi::osc::OscSequence::ClipboardSet
//! [`OSC_QUERY`]: crate::core::ansi::constants::OSC_QUERY
//! [`OSC`]: crate::core::ansi::osc::OscSequence
//! [`OscSender::send_color_query`]: crate::core::ansi::osc::OscSender::send_color_query
//! [`OscSequence::try_parse()`]: crate::core::ansi::osc::OscSequence::try_parse
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`RgbValue`]: crate::RgbValue
//! [`stdin`]: std::io::stdin
//! [`stdout`]: std::io::stdout
//! [`TerminalColorReport`]: crate::TerminalColorReport
//! [`VT100InputEventIR::Ignored`]: crate::core::ansi::vt_100_terminal_input_parser::VT100InputEventIR::Ignored
//! [`vt_100`]: crate::core::ansi::vt_100_terminal_input_parser
//! [process roles documentation]: mod@super#architecture-and-process-roles

use crate::{GLYPH_CONTROLLED, GLYPH_CONTROLLER, GLYPH_CONTROLLER_CLEANUP, GLYPH_SUCCESS,
            GLYPH_WAITING, InputEvent, Key, KeyPress, MSG_CONTROLLED_READY,
            MSG_CONTROLLED_STARTING, MSG_TEST_RUNNING, OscSender, OscSequence,
            OutputDevice, PtyTestContext, PtyTestMode, RgbValue, TerminalColorRole,
            core::ansi::{constants::{OSC_CODE_COLOR_REPORT_BACKGROUND,
                                     OSC_CODE_COLOR_REPORT_CURSOR,
                                     OSC_CODE_COLOR_REPORT_FOREGROUND,
                                     OSC_CODE_COLOR_REPORT_HIGHLIGHT,
                                     OSC_CODE_COLOR_REPORT_HIGHLIGHT_FOREGROUND,
                                     OSC_CODE_COLOR_REPORT_MOUSE_BACKGROUND,
                                     OSC_CODE_COLOR_REPORT_MOUSE_FOREGROUND,
                                     OSC_COLOR_SPEC_RGB_PREFIX, OSC_DELIMITER,
                                     OSC_QUERY, OSC_START, OSC_TERMINATOR_BEL,
                                     OSC_TERMINATOR_ST},
                         generator::generate_keyboard_sequence,
                         osc::ClipboardTarget,
                         vt_100_terminal_input_parser::ir_event_types::{VT100InputEventIR,
                                                                        VT100KeyCodeIR,
                                                                        VT100KeyModifiersIR}},
            generate_pty_test,
            tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice};
use std::{io::Write, time::Duration};

generate_pty_test! {
    test_fn: test_pty_osc_color,
    controller: controller,
    controlled: controlled,
    mode: PtyTestMode::Raw,
}

/// [`PTY`] Controller: Emulate host terminal response to color queries and clipboard.
///
/// Acts as the simulated terminal emulator peer connected to the controller side of the
/// [`PTY`]. Reads child query strings from stdout, verifies them, and writes back
/// synthesized color reports and clipboard payloads.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
fn controller(context: PtyTestContext) {
    let PtyTestContext {
        pty_pair,
        child,
        mut buf_reader,
        mut writer,
    } = context;

    eprintln!("{GLYPH_CONTROLLER} PTY Controller: Starting OSC color query test...");
    eprintln!(
        "{GLYPH_WAITING} PTY Controller: Waiting for controlled process to start..."
    );

    // Wait for controlled to confirm it's running and ready.
    child
        .wait_for_ready(&mut buf_reader, MSG_CONTROLLED_READY)
        .expect("Failed to wait for ready signal");

    // Test OSC color queries and responses across all 7 roles, terminators, and formats.
    for test_case in generate_test_cases() {
        let role = test_case.role;
        eprintln!("{GLYPH_WAITING} Controller: Waiting for query for {role:?}...");
        let query_line = child.read_line_state(&mut buf_reader, |line| {
            line.contains(role.as_str()) && line.contains(OSC_QUERY)
        });
        eprintln!("  ← Child asked: {query_line:?}");

        // Verify query contains the expected OSC code and '?'.
        assert!(
            query_line.contains(role.as_str()),
            "Expected query for {role:?} to contain OSC code {}, got: {query_line}",
            role.as_str()
        );
        assert!(
            query_line.contains(OSC_QUERY),
            "Expected query for {role:?} to contain '?', got: {query_line}"
        );

        // Synthesize and send wire response.
        let report_str = &test_case.wire_response;
        eprintln!("  → Controller sending report: {report_str:?}");
        writer
            .write_all(report_str.as_bytes())
            .expect("Failed to write color report to PTY");
        writer.flush().expect("Failed to flush color report");

        // Wait for child confirmation.
        let expected_conf = format!("CONFIRMED:{role:?}");
        let conf_line = child
            .read_line_state(&mut buf_reader, |line| line.starts_with(&expected_conf));
        eprintln!("  {GLYPH_SUCCESS} Child confirmed: {conf_line}");
    }

    // Verify OSC 52 clipboard absorption without spurious events.
    eprintln!("{GLYPH_WAITING} Controller: Waiting for OSC 52 test readiness...");
    child.read_line_state(&mut buf_reader, |line| line == "READY_FOR_OSC52");

    eprintln!(
        "  → Controller sending OSC 52 response followed by marker keystroke 'Z'..."
    );
    let osc52 = OscSequence::ClipboardSet {
        target: ClipboardTarget::System,
        data: "Hello".to_string(),
    };
    // Trailing marker keystroke 'Z' verifies that the OSC 52 payload was cleanly
    // absorbed: if phantom keystrokes leaked, the child dequeues those instead of 'Z'.
    let marker_keystroke = generate_keyboard_sequence(&VT100InputEventIR::Keyboard {
        code: VT100KeyCodeIR::Char('Z'),
        modifiers: VT100KeyModifiersIR::NONE,
    })
    .expect("Failed to generate marker keystroke sequence");

    let mut osc52_payload = osc52.to_string().into_bytes();
    osc52_payload.extend(marker_keystroke);

    writer
        .write_all(&osc52_payload)
        .expect("Failed to write OSC 52 payload to PTY");
    writer.flush().expect("Failed to flush OSC 52 payload");

    let osc52_conf = child.read_line_state(&mut buf_reader, |line| {
        line.starts_with("CONFIRMED:Osc52Absorbed")
    });
    eprintln!("  {GLYPH_SUCCESS} Child confirmed: {osc52_conf}");

    eprintln!("{GLYPH_CONTROLLER_CLEANUP} PTY Controller: Cleaning up...");
    drop(writer);
    child.drain_and_wait(buf_reader, pty_pair);

    eprintln!("{GLYPH_SUCCESS} PTY Controller: Test passed!");
}

/// [`PTY`] Controlled: Queries terminal colors and verifies decoded events.
///
/// Runs inside the child process connected to the controlled side of the [`PTY`].
/// Exercises [`OscSender::send_color_query`] on stdout, and uses the Sans-IO
/// [`DirectToAnsiInputDevice`] to frame and decode inbound response bytes on stdin.
///
/// The harness performs [`std::process::exit(0)`] after this function returns.
///
/// [`DirectToAnsiInputDevice`]: crate::direct_to_ansi::DirectToAnsiInputDevice
/// [`OscSender::send_color_query`]: crate::core::ansi::osc::OscSender::send_color_query
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
fn controlled() {
    println!("{MSG_TEST_RUNNING}");
    println!("{MSG_CONTROLLED_STARTING}");
    std::io::stdout().flush().expect("Failed to flush stdout");

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Starting...");
        let mut input_device = DirectToAnsiInputDevice::new()
            .expect("Failed to initialize DirectToAnsiInputDevice");
        eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Device created, ready...");

        // Signal ready to controller.
        println!("{MSG_CONTROLLED_READY}");
        std::io::stdout().flush().expect("Failed to flush stdout");

        let output_device = OutputDevice::new_stdout();
        let mut osc_sender = OscSender::new(&output_device);
        let timeout = Duration::from_secs(5);

        // Test OSC color queries and responses across all 7 roles, terminators, and
        // formats.
        for case in generate_test_cases() {
            let role = case.role;
            // Send query to stdout via `OscSender`.
            osc_sender
                .send_color_query(role)
                .expect("Failed to send color query via OscSender");
            println!();
            std::io::stdout().flush().expect("Failed to flush query");

            // Await response from input_device with timeout.
            let event = tokio::time::timeout(timeout, input_device.next())
                .await
                .unwrap_or_else(|_| {
                    panic!("Timed out waiting for color report for {role:?}")
                })
                .expect("Input device stream ended unexpectedly");

            match event {
                InputEvent::TerminalColorReport(report) => {
                    assert_eq!(report.role, role);
                    assert_eq!(
                        report.color, case.expected_color,
                        "Color mismatch for {role:?}: expected {:?}, got {:?}",
                        case.expected_color, report.color
                    );
                    println!("CONFIRMED:{role:?}:{:?}", report.color);
                    std::io::stdout()
                        .flush()
                        .expect("Failed to flush confirmation");
                }
                other => {
                    panic!("Expected TerminalColorReport for {role:?}, got: {other:?}")
                }
            }
        }

        // Test OSC 52 absorption.
        println!("READY_FOR_OSC52");
        std::io::stdout()
            .flush()
            .expect("Failed to flush readiness");

        let event = tokio::time::timeout(timeout, input_device.next())
            .await
            .expect("Timed out waiting for marker keystroke after OSC 52")
            .expect("Input device stream ended unexpectedly");

        match event {
            InputEvent::Keyboard(KeyPress::Plain {
                key: Key::Character('Z'),
            }) => {
                println!("CONFIRMED:Osc52Absorbed");
                std::io::stdout()
                    .flush()
                    .expect("Failed to flush confirmation");
            }
            other => {
                panic!("Expected Keyboard('Z') after absorbed OSC 52, got: {other:?}")
            }
        }

        eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Completed successfully");
    });
}

// -------------------------------------------------------------------------------------------------
// Test cases
// -------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
struct TestCase {
    pub role: TerminalColorRole,
    pub wire_response: String,
    pub expected_color: RgbValue,
}

impl TestCase {
    pub fn new(
        role: TerminalColorRole,
        wire_response: String,
        expected_color: RgbValue,
    ) -> Self {
        Self {
            role,
            wire_response,
            expected_color,
        }
    }
}

fn generate_test_cases() -> Vec<TestCase> {
    vec![
        // 1. Foreground (OSC 10): 4-digit hex with ST terminator.
        TestCase::new(
            TerminalColorRole::Foreground,
            format!(
                "{OSC_START}{OSC_CODE_COLOR_REPORT_FOREGROUND}{OSC_DELIMITER}{OSC_COLOR_SPEC_RGB_PREFIX}aaaa/bbbb/cccc{OSC_TERMINATOR_ST}"
            ),
            RgbValue::from_u8(0xaa, 0xbb, 0xcc),
        ),
        // 2. Background (OSC 11): 2-digit hex with BEL terminator.
        TestCase::new(
            TerminalColorRole::Background,
            format!(
                "{OSC_START}{OSC_CODE_COLOR_REPORT_BACKGROUND}{OSC_DELIMITER}{OSC_COLOR_SPEC_RGB_PREFIX}12/34/56{OSC_TERMINATOR_BEL}"
            ),
            RgbValue::from_u8(0x12, 0x34, 0x56),
        ),
        // 3. Cursor (OSC 12): 1-digit hex with ST terminator.
        TestCase::new(
            TerminalColorRole::Cursor,
            format!(
                "{OSC_START}{OSC_CODE_COLOR_REPORT_CURSOR}{OSC_DELIMITER}{OSC_COLOR_SPEC_RGB_PREFIX}f/0/8{OSC_TERMINATOR_ST}"
            ),
            RgbValue::from_u8(255, 0, 136),
        ),
        // 4. MouseForeground (OSC 13): Hash hex `#rrggbb` with BEL terminator.
        TestCase::new(
            TerminalColorRole::MouseForeground,
            format!(
                "{OSC_START}{OSC_CODE_COLOR_REPORT_MOUSE_FOREGROUND}{OSC_DELIMITER}#1a2b3c{OSC_TERMINATOR_BEL}"
            ),
            RgbValue::from_u8(0x1a, 0x2b, 0x3c),
        ),
        // 5. MouseBackground (OSC 14): 4-digit hex with BEL terminator.
        TestCase::new(
            TerminalColorRole::MouseBackground,
            format!(
                "{OSC_START}{OSC_CODE_COLOR_REPORT_MOUSE_BACKGROUND}{OSC_DELIMITER}{OSC_COLOR_SPEC_RGB_PREFIX}4444/5555/6666{OSC_TERMINATOR_BEL}"
            ),
            RgbValue::from_u8(0x44, 0x55, 0x66),
        ),
        // 6. Highlight (OSC 17): 2-digit hex with ST terminator.
        TestCase::new(
            TerminalColorRole::Highlight,
            format!(
                "{OSC_START}{OSC_CODE_COLOR_REPORT_HIGHLIGHT}{OSC_DELIMITER}{OSC_COLOR_SPEC_RGB_PREFIX}78/9a/bc{OSC_TERMINATOR_ST}"
            ),
            RgbValue::from_u8(0x78, 0x9a, 0xbc),
        ),
        // 7. HighlightForeground (OSC 19): Hash hex `#rrggbb` with ST terminator.
        TestCase::new(
            TerminalColorRole::HighlightForeground,
            format!(
                "{OSC_START}{OSC_CODE_COLOR_REPORT_HIGHLIGHT_FOREGROUND}{OSC_DELIMITER}#fedcba{OSC_TERMINATOR_ST}"
            ),
            RgbValue::from_u8(0xfe, 0xdc, 0xba),
        ),
    ]
}
