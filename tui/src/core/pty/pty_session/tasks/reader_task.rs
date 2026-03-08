// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{CaptureFlag, ControllerReader, CursorModeDetector, DetectFlag, OscBuffer,
            PtyOutputEvent, PtySessionConfig, READ_BUFFER_SIZE};
use std::io::Read;
use tokio::sync::mpsc::Sender;

/// Spawns a blocking task that reads [`PTY`] output and sends [`PtyOutputEvent`]s.
///
/// This function is the core engine for capturing terminal data. It runs on a dedicated
/// blocking thread to ensure that heavy I/O operations don't block the async executor.
///
/// # Processing Engine
///
/// The reader task performs three main functions on the incoming byte stream:
/// 1. **Capture Output**: Raw bytes are bundled into [`PtyOutputEvent::Output`] and sent.
/// 2. **[`OSC`] Detection**: Scans for [`OSC`] sequences (like terminal titles) if
///    enabled.
/// 3. **Cursor Detection**: Monitors for terminal mode changes if enabled.
///
/// # Backpressure Mechanism
///
/// This task implements a robust backpressure chain to protect system resources:
/// 1. **Internal Buffer**: When the MPSC channel (sized by
///    [`DefaultSize::PtyChannelBufferSize`]) is full, [`blocking_send()`] will stall this
///    thread.
/// 2. **OS Pipe**: While this thread is stalled, it stops reading from the [`PTY`],
///    causing the OS-level pipe buffer to fill up.
/// 3. **Process Throttling**: Once the OS pipe is full, the kernel puts the child process
///    into a blocked state when it attempts to write more data.
///
/// This chain ensures the child process is throttled to match the consumption speed of
/// the main event loop, preventing memory exhaustion and "buffer bloat".
///
/// [`blocking_send()`]: tokio::sync::mpsc::Sender::blocking_send
/// [`DefaultSize::PtyChannelBufferSize`]: crate::DefaultSize::PtyChannelBufferSize
/// [`OSC`]: crate::osc_codes::OscSequence
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
#[must_use]
pub fn spawn_blocking_reader_task(
    mut reader: ControllerReader,
    output_event_ch_tx_half: Sender<PtyOutputEvent>,
    arg_config: impl Into<PtySessionConfig>,
) -> tokio::task::JoinHandle<miette::Result<()>> {
    let config = arg_config.into();
    tokio::task::spawn_blocking(move || -> miette::Result<()> {
        let mut buf = [0u8; READ_BUFFER_SIZE];
        let mut osc_buffer = OscBuffer::new();
        let mut cursor_detector = CursorModeDetector::new();

        loop {
            match reader.read(&mut buf) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    // 1. Process regular output.
                    if config.capture_output == CaptureFlag::Capture {
                        let bytes = buf[..n].to_vec();
                        let _unused = output_event_ch_tx_half
                            .blocking_send(PtyOutputEvent::Output(bytes));
                    }

                    // 2. Process OSC sequences if enabled.
                    if config.capture_osc == CaptureFlag::Capture {
                        let events = osc_buffer.append_and_extract(&buf, n);
                        for event in events {
                            let _unused = output_event_ch_tx_half
                                .blocking_send(PtyOutputEvent::Osc(event));
                        }
                    }

                    // 3. Detect cursor mode changes if enabled.
                    if config.detect_cursor_mode == DetectFlag::Detect
                        && let Some(mode) =
                            cursor_detector.scan_for_mode_change(&buf[..n])
                    {
                        let _unused = output_event_ch_tx_half
                            .blocking_send(PtyOutputEvent::CursorModeChange(mode));
                    }
                }
                Err(e) => {
                    // This error is expected when the PTY is closed (e.g., child
                    // process exits). Not a hard error for the caller.
                    let _unused = output_event_ch_tx_half.blocking_send(
                        PtyOutputEvent::WriteError(format!("Read from PTY failed: {e}")),
                    );
                    break;
                }
            }
        }
        Ok(())
    })
}
