// Copyright (c) 2025-2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`PTY`] integration test: Concurrent input and background output deadlock stress test.
//!
//! Validates that when background tasks continuously write multi-line messages to
//! [`SharedWriter`] while the user simultaneously types keystrokes, presses Enter, and
//! presses `Ctrl+C`, the strict lock hierarchy ([`SafeLineState`] -> [`OutputDevice`] ->
//! Leaf locks) completely eliminates lock-inversion deadlocks and preserves all input
//! events.
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_pty_concurrent_input_output_deadlock -- --nocapture
//! ```
//!
//! [`OutputDevice`]: crate::OutputDevice
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`SafeLineState`]: crate::SafeLineState
//! [`SharedWriter`]: crate::SharedWriter

use crate::{ChannelCapacity, InputDevice, MSG_CONTROLLED_READY, MSG_CONTROLLED_STARTING,
            MSG_SUCCESS, OutputDevice, PtyTestContext, PtyTestMode, Readline,
            generate_pty_test, vp_height, vp_width};
use std::io::Write;
use tokio::{sync::broadcast, time::Duration};

generate_pty_test! {
    test_fn: test_pty_concurrent_input_output_deadlock,
    controller: controller,
    controlled: controlled,
    mode: PtyTestMode::Raw,
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
        .expect("conversion error");

    // Give readline and background writers time to start.
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Concurrently send keystrokes and control characters while background output is
    // streaming:
    // 1. Send first line "concurrent_line_1\r".
    for byte in b"concurrent_line_1\r" {
        writer.write_all(&[*byte]).expect("conversion error");
        writer.flush().expect("conversion error");
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    // 2. Send second line "concurrent_line_2\r".
    for byte in b"concurrent_line_2\r" {
        writer.write_all(&[*byte]).expect("conversion error");
        writer.flush().expect("conversion error");
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    // 3. Send Ctrl+C (\x03) for interruption.
    writer.write_all(b"\x03").expect("conversion error");
    writer.flush().expect("conversion error");

    // Read until SUCCESS marker is reached.
    let result = child.read_until_marker(&mut buf_reader, MSG_SUCCESS, |line| {
        line.contains("EventReceived:")
    });

    assert!(
        result.found_marker,
        "Controlled process deadlocked or did not print SUCCESS"
    );

    assert!(
        result
            .lines
            .iter()
            .any(|l| l.contains("Line(\"concurrent_line_1\")")),
        "Expected Line(\"concurrent_line_1\") in output, got: {:?}",
        result.lines
    );

    assert!(
        result
            .lines
            .iter()
            .any(|l| l.contains("Line(\"concurrent_line_2\")")),
        "Expected Line(\"concurrent_line_2\") in output, got: {:?}",
        result.lines
    );

    assert!(
        result.lines.iter().any(|l| l.contains("Interrupted")),
        "Expected Interrupted (Ctrl+C) in output, got: {:?}",
        result.lines
    );

    child.drain_and_wait(buf_reader, pty_pair);
}

/// The harness performs [`std::process::exit(0)`] after this function returns.
fn controlled() {
    let rt = tokio::runtime::Runtime::new().expect("conversion error");
    rt.block_on(async {
        println!("{MSG_CONTROLLED_STARTING}");
        let output_device = OutputDevice::new_stdout();
        let input_device = InputDevice::new();
        let (shutdown_sender, _) = broadcast::channel::<()>(1);
        let test_size = vp_width(100) + vp_height(100);

        println!("{MSG_CONTROLLED_READY}");
        std::io::stdout().flush().expect("conversion error");

        let (mut readline, shared_writer) = Readline::try_new(
            "> ".into(),
            output_device,
            input_device,
            shutdown_sender,
            ChannelCapacity::Minimal,
            test_size,
        )
        .expect("conversion error");

        // Spawn multiple background tasks writing multiline blocks concurrently.
        let mut bg_handles = Vec::new();
        for task_id in 0..3 {
            let mut writer_clone = shared_writer.clone();
            bg_handles.push(tokio::spawn(async move {
                for i in 0..15 {
                    let _unused = writeln!(
                        writer_clone,
                        "[bg-{task_id}] Output chunk {i} line A\n[bg-{task_id}] Output chunk {i} line B"
                    );
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            }));
        }

        // Read 3 readline events (2 lines + 1 Ctrl+C).
        let mut count = 0;
        while count < 3 {
            match readline.readline().await {
                Ok(event) => {
                    println!("EventReceived: {event:?}");
                    std::io::stdout().flush().expect("conversion error");
                    count += 1;
                }
                Err(err) => {
                    println!("ErrorReceived: {err:?}");
                    std::io::stdout().flush().expect("conversion error");
                    break;
                }
            }
        }

        for handle in bg_handles {
            drop(handle.await);
        }

        println!("{MSG_SUCCESS}");
        std::io::stdout().flush().expect("conversion error");
    });
}
