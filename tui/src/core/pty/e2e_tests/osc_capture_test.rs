// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::cross_platform_commands;
use crate::{DefaultPtySessionConfig, OscEvent, PtyOutputEvent, PtySessionConfigOption,
            osc_codes::OscSequence};

#[tokio::test]
async fn test_osc_capture_enabled() {
    let osc_seq = OscSequence::ProgressUpdate(75).to_string();
    let mut session = cross_platform_commands::printf(&osc_seq)
        .with_config(
            DefaultPtySessionConfig
                + PtySessionConfigOption::CaptureOsc
                + PtySessionConfigOption::NoCaptureOutput,
        )
        .start()
        .expect("Failed to spawn read-only session");

    // 1. Wait for completion.
    let _status = (&mut session.orchestrator_task_handle)
        .await
        .expect("Join error")
        .expect("Session error");

    // 2. Drain channel.
    // All events are already in the channel buffer — the completion handle
    // joins the reader task and sends Exit before returning.
    let mut osc_received = false;
    while let Ok(event) = session.rx_output_event.try_recv() {
        if let PtyOutputEvent::Osc(OscEvent::ProgressUpdate(75)) = event {
            osc_received = true;
        }
    }

    assert!(osc_received);
}

#[tokio::test]
async fn test_osc_capture_disabled() {
    let osc_seq = OscSequence::ProgressUpdate(75).to_string();
    let mut session = cross_platform_commands::printf(&osc_seq)
        .with_config(DefaultPtySessionConfig + PtySessionConfigOption::NoCaptureOutput)
        .start()
        .expect("Failed to spawn read-only session");

    // 1. Wait for completion.
    let _status = (&mut session.orchestrator_task_handle)
        .await
        .expect("Join error")
        .expect("Session error");

    // 2. Drain channel.
    // All events are already in the channel buffer — the completion handle
    // joins the reader task and sends Exit before returning.
    let mut osc_received = false;
    while let Ok(event) = session.rx_output_event.try_recv() {
        if let PtyOutputEvent::Osc(_) = event {
            osc_received = true;
        }
    }

    assert!(!osc_received);
}
