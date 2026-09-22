// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// XMARK: pty test faking terminal emulator (in controller)

//! [`PTY`]-based integration test for [`OSC`] stream safety and circuit breaker defense.
//!
//! Validates that the [`Sans-IO`] input parser and stateful stream safety layer
//! ([`ChunkFramer`] and [`OscCircuitBreaker`]) handle rogue, oversized, and malformed
//! streams over a real [`PTY`] without memory growth, deadlocks, or event stream
//! poisoning.
//!
//! # Architecture and Process Roles
//!
//! In this test, the controller process acts as a fault injector simulating a host
//! terminal emulator while the controlled child runs our TUI input engine. For the
//! overall architecture and testing philosophy, see the [process roles documentation].
//!
//! 1. **[`PTY`] Controller / Test Driver (Fault Injector)**:
//!    - Holds the controller side of the [`PTY`].
//!    - **What the Fault Injector Simulates**:
//!      - **Rogue / Buggy Terminal Emulators**: Terminals or multiplexers (e.g. `tmux`,
//!        `screen`, `mosh`) that suffer internal bugs, buffer overflows, or missing
//!        string terminators (`BEL` / `ST`), spewing unbounded bytes into standard input.
//!      - **Adversarial Input / `DoS` Attacks**: Untrusted data streams or malicious
//!        payloads (e.g. catting an untrusted file or hostile escape sequence injection)
//!        attempting to cause memory exhaustion (OOM) by feeding endless [`OSC`] payloads
//!        into the parser's accumulator.
//!      - **Proprietary / Unknown Extensions**: Modern terminals ([`Kitty`], [`iTerm2`],
//!        [`WezTerm`]) emitting non-standard or experimental [`OSC`] codes that the
//!        parser must safely ignore without emitting phantom keypresses or corrupting
//!        subsequent input.
//!      - **Truncated or Corrupted Streams**: Flaky remote connections (e.g. mobile SSH
//!        drops) or intermediate terminal multiplexers (`tmux`, `zellij`) that drop
//!        buffers under high load, delivering truncated or malformed payloads.
//!    - After injecting each fault, pumps legitimate marker keystrokes (`'A'`, `'B'`,
//!      `'C'`) down the wire to verify that the child's input pipeline recovered
//!      completely and remains fully responsive.
//!
//! 2. **[`PTY`] Controlled / System Under Test (TUI Input Engine)**:
//!    - Represents our application's input subsystem running on the terminal controlled
//!      device.
//!    - Initializes [`DirectToAnsiInputDevice`].
//!    - Behind the scenes, [`DirectToAnsiInputDevice`] runs the background [`mio`]
//!      polling thread on standard input, driving [`ChunkFramer`] and
//!      [`OscCircuitBreaker`].
//!    - Consumes from `input_device.next().await`, verifying that stream faults do not
//!      cause memory leaks, unbounded heap growth, or parser hangs, and that expected
//!      [`InputEvent::Keyboard`] events arrive cleanly.
//!    - Sends phase synchronization and confirmation signals back to the test driver over
//!      [`stdout`].
//!
//! # Test Scenarios
//!
//! 1. **Runaway Payload Drain**: Controller streams an oversized [`OSC`] payload (120% of
//!    [`MAX_OSC_SEQUENCE_LENGTH`], > 1.25 MiB). In the child, the background [`mio`]
//!    polling thread trips [`OscCircuitBreaker`] into the `Open` state, drains the
//!    remaining stream without heap growth, resets on the string terminator (`BEL`), and
//!    cleanly delivers the subsequent marker keystroke (`'A'`).
//! 2. **Unrecognized [`OSC`] Sequences**: Controller emits an unknown [`OSC`] command
//!    (`OSC 9999`). Child verifies it is safely absorbed as
//!    [`VT100InputEventIR::Ignored`] without emitting phantom keystrokes, and cleanly
//!    delivers the subsequent marker keystroke (`'B'`).
//! 3. **Malformed Color Payloads**: Controller emits an unparsable color spec (`OSC
//!    10;rgb:invalid`). Child verifies graceful fallback to
//!    [`VT100InputEventIR::Ignored`] and delivery of subsequent marker keystroke (`'C'`).
//!
//! # Cross References
//!
//! - [`pty_osc_sender_test`]: Tests outbound [`OSC`] sequences emitted by [`OscSender`].
//! - [`pty_osc_color_test`]: Tests bidirectional color queries and synthesized responses.
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui --lib test_pty_osc_safety -- --nocapture
//! ```
//!
//! [`ChunkFramer`]: crate::core::ansi::vt_100_terminal_input_parser::ChunkFramer
//! [`DirectToAnsiInputDevice`]: crate::DirectToAnsiInputDevice
//! [`InputEvent::Keyboard`]: crate::InputEvent::Keyboard
//! [`iTerm2`]: https://iterm2.com/
//! [`Kitty`]: https://sw.kovidgoyal.net/kitty/
//! [`MAX_OSC_SEQUENCE_LENGTH`]: crate::MAX_OSC_SEQUENCE_LENGTH
//! [`mio`]: mio
//! [`OSC`]: crate::core::ansi::osc::OscSequence
//! [`OscCircuitBreaker`]: crate::core::ansi::vt_100_terminal_input_parser::chunk_framer::OscCircuitBreaker
//! [`OscSender`]: crate::OscSender
//! [`pty_osc_color_test`]: mod@super::pty_osc_color_test
//! [`pty_osc_sender_test`]: mod@super::pty_osc_sender_test
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`Sans-IO`]: https://sans-io.readthedocs.io/
//! [`stdin`]: std::io::stdin
//! [`stdout`]: std::io::stdout
//! [`VT100InputEventIR::Ignored`]: crate::core::ansi::vt_100_terminal_input_parser::VT100InputEventIR::Ignored
//! [`WezTerm`]: https://wezfurlong.org/wezterm/
//! [process roles documentation]: mod@super#architecture-and-process-roles

