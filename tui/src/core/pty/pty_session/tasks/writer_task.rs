// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{Continuation, Controller, ControllerWriter, LINE_FEED_BYTE, PtyInputEvent,
            PtyOutputEvent};
use miette::miette;
use std::io::Write;
use tokio::sync::mpsc::{Receiver, Sender};

/// Spawns a blocking task that reads [`PtyInputEvent`]s from an [`bounded MPSC channel`]
/// channel and writes to the [`PTY`] controller.
///
/// This task runs on a dedicated blocking thread. It uses [`blocking_recv()`] to wait for
/// input events without spinning, ensuring efficient CPU usage.
///
/// # Backpressure and Stalling
///
/// 1. **Input Empty**: If the input channel is empty, this task stalls on
///    [`blocking_recv()`], waiting for the TUI to send a command.
/// 2. **Output Full**: If the output event channel is full, any error reporting via
///    [`blocking_send()`] will stall this task until the main event loop drains the
///    output queue.
///
/// [`blocking_recv()`]: tokio::sync::mpsc::Receiver::blocking_recv
/// [`blocking_send()`]: tokio::sync::mpsc::Sender::blocking_send
/// [`bounded MPSC channel`]: tokio::sync::mpsc::channel
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
#[must_use]
pub fn spawn_blocking_writer_task(
    controller: Controller,
    mut input_event_ch_rx_half: Receiver<PtyInputEvent>,
    output_event_ch_tx_half: Sender<PtyOutputEvent>,
) -> tokio::task::JoinHandle<miette::Result<()>> {
    tokio::task::spawn_blocking(move || -> miette::Result<()> {
        let mut writer = controller
            .take_writer()
            .map_err(|e| miette!("Failed to take PTY writer: {}", e))?;

        while let Some(input) = input_event_ch_rx_half.blocking_recv() {
            match writer_task_impl::handle_pty_input_event(
                input,
                &mut writer,
                &controller,
                &output_event_ch_tx_half,
            )? {
                Continuation::Continue => {}
                Continuation::Stop => break,
                Continuation::Restart => {
                    unreachable!("handle_pty_input_event never returns Restart")
                }
            }
        }
        Ok(())
    })
}

mod writer_task_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub fn handle_pty_input_event(
        input: PtyInputEvent,
        writer: &mut ControllerWriter,
        controller: &Controller,
        output_event_ch_tx_half: &Sender<PtyOutputEvent>,
    ) -> miette::Result<Continuation> {
        match input {
            PtyInputEvent::Write(bytes) => write_to_pty_with_flush(
                writer,
                &bytes,
                "Write failed",
                output_event_ch_tx_half,
            )?,
            PtyInputEvent::WriteLine(text) => {
                let mut data = text.into_bytes();
                data.push(LINE_FEED_BYTE);
                write_to_pty_with_flush(
                    writer,
                    &data,
                    "WriteLine failed",
                    output_event_ch_tx_half,
                )?;
            }
            PtyInputEvent::SendControl(ctrl, mode) => {
                let bytes = ctrl.to_bytes(mode);
                write_to_pty_with_flush(
                    writer,
                    &bytes,
                    "SendControl failed",
                    output_event_ch_tx_half,
                )?;
            }
            PtyInputEvent::Resize(size) => {
                controller.resize(size.into()).map_err(|e| {
                    let _unused = output_event_ch_tx_half.blocking_send(
                        PtyOutputEvent::WriteError(format!("Resize failed: {e}")),
                    );
                    miette!("Failed to resize PTY")
                })?;
            }
            PtyInputEvent::Flush => {
                writer.flush().map_err(|e| {
                    let _unused = output_event_ch_tx_half.blocking_send(
                        PtyOutputEvent::WriteError(format!("Flush failed: {e}")),
                    );
                    miette!("Failed to flush PTY")
                })?;
            }
            PtyInputEvent::Close => return Ok(Continuation::Stop),
        }
        Ok(Continuation::Continue)
    }

    pub fn write_to_pty_with_flush(
        writer: &mut ControllerWriter,
        data: &[u8],
        error_msg: &str,
        output_event_ch_tx_half: &Sender<PtyOutputEvent>,
    ) -> miette::Result<()> {
        writer.write_all(data).map_err(|e| {
            let _unused = output_event_ch_tx_half
                .blocking_send(PtyOutputEvent::WriteError(format!("Write failed: {e}")));
            miette!("{error_msg}")
        })?;
        writer.flush().map_err(|e| {
            let _unused = output_event_ch_tx_half
                .blocking_send(PtyOutputEvent::WriteError(format!("Flush failed: {e}")));
            miette!("{error_msg}")
        })?;
        Ok(())
    }
}
