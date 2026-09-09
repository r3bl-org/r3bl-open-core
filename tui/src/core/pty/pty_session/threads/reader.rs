// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{CaptureFlag, ControllerReader, OscBuffer, PtyOutputEvent, PtySessionConfig,
            READ_BUFFER_SIZE, ok};
use miette::miette;
use std::{io::Read, sync::mpsc::SyncSender, thread::JoinHandle};

/// Spawns a dedicated thread that reads [`PTY`] output and sends [`PtyOutputEvent`]s.
///
/// This function is the core engine for capturing terminal data. It runs on a dedicated
/// thread named `pty-reader` to ensure that heavy I/O operations don't block other
/// threads.
///
/// # Processing Engine
///
/// The reader thread performs three main functions on the incoming byte stream:
/// 1. **Capture Output**: Raw bytes are bundled into [`PtyOutputEvent::Output`] and sent.
/// 2. **[`OSC`] Detection**: Scans for [`OSC`] sequences (like terminal titles) if
///    enabled.
/// 3. **Cursor Detection**: Monitors for terminal mode changes if enabled.
///
/// # Backpressure
///
/// This thread implements Gate 2 of the flow control system. For the full system
/// architecture, wiring diagram, and how backpressure cascades to the child process, see
/// the [Backpressure Architecture].
///
/// # Errors
///
/// Returns an [`Err`] if spawning the OS thread fails.
///
/// [`OSC`]: crate::osc_codes::OscSequence
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [Backpressure Architecture]: crate::core::pty#backpressure-architecture
pub fn spawn_pty_reader_thread(
    mut reader: ControllerReader,
    output_event_ch_tx_half: SyncSender<PtyOutputEvent>,
    arg_config: impl Into<PtySessionConfig>,
) -> miette::Result<JoinHandle<miette::Result<()>>> {
    let config = arg_config.into();
    std::thread::Builder::new()
        .name("pty-reader".into())
        .spawn(move || -> miette::Result<()> {
            let mut buf = [0u8; READ_BUFFER_SIZE];
            let mut osc_buffer = OscBuffer::new();

            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break, // EOF.
                    Ok(n) => {
                        // 1. Process regular output.
                        if config.capture_output == CaptureFlag::Capture {
                            let bytes = buf[..n].to_vec();
                            let _unused = output_event_ch_tx_half
                                .send(PtyOutputEvent::Output(bytes));
                        }

                        // 2. Process OSC sequences if enabled.
                        if config.capture_osc == CaptureFlag::Capture {
                            let events = osc_buffer.append_and_extract(&buf, n);
                            for event in events {
                                let _unused = output_event_ch_tx_half
                                    .send(PtyOutputEvent::Osc(event));
                            }
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {
                        // Expected when the PTY output pipe is closed after child exit.
                        break;
                    }
                    Err(e) => {
                        // This error is expected when the PTY is closed (e.g., child
                        // process exits). Not a hard error for the caller.
                        let _unused =
                            output_event_ch_tx_half.send(PtyOutputEvent::WriteError(
                                format!("Read from PTY failed: {e}"),
                            ));
                        break;
                    }
                }
            }
            ok!()
        })
        .map_err(|e| miette!("Failed to spawn pty-reader thread: {e}"))
}
