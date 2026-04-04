// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`PTY`]-based integration tests for [`Spinner`].
//!
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal

use crate::{CONTROLLED_READY, DefaultIoDevices, LINE_PREFIX, PtyTestContext, SUCCESS,
            SharedWriter, Spinner, SpinnerColor, SpinnerStyle, SpinnerTemplate,
            TuiAvailability, generate_pty_test};
use std::{io::Write, time::Duration};

const QUANTUM: Duration = Duration::from_millis(100);
const FACTOR: u32 = 5;

// --- test_pty_spinner_color ---

generate_pty_test! {
    test_fn: test_pty_spinner_color,
    controller: controller_spinner_color,
    controlled: controlled_spinner_color,
    mode: crate::PtyTestMode::Cooked,
}

fn controller_spinner_color(context: PtyTestContext) {
    let PtyTestContext {
        pty_pair,
        child,
        mut buf_reader,
        ..
    } = context;

    child
        .wait_for_ready(&mut buf_reader, CONTROLLED_READY)
        .unwrap();

    let result = child.read_until_marker(&mut buf_reader, SUCCESS, |line| {
        line.contains("⠁ message") || line.contains("final message")
    });

    assert!(
        result.found_marker,
        "Controlled process did not print SUCCESS"
    );
    assert!(
        result.lines.iter().any(|l| l.contains("⠁ message")),
        "Spinner message not found in output"
    );
    assert!(
        result.lines.iter().any(|l| l.contains("final message")),
        "Final message not found in output"
    );

    child.drain_and_wait(buf_reader, pty_pair);
}

fn controlled_spinner_color() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let (mut line_receiver, shared_writer) = SharedWriter::new_mock();
        let io = DefaultIoDevices::default();

        println!("{CONTROLLED_READY}");
        std::io::stdout().flush().unwrap();

        let res_maybe_spinner = Spinner::try_start(
            "message",
            "final message",
            QUANTUM,
            SpinnerStyle {
                template: SpinnerTemplate::Braille,
                color: SpinnerColor::None,
            },
            io.output_device,
            Some(shared_writer),
        )
        .await;

        let TuiAvailability::Available(mut spinner) = res_maybe_spinner else {
            panic!("Spinner should be available")
        };

        tokio::time::sleep(QUANTUM * FACTOR).await;
        spinner.request_shutdown();
        spinner.await_shutdown().await;

        line_receiver.close();
        while let Some(signal) = line_receiver.recv().await {
            println!("{LINE_PREFIX}{signal:?}");
        }

        println!("{SUCCESS}");
        std::io::stdout().flush().unwrap();
    });
}

// --- test_pty_spinner_no_color ---

generate_pty_test! {
    test_fn: test_pty_spinner_no_color,
    controller: controller_spinner_no_color,
    controlled: controlled_spinner_no_color,
    mode: crate::PtyTestMode::Cooked,
}

fn controller_spinner_no_color(context: PtyTestContext) {
    let PtyTestContext {
        pty_pair,
        child,
        mut buf_reader,
        ..
    } = context;

    child
        .wait_for_ready(&mut buf_reader, CONTROLLED_READY)
        .unwrap();

    let result = child.read_until_marker(&mut buf_reader, SUCCESS, |line| {
        line.contains("⠁ message") || line.contains("final message")
    });

    assert!(
        result.found_marker,
        "Controlled process did not print SUCCESS"
    );
    assert!(
        result.lines.iter().any(|l| l.contains("⠁ message")),
        "Spinner message not found in output"
    );
    assert!(
        result.lines.iter().any(|l| l.contains("final message")),
        "Final message not found in output"
    );

    child.drain_and_wait(buf_reader, pty_pair);
}

fn controlled_spinner_no_color() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let (mut line_receiver, shared_writer) = SharedWriter::new_mock();
        let io = DefaultIoDevices::default();

        println!("{CONTROLLED_READY}");
        std::io::stdout().flush().unwrap();

        let res_maybe_spinner = Spinner::try_start(
            "message",
            "final message",
            QUANTUM,
            SpinnerStyle {
                template: SpinnerTemplate::Braille,
                color: SpinnerColor::None,
            },
            io.output_device,
            Some(shared_writer),
        )
        .await;

        let TuiAvailability::Available(mut spinner) = res_maybe_spinner else {
            panic!("Spinner should be available")
        };

        tokio::time::sleep(QUANTUM * FACTOR).await;
        spinner.request_shutdown();
        spinner.await_shutdown().await;

        line_receiver.close();
        while let Some(signal) = line_receiver.recv().await {
            println!("{LINE_PREFIX}{signal:?}");
        }

        println!("{SUCCESS}");
        std::io::stdout().flush().unwrap();
    });
}

// --- test_pty_spinner_message_update ---

generate_pty_test! {
    test_fn: test_pty_spinner_message_update,
    controller: controller_spinner_message_update,
    controlled: controlled_spinner_message_update,
    mode: crate::PtyTestMode::Cooked,
}

fn controller_spinner_message_update(context: PtyTestContext) {
    let PtyTestContext {
        pty_pair,
        child,
        mut buf_reader,
        ..
    } = context;

    child
        .wait_for_ready(&mut buf_reader, CONTROLLED_READY)
        .unwrap();

    let result = child.read_until_marker(&mut buf_reader, SUCCESS, |line| {
        line.contains("message") || line.contains("final message")
    });

    assert!(
        result.found_marker,
        "Controlled process did not print SUCCESS"
    );
    assert!(
        result.lines.iter().any(|l| l.contains("updated message")),
        "Updated message not found in output: {:?}",
        result.lines
    );
    assert!(
        result.lines.iter().any(|l| l.contains("final message")),
        "Final message not found in output"
    );

    child.drain_and_wait(buf_reader, pty_pair);
}

fn controlled_spinner_message_update() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let (mut line_receiver, shared_writer) = SharedWriter::new_mock();
        let io = DefaultIoDevices::default();

        println!("{CONTROLLED_READY}");
        std::io::stdout().flush().unwrap();

        let res_maybe_spinner = Spinner::try_start(
            "message",
            "final message",
            QUANTUM,
            SpinnerStyle {
                template: SpinnerTemplate::Braille,
                color: SpinnerColor::None,
            },
            io.output_device,
            Some(shared_writer),
        )
        .await;

        let TuiAvailability::Available(mut spinner) = res_maybe_spinner else {
            panic!("Spinner should be available")
        };

        tokio::time::sleep(QUANTUM * 2).await;
        spinner.update_message("updated message");
        tokio::time::sleep(QUANTUM * FACTOR).await;
        spinner.request_shutdown();
        spinner.await_shutdown().await;

        line_receiver.close();
        while let Some(signal) = line_receiver.recv().await {
            println!("{LINE_PREFIX}{signal:?}");
        }

        println!("{SUCCESS}");
        std::io::stdout().flush().unwrap();
    });
}
