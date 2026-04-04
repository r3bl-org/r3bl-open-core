// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`PTY`]-based integration tests for [`Readline`].
//!
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal

use crate::{CONTROLLED_READY, CONTROLLED_STARTING, ChannelCapacity,
            LineStateControlSignal, OutputDevice, PtyTestContext, Readline, SUCCESS,
            Size, generate_pty_test, height, width};
use std::io::Write;
use tokio::{sync::broadcast, time::Duration};

generate_pty_test! {
    test_fn: test_pty_readline,
    controller: controller_readline,
    controlled: controlled_readline,
    mode: crate::PtyTestMode::Raw,
}

fn controller_readline(context: PtyTestContext) {
    let PtyTestContext {
        pty_pair,
        child,
        mut buf_reader,
        mut writer,
    } = context;

    child
        .wait_for_ready(&mut buf_reader, CONTROLLED_READY)
        .unwrap();

    // Give readline time to start its event loop.
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Send "abc" followed by Enter (\r in raw mode).
    writer.write_all(b"abc\r").unwrap();
    writer.flush().unwrap();

    let result = child.read_until_marker(&mut buf_reader, SUCCESS, |line| {
        line.contains("ReadlineEvent:")
    });

    assert!(
        result.found_marker,
        "Controlled process did not print SUCCESS"
    );
    assert!(
        result
            .lines
            .iter()
            .any(|l| l.contains("Line(\"abc\")")),
        "Expected Line(\"abc\") in output, got: {:?}",
        result.lines
    );

    child.drain_and_wait(buf_reader, pty_pair);
}

fn controlled_readline() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        println!("{CONTROLLED_STARTING}");
        let output_device = OutputDevice::new_stdout();
        let input_device = crate::InputDevice::new();
        let (shutdown_sender, _) = broadcast::channel::<()>(1);
        let test_size = Size::new((width(100), height(100)));

        println!("{CONTROLLED_READY}");
        std::io::stdout().flush().unwrap();

        let (mut readline, _shared_writer) = Readline::try_new(
            "> ".into(),
            output_device,
            input_device,
            shutdown_sender,
            ChannelCapacity::Minimal,
            test_size,
        )
        .unwrap();

        let result = readline.readline().await;
        println!("ReadlineEvent: {result:?}");

        println!("{SUCCESS}");
        std::io::stdout().flush().unwrap();
    });
}

// --- test_pty_pause_resume ---

generate_pty_test! {
    test_fn: test_pty_pause_resume,
    controller: controller_pause_resume,
    controlled: controlled_pause_resume,
    mode: crate::PtyTestMode::Cooked,
}

fn controller_pause_resume(context: PtyTestContext) {
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
        line.contains("IsPaused: Paused") || line.contains("IsPaused: NotPaused")
    });

    assert!(
        result.found_marker,
        "Controlled process did not print SUCCESS"
    );
    assert!(
        result.lines.iter().any(|l| l.contains("IsPaused: Paused")),
        "Paused state not found in output"
    );
    assert!(
        result
            .lines
            .iter()
            .any(|l| l.contains("IsPaused: NotPaused")),
        "Live state not found in output"
    );

    child.drain_and_wait(buf_reader, pty_pair);
}

fn controlled_pause_resume() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        println!("{CONTROLLED_STARTING}");
        let output_device = OutputDevice::new_stdout();
        let input_device = crate::InputDevice::new();
        let (shutdown_sender, _) = broadcast::channel::<()>(1);
        let test_size = Size::new((width(100), height(100)));

        println!("{CONTROLLED_READY}");
        std::io::stdout().flush().unwrap();

        let (readline, shared_writer) = Readline::try_new(
            "> ".into(),
            output_device,
            input_device,
            shutdown_sender,
            ChannelCapacity::Minimal,
            test_size,
        )
        .unwrap();

        shared_writer
            .line_state_control_channel_sender
            .send(LineStateControlSignal::Pause)
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;

        let is_paused = readline.safe_line_state.lock().unwrap().is_paused;
        println!("IsPaused: {is_paused:?}");

        shared_writer
            .line_state_control_channel_sender
            .send(LineStateControlSignal::Resume)
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;

        let is_paused = readline.safe_line_state.lock().unwrap().is_paused;
        println!("IsPaused: {is_paused:?}");

        println!("{SUCCESS}");
        std::io::stdout().flush().unwrap();
    });
}

// --- test_pty_pause_resume_with_output ---

generate_pty_test! {
    test_fn: test_pty_pause_resume_with_output,
    controller: controller_pause_resume_with_output,
    controlled: controlled_pause_resume_with_output,
    mode: crate::PtyTestMode::Cooked,
}

fn controller_pause_resume_with_output(context: PtyTestContext) {
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
        line.contains("PauseBuffer: [\"abc\"]") || line.contains("IsPaused: NotPaused")
    });

    assert!(
        result.found_marker,
        "Controlled process did not print SUCCESS"
    );
    assert!(
        result
            .lines
            .iter()
            .any(|l| l.contains("PauseBuffer: [\"abc\"]")),
        "Pause buffer not found in output"
    );
    assert!(
        result
            .lines
            .iter()
            .any(|l| l.contains("IsPaused: NotPaused")),
        "Live state not found in output"
    );

    child.drain_and_wait(buf_reader, pty_pair);
}

fn controlled_pause_resume_with_output() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        println!("{CONTROLLED_STARTING}");
        let output_device = OutputDevice::new_stdout();
        let input_device = crate::InputDevice::new();
        let (shutdown_sender, _) = broadcast::channel::<()>(1);
        let test_size = Size::new((width(100), height(100)));

        println!("{CONTROLLED_READY}");
        std::io::stdout().flush().unwrap();

        let (readline, shared_writer) = Readline::try_new(
            "> ".into(),
            output_device,
            input_device,
            shutdown_sender,
            ChannelCapacity::Minimal,
            test_size,
        )
        .unwrap();

        shared_writer
            .line_state_control_channel_sender
            .send(LineStateControlSignal::Pause)
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;

        shared_writer
            .line_state_control_channel_sender
            .send(LineStateControlSignal::Line("abc".into()))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;

        let pause_buffer = readline.safe_is_paused_buffer.lock().unwrap().clone();
        println!("PauseBuffer: {pause_buffer:?}");

        shared_writer
            .line_state_control_channel_sender
            .send(LineStateControlSignal::Resume)
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;

        let is_paused = readline.safe_line_state.lock().unwrap().is_paused;
        println!("IsPaused: {is_paused:?}");

        println!("{SUCCESS}");
        std::io::stdout().flush().unwrap();
    });
}
