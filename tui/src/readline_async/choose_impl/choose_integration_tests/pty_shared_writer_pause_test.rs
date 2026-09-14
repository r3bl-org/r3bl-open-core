// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`PTY`]-based integration test verifying that [`ModalTerminalGuard`] correctly manages
//! terminal suspension and sends [`Flush`] when [`choose()`] completes.
//!
//! The controlled process runs [`choose()`] under [`ModalTerminalGuard`] with real I/O
//! devices in a real [`PTY`], collects the [`LineStateControlSignal`]s from the lease,
//! and prints them to [`stdout`]. The controller sends keystrokes via the [`PTY`] writer,
//! reads the signal output, and asserts correctness.
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
//! [`Flush`]: crate::LineStateControlSignal::Flush
//! [`LineStateControlSignal`]: crate::LineStateControlSignal
//! [`ModalTerminalGuard`]: crate::ModalTerminalGuard
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`stdout`]: std::io::stdout
//! [raw mode]: mod@crate::terminal_raw_mode#raw-mode-vs-cooked-mode

use crate::{ChannelCapacity, Header, InputDevice, MSG_CONTROLLED_READY, MSG_LINE_PREFIX,
            MSG_SUCCESS, ModalTerminalGuard, OutputDevice, PtyTestContext, PtyTestMode,
            Readline, TuiAvailabilityChooseExt, choose, generate_keyboard_sequence,
            generate_pty_test, vp_height, vp_width,
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
/// via [`generate_keyboard_sequence()`], then verifies [`Flush`] signal was emitted.
///
/// [`Flush`]: crate::LineStateControlSignal::Flush
/// [`generate_keyboard_sequence()`]: crate::generate_keyboard_sequence
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
        .expect("conversion error");

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
        .expect("conversion error");
        writer.write_all(&bytes).expect("conversion error");
        writer.flush().expect("conversion error");
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
        result
            .lines
            .first()
            .expect("conversion error")
            .contains("Flush"),
        "First signal should be Flush, got: {}",
        result.lines.first().expect("conversion error")
    );

    child.drain_and_wait(buf_reader, pty_pair);
}

/// Controlled: runs [`choose()`] under [`ModalTerminalGuard`] with real I/O, collects
/// [`LineStateControlSignal`]s, prints them to [`stdout`] for the controller to verify.
/// The harness performs [`std::process::exit(0)`] after this function returns.
///
/// [`choose()`]: crate::choose
/// [`LineStateControlSignal`]: crate::LineStateControlSignal
/// [`ModalTerminalGuard`]: crate::ModalTerminalGuard
/// [`stdout`]: std::io::stdout
fn controlled() {
    let rt = tokio::runtime::Runtime::new().expect("conversion error");
    rt.block_on(async {
        let (line_sender, mut line_receiver) =
            tokio::sync::mpsc::channel::<crate::LineStateControlSignal>(10);
        let output_device = OutputDevice::new_stdout();
        let input_device = InputDevice::default();
        let (shutdown_sender, _) = tokio::sync::broadcast::channel::<()>(1);
        let test_size = vp_width(80) + vp_height(24);

        let (mut readline, _) = Readline::try_new(
            "> ".into(),
            output_device,
            input_device,
            shutdown_sender,
            ChannelCapacity::Minimal,
            test_size,
        )
        .expect("conversion error");

        // Intercept line_control_sender with our receiver to observe signals.
        readline.line_control_sender = Some(line_sender);

        // Signal readiness to the controller.
        println!("{MSG_CONTROLLED_READY}");
        std::io::stdout().flush().expect("conversion error");

        // Run choose() with real I/O devices under ModalTerminalGuard.
        {
            let mut guard = ModalTerminalGuard::acquire(&mut readline);
            let _unused = choose(
                Header::SingleLine("Choose:".into()),
                &["one", "two", "three"],
                None,
                None,
                crate::readline_async::HowToChoose::Single,
                crate::readline_async::StyleSheet::default(),
                guard.as_mut_tuple(),
            )
            .get_first_result()
            .await;
        }

        // Collect signals and print them for the controller.
        if let Ok(signal) = line_receiver.try_recv() {
            println!("{MSG_LINE_PREFIX}{signal:?}");
            std::io::stdout().flush().expect("conversion error");
        }
        println!("{MSG_SUCCESS}");
        std::io::stdout().flush().expect("conversion error");
    });
}
