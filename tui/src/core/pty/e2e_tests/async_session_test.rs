// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::cross_platform_commands;
use crate::{PtyInputEvent, PtyOutputEvent};
use std::time::Duration;

#[tokio::test]
async fn test_async_session_with_cat() {
    let mut session = cross_platform_commands::cat()
        .start_async()
        .expect("Failed to spawn async session");

    // 1. Send input.
    #[cfg(unix)]
    let test_data = b"hello async cat\n";
    #[cfg(windows)]
    let test_data = b"hello async cat\r\n";

    session
        .tx_input_event
        .send(PtyInputEvent::Write(test_data.to_vec()))
        .await
        .expect("Failed to send input");

    // 2. Send close.
    session
        .tx_input_event
        .send(PtyInputEvent::Close)
        .await
        .expect("Failed to send close");

    // 3. Drive session with tokio::select! until orchestrator completes.
    let mut captured_output = Vec::new();
    let mut completed = false;

    while !completed {
        tokio::select! {
            result = &mut session.orchestrator_task_handle => {
                let status = result
                    .expect("Join error")
                    .expect("Session error");
                assert!(status.success());
                completed = true;
            }
            Some(event) = session.rx_output_event.recv() => {
                if let PtyOutputEvent::Output(bytes) = event {
                    captured_output.extend_from_slice(&bytes);
                }
            }
        }
    }

    // 4. Drain any remaining output from the channel.
    while let Ok(event) = session.rx_output_event.try_recv() {
        if let PtyOutputEvent::Output(bytes) = event {
            captured_output.extend_from_slice(&bytes);
        }
    }

    let output_str = String::from_utf8_lossy(&captured_output);
    assert!(
        output_str.contains("hello async cat"),
        "Output did not contain 'hello async cat'. Actual output: {output_str}",
    );
}

#[tokio::test]
async fn test_async_session_kill_reporting() {
    let mut session = cross_platform_commands::sleep(10)
        .start_async()
        .expect("Failed to spawn async session");

    // Terminate the child process externally via the termination handle.
    session
        .child_process_termination_handle
        .kill()
        .expect("Failed to kill child process");

    // Await the orchestrator task handle.
    let status = session
        .orchestrator_task_handle
        .await
        .expect("Join error")
        .expect("Session error");
    assert!(!status.success());

    // Verify that the output channel receives an Exit event and closes.
    let mut exit_reported = false;
    while let Some(event) = session.rx_output_event.recv().await {
        match event {
            PtyOutputEvent::Exit(s) => {
                assert!(!s.success());
                exit_reported = true;
            }
            PtyOutputEvent::UnexpectedExit(_) => {
                exit_reported = true;
            }
            _ => {}
        }
    }
    assert!(exit_reported);
}

#[tokio::test]
async fn test_async_session_early_channel_drop() {
    let mut session = cross_platform_commands::cat()
        .start_async()
        .expect("Failed to spawn async session");

    #[cfg(unix)]
    let test_data = b"hello\n";
    #[cfg(windows)]
    let test_data = b"hello\r\n";

    // Send initial write to confirm process is active.
    session
        .tx_input_event
        .send(PtyInputEvent::Write(test_data.to_vec()))
        .await
        .expect("Failed to send input");

    // Read the first output event to confirm the output bridge is flowing.
    let _first_event = session.rx_output_event.recv().await;

    // Simulate consumer cancellation: drop both channel handles early.
    // The output bridge must terminate when blocking_send() fails.
    // The input bridge must terminate when blocking_recv() returns None.
    drop(session.rx_output_event);
    drop(session.tx_input_event);

    // Terminate the child so orchestrator can finish.
    session
        .child_process_termination_handle
        .kill()
        .expect("Failed to kill child process");

    // Await the orchestrator task handle with a timeout to verify no deadlock.
    let join_result =
        tokio::time::timeout(Duration::from_secs(5), session.orchestrator_task_handle)
            .await;

    assert!(
        join_result.is_ok(),
        "Orchestrator task timed out, possible deadlock in bridge cleanup"
    );
}