use crate::{GLYPH_CONTROLLED, GLYPH_CONTROLLER, GLYPH_CONTROLLER_CLEANUP, GLYPH_SUCCESS,
            GLYPH_WAITING, InputEvent, Key, KeyPress, MAX_OSC_SEQUENCE_LENGTH,
            MSG_CONTROLLED_READY, MSG_CONTROLLED_STARTING, MSG_TEST_RUNNING,
            PtyTestContext, PtyTestMode,
            core::ansi::{constants::{OSC_CODE_COLOR_REPORT_BACKGROUND,
                                     OSC_CODE_COLOR_REPORT_FOREGROUND,
                                     OSC_COLOR_SPEC_RGB_PREFIX, OSC_DELIMITER,
                                     OSC_DELIMITER_BYTES, OSC_START, OSC_START_BYTES,
                                     OSC_TERMINATOR_BEL, OSC_TERMINATOR_BEL_BYTES},
                         generator::generate_keyboard_sequence,
                         vt_100_terminal_input_parser::ir_event_types::{VT100InputEventIR,
                                                                        VT100KeyCodeIR,
                                                                        VT100KeyModifiersIR}},
            generate_pty_test,
            tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice};
use std::{io::Write, time::Duration};

generate_pty_test! {
    test_fn: test_pty_osc_safety,
    controller: controller,
    controlled: controlled,
    mode: PtyTestMode::Raw,
}

/// [`PTY`] Controller / Test Driver (Fault Injector): Injects runaway and malformed
/// [`OSC`] streams into controller [`PTY`].
///
/// Simulates buggy or rogue terminal behavior by injecting oversized runaway streams,
/// unrecognized command codes, and malformed color payloads, followed by valid marker
/// keystrokes to prove pipeline health.
///
/// [`OSC`]: crate::osc_codes::OscSequence
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
fn controller(context: PtyTestContext) {
    let PtyTestContext {
        pty_pair,
        child,
        mut buf_reader,
        mut writer,
    } = context;

    eprintln!("{GLYPH_CONTROLLER} PTY Controller: Starting OSC safety test...");
    eprintln!(
        "{GLYPH_WAITING} PTY Controller: Waiting for controlled process to start..."
    );

    // Wait for controlled to confirm it's running and ready.
    child
        .wait_for_ready(&mut buf_reader, MSG_CONTROLLED_READY)
        .expect("Failed to wait for ready signal");

    for test_case in generate_test_cases() {
        let desc = test_case.description;
        let ready_signal = test_case.ready_signal;
        let payload = &test_case.payload;
        let expected_conf = test_case.expected_confirmation;

        eprintln!("{GLYPH_WAITING} Controller: Waiting for {desc} readiness...");
        child.read_line_state(&mut buf_reader, |line| line == ready_signal);

        eprintln!(
            "  → Controller sending payload for {desc} ({} bytes)...",
            payload.len()
        );
        writer
            .write_all(payload)
            .expect("Failed to write payload to PTY");
        writer.flush().expect("Failed to flush payload");

        let conf = child
            .read_line_state(&mut buf_reader, |line| line.starts_with(expected_conf));
        eprintln!("  {GLYPH_SUCCESS} {desc} passed: {conf}");
    }

    eprintln!("{GLYPH_CONTROLLER_CLEANUP} PTY Controller: Cleaning up...");
    drop(writer);
    child.drain_and_wait(buf_reader, pty_pair);

    eprintln!("{GLYPH_SUCCESS} PTY Controller: Test passed!");
}

