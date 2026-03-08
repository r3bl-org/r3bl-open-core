// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::cross_platform_commands;
use crate::{PtyInputEvent, PtyOutputEvent};

#[tokio::test]
async fn test_session_with_cat() {
    let mut session = cross_platform_commands::cat()
        .start()
        .expect("Failed to spawn session");

    // 1. Send input.
    let test_data = b"hello cat\n";
    session
        .tx_input_event
        .try_send(PtyInputEvent::Write(test_data.to_vec()))
        .expect("Failed to send input");

    // 2. Send close.
    session
        .tx_input_event
        .try_send(PtyInputEvent::Close)
        .expect("Failed to send close");

    // 3. Wait for completion.
    let _status = (&mut session.orchestrator_task_handle)
        .await
        .expect("Join error")
        .expect("Session error");

    // 4. Drain the channel.
    // All events are already in the channel buffer — the completion handle
    // joins the reader task and sends Exit before returning.
    let mut captured_output = Vec::new();
    while let Ok(event) = session.rx_output_event.try_recv() {
        if let PtyOutputEvent::Output(bytes) = event {
            captured_output.extend_from_slice(&bytes);
        }
    }

    let output_str = String::from_utf8_lossy(&captured_output);
    assert!(
        output_str.contains("hello cat"),
        "Output did not contain 'hello cat'. Actual output: {output_str}"
    );
}
