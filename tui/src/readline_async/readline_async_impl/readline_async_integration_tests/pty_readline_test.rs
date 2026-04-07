// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`PTY`]-based integration tests for [`Readline`].
//!
//! # Run with:
//!
//! ```bash
//! # Run all tests in this file:
//! cargo test -p r3bl_tui pty_readline_test -- --nocapture
//!
//! # Run specific tests:
//! cargo test -p r3bl_tui test_pty_readline -- --nocapture
//! cargo test -p r3bl_tui test_pty_pause_resume -- --nocapture
//! cargo test -p r3bl_tui test_pty_pause_resume_with_output -- --nocapture
//! ```
//!
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal

use crate::{ChannelCapacity, LineStateControlSignal, MSG_CONTROLLED_READY,
            MSG_CONTROLLED_STARTING, MSG_SUCCESS, OutputDevice, PtyTestContext, Readline,
            Size, generate_pty_test, height, width};
use std::io::Write;
use tokio::{sync::broadcast, time::Duration};

// --- test_pty_readline ---

mod test_pty_readline {
    use super::*;

    generate_pty_test! {
        test_fn: test_pty_readline,
        controller: controller,
        controlled: controlled,
        mode: crate::PtyTestMode::Raw,
    }

    fn controller(context: PtyTestContext) {
        let PtyTestContext {
            pty_pair,
            child,
            mut buf_reader,
            mut writer,
        } = context;

        child
            .wait_for_ready(&mut buf_reader, MSG_CONTROLLED_READY)
            .unwrap();

        // Give readline time to start its event loop.
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Send "abc" followed by Enter (\r in raw mode).
        writer.write_all(b"abc\r").unwrap();
        writer.flush().unwrap();

        let result = child.read_until_marker(&mut buf_reader, MSG_SUCCESS, |line| {
            line.contains("ReadlineEvent:")
        });

        assert!(
            result.found_marker,
            "Controlled process did not print SUCCESS"
        );
        assert!(
            result.lines.iter().any(|l| l.contains("Line(\"abc\")")),
            "Expected Line(\"abc\") in output, got: {:?}",
            result.lines
        );

        child.drain_and_wait(buf_reader, pty_pair);
    }

    /// The harness performs [`std::process::exit(0)`] after this function returns.
    fn controlled() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            println!("{MSG_CONTROLLED_STARTING}");
            let output_device = OutputDevice::new_stdout();
            let input_device = crate::InputDevice::new();
            let (shutdown_sender, _) = broadcast::channel::<()>(1);
            let test_size = Size::new((width(100), height(100)));

            println!("{MSG_CONTROLLED_READY}");
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

            println!("{MSG_SUCCESS}");
            std::io::stdout().flush().unwrap();
        });
    }
}

// --- test_pty_pause_resume ---

mod test_pty_pause_resume {
    use super::*;

    generate_pty_test! {
        test_fn: test_pty_pause_resume,
        controller: controller,
        controlled: controlled,
        mode: crate::PtyTestMode::Cooked,
    }

    fn controller(context: PtyTestContext) {
        let PtyTestContext {
            pty_pair,
            child,
            mut buf_reader,
            ..
        } = context;

        child
            .wait_for_ready(&mut buf_reader, MSG_CONTROLLED_READY)
            .unwrap();

        let result = child.read_until_marker(&mut buf_reader, MSG_SUCCESS, |line| {
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

    /// The harness performs [`std::process::exit(0)`] after this function returns.
    fn controlled() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            println!("{MSG_CONTROLLED_STARTING}");
            let output_device = OutputDevice::new_stdout();
            let input_device = crate::InputDevice::new();
            let (shutdown_sender, _) = broadcast::channel::<()>(1);
            let test_size = Size::new((width(100), height(100)));

            println!("{MSG_CONTROLLED_READY}");
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

            println!("{MSG_SUCCESS}");
            std::io::stdout().flush().unwrap();
        });
    }
}

// --- test_pty_pause_resume_with_output ---

mod test_pty_pause_resume_with_output {
    use super::*;

    generate_pty_test! {
        test_fn: test_pty_pause_resume_with_output,
        controller: controller,
        controlled: controlled,
        mode: crate::PtyTestMode::Cooked,
    }

    fn controller(context: PtyTestContext) {
        let PtyTestContext {
            pty_pair,
            child,
            mut buf_reader,
            ..
        } = context;

        child
            .wait_for_ready(&mut buf_reader, MSG_CONTROLLED_READY)
            .unwrap();

        let result = child.read_until_marker(&mut buf_reader, MSG_SUCCESS, |line| {
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

    /// The harness performs [`std::process::exit(0)`] after this function returns.
    fn controlled() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            println!("{MSG_CONTROLLED_STARTING}");
            let output_device = OutputDevice::new_stdout();
            let input_device = crate::InputDevice::new();
            let (shutdown_sender, _) = broadcast::channel::<()>(1);
            let test_size = Size::new((width(100), height(100)));

            println!("{MSG_CONTROLLED_READY}");
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

            println!("{MSG_SUCCESS}");
            std::io::stdout().flush().unwrap();
        });
    }
}