/// [`PTY`] Controlled / System Under Test (TUI Input Engine): Ingests streamed bytes via
/// [`DirectToAnsiInputDevice`] and verifies events.
///
/// Runs on the controlled side of the [`PTY`], exercising [`ChunkFramer`] and
/// [`OscCircuitBreaker`] to safely absorb faults while verifying clean delivery of
/// marker keystrokes.
///
/// [`ChunkFramer`]: crate::core::ansi::vt_100_terminal_input_parser::ChunkFramer
/// [`DirectToAnsiInputDevice`]: crate::DirectToAnsiInputDevice
/// [`OscCircuitBreaker`]: crate::core::ansi::vt_100_terminal_input_parser::chunk_framer::OscCircuitBreaker
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

        let timeout = Duration::from_secs(10);

        for test_case in generate_test_cases() {
            println!("{}", test_case.ready_signal);
            std::io::stdout()
                .flush()
                .expect("Failed to flush readiness");

            let event = tokio::time::timeout(timeout, input_device.next())
                .await
                .unwrap_or_else(|_| {
                    panic!(
                        "Timed out waiting for marker keystroke after {}",
                        test_case.description
                    )
                })
                .expect("Input device stream ended unexpectedly");

            match event {
                InputEvent::Keyboard(KeyPress::Plain {
                    key: Key::Character(ch),
                }) if ch == test_case.marker_char => {
                    println!("{}", test_case.expected_confirmation);
                    std::io::stdout()
                        .flush()
                        .expect("Failed to flush confirmation");
                }
                other => panic!(
                    "Expected Keyboard('{expected}') after {desc}, got: {other:?}",
                    expected = test_case.marker_char,
                    desc = test_case.description
                ),
            }
        }

        eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Completed successfully");
    });
}

// -------------------------------------------------------------------------------------------------
// Test cases
// -------------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct TestCase {
    pub description: &'static str,
    pub ready_signal: &'static str,
    pub payload: Vec<u8>,
    pub expected_confirmation: &'static str,
    pub marker_char: char,
}

impl TestCase {
    pub fn new(
        description: &'static str,
        ready_signal: &'static str,
        payload: Vec<u8>,
        expected_confirmation: &'static str,
        marker_char: char,
    ) -> Self {
        Self {
            description,
            ready_signal,
            payload,
            expected_confirmation,
            marker_char,
        }
    }
}

fn generate_test_cases() -> Vec<TestCase> {
    let marker_a = payload::marker_keystroke(constants::MARKER_CHAR_RUNAWAY);
    let marker_b = payload::marker_keystroke(constants::MARKER_CHAR_UNRECOGNIZED);
    let marker_c = payload::marker_keystroke(constants::MARKER_CHAR_MALFORMED);

    vec![
        // Test 1: Runaway Payload Drain (> 1 MiB).
        TestCase::new(
            "Test 1 (Runaway)",
            constants::READY_FOR_RUNAWAY,
            payload::runaway(&marker_a),
            constants::CONFIRMED_RUNAWAY_DRAINED,
            constants::MARKER_CHAR_RUNAWAY,
        ),
        // Test 2: Unrecognized OSC Sequence (`OSC 9999`).
        TestCase::new(
            "Test 2 (Unrecognized)",
            constants::READY_FOR_UNRECOGNIZED,
            payload::unrecognized(&marker_b),
            constants::CONFIRMED_UNRECOGNIZED_ABSORBED,
            constants::MARKER_CHAR_UNRECOGNIZED,
        ),
        // Test 3: Malformed Color Spec (`OSC 10;rgb:bad-spec`).
        TestCase::new(
            "Test 3 (Malformed)",
            constants::READY_FOR_MALFORMED,
            payload::malformed_color(&marker_c),
            constants::CONFIRMED_MALFORMED_ABSORBED,
            constants::MARKER_CHAR_MALFORMED,
        ),
    ]
}

