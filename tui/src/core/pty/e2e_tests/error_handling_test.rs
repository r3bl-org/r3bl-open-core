// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::cross_platform_commands;
use crate::{DefaultPtySessionConfig, PtyOutputEvent, PtySessionConfigOption};

#[tokio::test]
async fn test_unexpected_exit_reporting() {
    let mut session = cross_platform_commands::sleep(10)
        .with_config(DefaultPtySessionConfig + PtySessionConfigOption::NoCaptureOutput)
        .start()
        .expect("Failed to spawn session");

    // Kill the process externally.
    session
        .child_process_termination_handle
        .kill()
        .expect("Failed to kill child");

    // 1. Wait for completion.
    let status = (&mut session.orchestrator_task_handle)
        .await
        .expect("Join error")
        .expect("Session error");
    assert!(!status.success());

    // 2. Drain channel.
    // All events are already in the channel buffer — the completion handle
    // joins the reader task and sends Exit before returning.
    let mut exit_reported = false;
    while let Ok(event) = session.rx_output_event.try_recv() {
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
