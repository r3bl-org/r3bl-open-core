// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`PTY`]-based integration test for [`OSC`] color queries and reports.
//!
//! In this test, the controller parent simulates an external terminal emulator by
//! generating [`OSC`] responses to the child's queries. The controlled child acts
//! as the terminal application, and it queries the terminal's colors (background,
//! foreground, cursor) and verifies the responses (decoded reports), while also
//! confirming that inbound [`OSC 52`] clipboard payloads are safely absorbed without
//! leaking phantom keystrokes.
//!
//! > 💡 **Why absorb [`OSC 52`]?** Modern terminal emulators (such as Ghostty, [`Kitty`],
//! > and [`Alacritty`]) forbid clipboard read queries by default because allowing
//! > programs
//! > to silently inspect the host clipboard is a major security risk (exfiltrating
//! > passwords and secrets). Because clipboard queries are untrusted,
//! > [`DirectToAnsiInputDevice`] mimics the security posture of modern terminal
//! > emulators and safely absorbs and discards inbound [`OSC 52`] payloads as
//! > [`VT100InputEventIR::Ignored`] (bracketed paste is used for clipboard input
//! > instead).
//!
//! Here are the details of this test which validates end-to-end bidirectional [`OSC`]
//! color query & response handling:
//! - Controlled child requests colors via [`OscSender::send_color_query`] which writes
//!   query bytes to [`stdout`].
//! - Controller parent simulates the host terminal emulator over the [`PTY`], verifying
//!   the query format and emitting [`OscSequence::ColorReport`] response bytes.
//! - Controlled child decodes the response into [`InputEvent::TerminalColor`] through the
//!   Sans-IO pipeline: [`DirectToAnsiInputDevice`] -> [`ChunkFramer`] (scanned by
//!   [`OscScanResult`]) -> [`OscSequence::try_parse()`].
//! - Controlled child asserts that the decoded [`TerminalColorReport`] matches the shared
//!   test fixtures across [`TerminalColorRole::Background`],
//!   [`TerminalColorRole::Foreground`], and [`TerminalColorRole::Cursor`].
//! - Verifies that inbound [`OSC 52`] clipboard responses are safely absorbed as
//!   [`VT100InputEventIR::Ignored`] without emitting phantom keystrokes.
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui --lib test_pty_osc_color -- --nocapture
//! ```
//!
//! [`Alacritty`]: https://alacritty.org/
//! [`ChunkFramer`]: crate::core::ansi::vt_100_terminal_input_parser::ChunkFramer
//! [`DirectToAnsiInputDevice`]: crate::direct_to_ansi::DirectToAnsiInputDevice
//! [`InputEvent::TerminalColor`]: crate::InputEvent::TerminalColor
//! [`Kitty`]: https://sw.kovidgoyal.net/kitty/
//! [`OSC 52`]: crate::core::ansi::osc::OscSequence::ClipboardSet
//! [`OSC`]: crate::core::ansi::osc::OscSequence
//! [`OscScanResult`]: crate::core::ansi::vt_100_terminal_input_parser::chunk_decoder::osc_scanner::OscScanResult
//! [`OscSender::send_color_query`]: crate::core::ansi::osc::OscSender::send_color_query
//! [`OscSequence::ColorReport`]: crate::core::ansi::osc::OscSequence::ColorReport
//! [`OscSequence::try_parse()`]: crate::core::ansi::osc::OscSequence::try_parse
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`Sans-IO`]: https://sans-io.readthedocs.io/
//! [`stdin`]: std::io::stdin
//! [`stdout`]: std::io::stdout
//! [`TerminalColorReport`]: crate::TerminalColorReport
//! [`TerminalColorRole::Background`]: crate::TerminalColorRole::Background
//! [`TerminalColorRole::Cursor`]: crate::TerminalColorRole::Cursor
//! [`TerminalColorRole::Foreground`]: crate::TerminalColorRole::Foreground
//! [`VT100InputEventIR::Ignored`]: crate::core::ansi::vt_100_terminal_input_parser::VT100InputEventIR::Ignored

use crate::{GLYPH_CONTROLLED, GLYPH_CONTROLLER, GLYPH_CONTROLLER_CLEANUP, GLYPH_SUCCESS,
            GLYPH_WAITING, InputEvent, Key, KeyPress, MSG_CONTROLLED_READY,
            MSG_CONTROLLED_STARTING, MSG_TEST_RUNNING, OscSender, OscSequence,
            OutputDevice, PtyTestContext, PtyTestMode, RgbValue, TerminalColorReport,
            TerminalColorRole,
            core::ansi::{constants::OSC_QUERY,
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
/// Acts as the simulated terminal emulator peer connected to the master side of the
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

    // Test OSC color queries and responses.
    for expected_report in test_cases() {
        let role = expected_report.role;
        eprintln!("{GLYPH_WAITING} Controller: Waiting for query for {role:?}...");
        let query_line = child.read_line_state(&mut buf_reader, |line| {
            line.contains(role.as_str()) && line.contains(OSC_QUERY)
        });
        eprintln!("  → Child asked: {query_line:?}");

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

        // Synthesize ColorReport response.
        let report = OscSequence::ColorReport(expected_report);
        let report_str = report.to_string();
        eprintln!("  ← Controller sending report: {report_str:?}");
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
        "  ← Controller sending OSC 52 response followed by marker keystroke 'Z'..."
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
/// Runs inside the child process connected to the slave side of the [`PTY`].
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

        // Test OSC color queries and responses.
        for expected_report in test_cases() {
            let role = expected_report.role;
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
                InputEvent::TerminalColor(report) => {
                    assert_eq!(report, expected_report);
                    println!("CONFIRMED:{role:?}:{:?}", report.color);
                    std::io::stdout()
                        .flush()
                        .expect("Failed to flush confirmation");
                }
                other => panic!("Expected TerminalColor for {role:?}, got: {other:?}"),
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

fn test_cases() -> Vec<TerminalColorReport> {
    vec![
        TerminalColorReport {
            role: TerminalColorRole::Background,
            color: RgbValue {
                red: 18,
                green: 20,
                blue: 24,
            },
        },
        TerminalColorReport {
            role: TerminalColorRole::Foreground,
            color: RgbValue {
                red: 220,
                green: 220,
                blue: 220,
            },
        },
        TerminalColorReport {
            role: TerminalColorRole::Cursor,
            color: RgbValue {
                red: 255,
                green: 0,
                blue: 128,
            },
        },
    ]
}