mod constants {
    use super::*;

    /// Oversized payload length: 120% of [`MAX_OSC_SEQUENCE_LENGTH`] (1 MiB), ensuring
    /// the circuit breaker threshold is clearly exceeded.
    pub const RUNAWAY_PAYLOAD_LEN: usize = (MAX_OSC_SEQUENCE_LENGTH * 120) / 100;

    // IPC Handshake signals between controlled child and controller parent.
    pub const READY_FOR_RUNAWAY: &str = "READY_FOR_RUNAWAY";
    pub const CONFIRMED_RUNAWAY_DRAINED: &str = "CONFIRMED:RunawayDrained";

    pub const READY_FOR_UNRECOGNIZED: &str = "READY_FOR_UNRECOGNIZED";
    pub const CONFIRMED_UNRECOGNIZED_ABSORBED: &str = "CONFIRMED:UnrecognizedAbsorbed";

    pub const READY_FOR_MALFORMED: &str = "READY_FOR_MALFORMED";
    pub const CONFIRMED_MALFORMED_ABSORBED: &str = "CONFIRMED:MalformedAbsorbed";

    // Test payload content.
    pub const RUNAWAY_FILL_BYTE: u8 = b'a';
    pub const UNRECOGNIZED_CODE: &str = "9999";
    pub const UNRECOGNIZED_PAYLOAD: &str = "arbitrary_unrecognized_payload";
    pub const MALFORMED_COLOR_SPEC: &str = "not-valid-hex/format";

    // Canary marker characters used to verify stream health following fault injection.
    pub const MARKER_CHAR_RUNAWAY: char = 'A';
    pub const MARKER_CHAR_UNRECOGNIZED: char = 'B';
    pub const MARKER_CHAR_MALFORMED: char = 'C';
}

