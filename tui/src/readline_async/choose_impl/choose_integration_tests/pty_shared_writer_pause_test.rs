// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`PTY`]-based integration test verifying that [`choose()`] correctly sends [`Pause`]
//! and [`Resume`] signals to the [`SharedWriter`].
//!
//! The controlled process runs [`choose()`] with real I/O devices in a real [`PTY`],
//! collects the [`LineStateControlSignal`]s from the [`SharedWriter`], and prints them to
//! [`stdout`]. The controller sends keystrokes via the [`PTY`] writer, reads the signal
//! output, and asserts correctness.
//!
//! [`choose()`] handles switching in and out of [raw mode] on its own, which is why this
//! test is run in [`PtyTestMode::Cooked`].
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_shared_writer_pause_works -- --nocapture
//! ```
//!
//! [`choose()`]: crate::choose
//! [`LineStateControlSignal`]: crate::LineStateControlSignal
//! [`Pause`]: crate::LineStateControlSignal::Pause
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`Resume`]: crate::LineStateControlSignal::Resume
//! [`SharedWriter`]: crate::SharedWriter
//! [`stdout`]: std::io::stdout
//! [raw mode]: mod@crate::terminal_raw_mode#raw-mode-vs-cooked-mode

use crate::{DefaultIoDevices, Header, MSG_CONTROLLED_READY, MSG_LINE_PREFIX,
            MSG_SUCCESS, PtyTestContext, PtyTestMode, SharedWriter,
            TuiAvailabilityChooseExt, choose, generate_keyboard_sequence,
            generate_pty_test,
            vt_100_terminal_input_parser::{VT100InputEventIR, VT100KeyCodeIR,
                                           VT100KeyModifiersIR}};
use std::io::Write;

generate_pty_test! {
    test_fn: test_shared_writer_pause_works,
    controller: controller,
    controlled: controlled,
    mode: PtyTestMode::Cooked,
}

/// Controller: sends keystrokes, reads signal output, asserts correctness.
///
/// Waits for the controlled process to signal readiness, sends key sequences
/// via [`generate_keyboard_sequence()`], then verifies [`Pause`] and [`Resume`]
/// signals were emitted.
///
/// [`generate_keyboard_sequence()`]: crate::generate_keyboard_sequence
/// [`Pause`]: crate::LineStateControlSignal::Pause
/// [`Resume`]: crate::LineStateControlSignal::Resume
fn controller(context: PtyTestContext) {
    let PtyTestContext {
        pty_pair,
        child,
        mut buf_reader,
        mut writer,
    } = context;

    // Wait for the controlled process to be ready.
    child
        .wait_for_ready(&mut buf_reader, MSG_CONTROLLED_READY)
        .unwrap();

    // Give choose() time to render and start its event loop.
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Send Down, Down, Enter to drive choose() to completion.
    for code in [
        VT100KeyCodeIR::Down,
        VT100KeyCodeIR::Down,
        VT100KeyCodeIR::Enter,
    ] {
        let bytes = generate_keyboard_sequence(&VT100InputEventIR::Keyboard {
            code,
            modifiers: VT100KeyModifiersIR::default(),
        })
        .unwrap();
        writer.write_all(&bytes).unwrap();
        writer.flush().unwrap();
    }

    // Read signal lines printed by the controlled process. Use `contains`
    // because choose()'s ANSI rendering may precede the signal on the same line.
    let result = child.read_until_marker(&mut buf_reader, MSG_SUCCESS, |line| {
        line.contains(MSG_LINE_PREFIX)
    });

    assert!(
        result.found_marker,
        "Controlled process did not print SUCCESS"
    );
    assert!(
        !result.lines.is_empty(),
        "No signals received from controlled process"
    );
    assert!(
        result.lines.first().unwrap().contains("Pause"),
        "First signal should be Pause, got: {}",
        result.lines.first().unwrap()
    );
    assert!(
        result.lines.last().unwrap().contains("Resume"),
        "Last signal should be Resume, got: {}",
        result.lines.last().unwrap()
    );

    child.drain_and_wait(buf_reader, pty_pair);
}

/// Controlled: runs [`choose()`] with real I/O, collects [`SharedWriter`] signals, prints
/// them to [`stdout`] for the controller to verify. The harness performs
/// [`std::process::exit(0)`] after this function returns.
///
/// [`choose()`]: crate::choose
/// [`SharedWriter`]: crate::SharedWriter
/// [`stdout`]: std::io::stdout
fn controlled() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let (mut line_receiver, shared_writer) = SharedWriter::new_mock();
        let mut io = DefaultIoDevices::default();

        // Signal readiness to the controller.
        println!("{MSG_CONTROLLED_READY}");
        std::io::stdout().flush().unwrap();

        // Run choose() with real I/O devices and the SharedWriter under test.
        let _unused = choose(
            Header::SingleLine("Choose:".into()),
            &["one", "two", "three"],
            None,
            None,
            crate::readline_async::HowToChoose::Single,
            crate::readline_async::StyleSheet::default(),
            (
                &mut io.output_device,
                &mut io.input_device,
                Some(shared_writer),
            ),
        )
        .get_first_result()
        .await;

        // Collect signals and print them for the controller.
        line_receiver.close();
        while let Some(signal) = line_receiver.recv().await {
            println!("{MSG_LINE_PREFIX}{signal:?}");
            std::io::stdout().flush().unwrap();
        }
        println!("{MSG_SUCCESS}");
        std::io::stdout().flush().unwrap();
    });
}
