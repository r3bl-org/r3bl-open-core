// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::cross_platform_commands;
use crate::{DefaultPtySessionConfig, PtyInputEvent, PtyOutputEvent,
            PtySessionConfigOption, height, size, width};
use std::time::Duration;

#[tokio::test]
async fn test_pty_resize() {
    let mut session = cross_platform_commands::bash_or_cmd()
        .with_config(
            DefaultPtySessionConfig
                + PtySessionConfigOption::Size(size(width(80) + height(24))),
        )
        .start()
        .expect("Failed to spawn session");

    // 1. Resize immediately.
    let new_size = size(width(100) + height(50));
    session
        .tx_input_event
        .send(PtyInputEvent::Resize(new_size))
        .await
        .expect("Failed to send resize");

    // 2. Wait for the shell to be ready by sending a probe command and polling output
    //    until we see its response. This replaces a fixed sleep and is both faster
    //    (proceeds as soon as the shell is ready) and more reliable (adapts to slow CI
    //    environments).
    session
        .tx_input_event
        .send(PtyInputEvent::WriteLine("echo READY".to_string()))
        .await
        .expect("Failed to send readiness probe");

    let mut captured_output = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        match tokio::time::timeout_at(deadline, session.rx_output_event.recv()).await {
            Ok(Some(PtyOutputEvent::Output(bytes))) => {
                captured_output.extend_from_slice(&bytes);
                if String::from_utf8_lossy(&captured_output).contains("READY") {
                    break;
                }
            }
            Ok(Some(_)) => {} // Ignore non-output events.
            Ok(None) => panic!("Output channel closed before shell became ready"),
            Err(err) => panic!("Shell did not become ready within 5 seconds: {err}"),
        }
    }

    // 3. Check size and exit.
    #[cfg(unix)]
    let cmd = "stty size && exit";
    #[cfg(windows)]
    let cmd = "powershell.exe -NoProfile -Command \"$host.UI.RawUI.WindowSize; exit\"";

    session
        .tx_input_event
        .send(PtyInputEvent::WriteLine(cmd.to_string()))
        .await
        .expect("Failed to send input");

    // 3b. Close the input channel so the writer task can exit.
    // Without this, the writer task blocks on `blocking_recv()` waiting for more input,
    // while the session completion handler waits for the writer task - causing a
    // deadlock.
    session
        .tx_input_event
        .send(PtyInputEvent::Close)
        .await
        .expect("Failed to send close");

    // 4. Wait for completion.
    let _status = (&mut session.orchestrator_task_handle)
        .await
        .expect("Join error")
        .expect("Session error");

    // 5. Drain remaining output from the channel.
    while let Ok(event) = session.rx_output_event.try_recv() {
        if let PtyOutputEvent::Output(bytes) = event {
            captured_output.extend_from_slice(&bytes);
        }
    }

    let output_str = String::from_utf8_lossy(&captured_output);

    #[cfg(unix)]
    assert!(
        output_str.contains("50 100"),
        "Output did not contain '50 100'. Actual output: {output_str}",
    );

    #[cfg(windows)]
    // PowerShell output for WindowSize might look different, but should contain the new
    // dims.
    assert!(
        output_str.contains("100") && output_str.contains("50"),
        "Output did not contain '100' and '50'. Actual output: {output_str}",
    );
}