/// Synthesized [`OSC`] attack and stress payloads injected by the controller.
///
/// Composes raw byte vectors from foundational Tier 1 constants without magic
/// strings or raw literals.
///
/// [`OSC`]: crate::core::ansi::osc::OscSequence
mod payload {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Generates the raw [`ANSI`] keyboard byte sequence for a given character marker.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    pub fn marker_keystroke(ch: char) -> Vec<u8> {
        generate_keyboard_sequence(&VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Char(ch),
            modifiers: VT100KeyModifiersIR::NONE,
        })
        .expect("Failed to generate marker keystroke")
    }

    /// Generates an oversized [`OSC`] payload exceeding [`MAX_OSC_SEQUENCE_LENGTH`].
    ///
    /// # Payload Structure
    ///
    /// Constructs a sequence formatted as:
    /// `ESC ] 11 ;` + `a` * [`constants::RUNAWAY_PAYLOAD_LEN`] + `BEL` + `marker`
    ///
    /// - **Header**: [`OSC_START_BYTES`] (`ESC ]`) + [`OSC_CODE_COLOR_REPORT_BACKGROUND`]
    ///   (`11`) + [`OSC_DELIMITER_BYTES`] (`;`).
    /// - **Runaway Body**: Resizes the buffer with [`constants::RUNAWAY_PAYLOAD_LEN`]
    ///   bytes of [`constants::RUNAWAY_FILL_BYTE`] (`b'a'`), totaling `1,258,291` bytes
    ///   (120% of [`MAX_OSC_SEQUENCE_LENGTH`]), which exceeds the 1 MiB threshold by 20%.
    /// - **Terminator**: [`OSC_TERMINATOR_BEL_BYTES`] (`\x07`), signaling end of stream
    ///   to trigger [`OscCircuitBreaker`] reset.
    /// - **Marker Keystroke**: Appends the provided keystroke byte sequence to verify
    ///   that the input stream resumed normal event delivery immediately following the
    ///   drained sequence.
    ///
    /// [`constants::RUNAWAY_FILL_BYTE`]: super::constants::RUNAWAY_FILL_BYTE
    /// [`constants::RUNAWAY_PAYLOAD_LEN`]: super::constants::RUNAWAY_PAYLOAD_LEN
    /// [`MAX_OSC_SEQUENCE_LENGTH`]: crate::MAX_OSC_SEQUENCE_LENGTH
    /// [`OSC`]: crate::osc_codes::OscSequence
    /// [`OscCircuitBreaker`]: crate::core::ansi::vt_100_terminal_input_parser::chunk_framer::OscCircuitBreaker
    pub fn runaway(marker: &[u8]) -> Vec<u8> {
        let mut data = Vec::with_capacity(
            OSC_START_BYTES.len()
                + OSC_CODE_COLOR_REPORT_BACKGROUND.len()
                + OSC_DELIMITER_BYTES.len()
                + constants::RUNAWAY_PAYLOAD_LEN
                + OSC_TERMINATOR_BEL_BYTES.len()
                + marker.len(),
        );
        data.extend_from_slice(OSC_START_BYTES);
        data.extend_from_slice(OSC_CODE_COLOR_REPORT_BACKGROUND.as_bytes());
        data.extend_from_slice(OSC_DELIMITER_BYTES);
        // NOTE: this will exceed `MAX_OSC_SEQUENCE_LENGTH`
        data.resize(
            data.len() + constants::RUNAWAY_PAYLOAD_LEN,
            constants::RUNAWAY_FILL_BYTE,
        );
        data.extend_from_slice(OSC_TERMINATOR_BEL_BYTES);
        data.extend_from_slice(marker);
        data
    }

    /// Generates an unrecognized [`OSC`] command sequence with an attached marker.
    ///
    /// # Payload Structure
    ///
    /// Constructs `ESC ] 9999 ; arbitrary_unrecognized_payload BEL` + `marker`:
    /// - Composed via [`OSC_START`], [`constants::UNRECOGNIZED_CODE`], [`OSC_DELIMITER`],
    ///   [`constants::UNRECOGNIZED_PAYLOAD`], and [`OSC_TERMINATOR_BEL`].
    /// - Appends the marker keystroke bytes immediately following the `BEL` terminator.
    /// - Validates that the input framer safely discards unrecognized command codes
    ///   without emitting phantom keypresses or corrupting subsequent input events.
    ///
    /// [`constants::UNRECOGNIZED_CODE`]: super::constants::UNRECOGNIZED_CODE
    /// [`constants::UNRECOGNIZED_PAYLOAD`]: super::constants::UNRECOGNIZED_PAYLOAD
    /// [`OSC`]: crate::core::ansi::osc::OscSequence
    pub fn unrecognized(marker: &[u8]) -> Vec<u8> {
        let mut data = format!(
            "{OSC_START}{code}{OSC_DELIMITER}{payload}{OSC_TERMINATOR_BEL}",
            code = constants::UNRECOGNIZED_CODE,
            payload = constants::UNRECOGNIZED_PAYLOAD,
        )
        .into_bytes();
        data.extend_from_slice(marker);
        data
    }

    /// Generates a syntactically malformed color specification payload with marker.
    ///
    /// # Payload Structure
    ///
    /// Constructs `ESC ] 10 ; rgb:not-valid-hex/format BEL` + `marker`:
    /// - Composed via [`OSC_START`], [`OSC_CODE_COLOR_REPORT_FOREGROUND`],
    ///   [`OSC_DELIMITER`], [`OSC_COLOR_SPEC_RGB_PREFIX`],
    ///   [`constants::MALFORMED_COLOR_SPEC`], and [`OSC_TERMINATOR_BEL`].
    /// - Appends the marker keystroke bytes immediately following the `BEL` terminator.
    /// - Validates that unparsable color specifications fail gracefully, drop safely into
    ///   ignored events, and do not panic or stall the input loop.
    ///
    /// [`constants::MALFORMED_COLOR_SPEC`]: super::constants::MALFORMED_COLOR_SPEC
    /// [`OSC`]: crate::core::ansi::osc::OscSequence
    pub fn malformed_color(marker: &[u8]) -> Vec<u8> {
        let mut data = format!(
            "{OSC_START}{code}{OSC_DELIMITER}{prefix}{spec}{OSC_TERMINATOR_BEL}",
            code = OSC_CODE_COLOR_REPORT_FOREGROUND,
            prefix = OSC_COLOR_SPEC_RGB_PREFIX,
            spec = constants::MALFORMED_COLOR_SPEC,
        )
        .into_bytes();
        data.extend_from_slice(marker);
        data
    }
}
