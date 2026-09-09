// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{Continuation, Controller, ControllerWriter, LINE_FEED_BYTE, PtyInputEvent,
            PtyOutputEvent, ok};
use miette::miette;
use std::{io::Write,
          sync::{Arc, Mutex,
                 mpsc::{Receiver, SyncSender}},
          thread::JoinHandle};

/// Spawns a dedicated thread that reads [`PtyInputEvent`]s from a synchronous bounded
/// channel and writes to the [`PTY`] controller.
///
/// This thread runs with the name `pty-writer`. It waits for input events without
/// spinning, ensuring efficient CPU usage.
///
/// # Backpressure and Stalling
///
/// 1. **Input Empty**: If the input channel is empty, this thread stalls waiting for the
///    caller to send a command.
/// 2. **Output Full**: If the output event channel is full, any error reporting via
///    [`send()`] will stall this thread until the receiver drains the output queue.
///
/// # Errors
///
/// Returns an [`Err`] if spawning the OS thread fails.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`send()`]: std::sync::mpsc::SyncSender::send
pub fn spawn_pty_writer_thread(
    mut writer: ControllerWriter,
    controller: Arc<Mutex<Option<Controller>>>,
    input_event_ch_rx_half: Receiver<PtyInputEvent>,
    output_event_ch_tx_half: SyncSender<PtyOutputEvent>,
) -> miette::Result<JoinHandle<miette::Result<()>>> {
    std::thread::Builder::new()
        .name("pty-writer".into())
        .spawn(move || -> miette::Result<()> {
            while let Ok(input) = input_event_ch_rx_half.recv() {
                match impl_writer::handle_pty_input_event(
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
                    Continuation::ReturnError(()) => {
                        unimplemented!(
                            "The PTY writer loop does not currently produce error-carrying \
                             continuations. If this is reached, the framework's internal \
                             logic has been violated."
                        );
                    }
                }
            }
            ok!()
        })
        .map_err(|e| miette!("Failed to spawn pty-writer thread: {e}"))
}

mod impl_writer {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    #[cfg(target_os = "windows")]
    /// Windows console EOF byte sequence: `Ctrl+Z` (`SUB`, decimal 26, `1A` in hex)
    /// followed by carriage return (`b'\r'`) and line feed (`b'\n'`).
    ///
    /// Win32 console handles (`STD_INPUT_HANDLE`) do not deliver native pipe EOF
    /// semantics when the input pipe is closed. Interactive Windows console utilities
    /// (such as `findstr "^"`, `more`, or `copy con`) require `Ctrl+Z` followed by
    /// `Enter` to recognize end of file.
    const WIN_CONSOLE_EOF_BYTES: [u8; 3] = [26, b'\r', b'\n'];

    pub fn handle_pty_input_event(
        input: PtyInputEvent,
        writer: &mut ControllerWriter,
        controller: &Arc<Mutex<Option<Controller>>>,
        output_event_ch_tx_half: &SyncSender<PtyOutputEvent>,
    ) -> miette::Result<Continuation> {
        match input {
            PtyInputEvent::Write(bytes) => {
                write_to_pty_with_flush(
                    writer,
                    &bytes,
                    "Write failed",
                    output_event_ch_tx_half,
                )?;
            }
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
                if let Ok(guard) = controller.lock() {
                    let Some(controller) = guard.as_ref() else {
                        return Ok(Continuation::Continue);
                    };
                    controller.resize(size.into()).map_err(|e| {
                        let _unused = output_event_ch_tx_half.send(
                            PtyOutputEvent::WriteError(format!("Resize failed: {e}")),
                        );
                        miette!("Failed to resize PTY")
                    })?;
                }
            }
            PtyInputEvent::Flush => {
                writer.flush().map_err(|e| {
                    let _unused = output_event_ch_tx_half
                        .send(PtyOutputEvent::WriteError(format!("Flush failed: {e}")));
                    miette!("Failed to flush PTY")
                })?;
            }
            PtyInputEvent::Close => {
                let is_teardown = controller.lock().is_ok_and(|g| g.is_none());
                if is_teardown {
                    return Ok(Continuation::Stop);
                }

                #[cfg(target_os = "windows")]
                {
                    // Attempt to write Ctrl+Z to signal EOF to interactive console
                    // utilities. On Windows, do not stop or drop writer while the
                    // child is still running, as closing the input pipe causes ConPTY
                    // to abort child processes with STATUS_CONTROL_C_EXIT (0xC000013A).
                    let _unused = writer.write_all(&WIN_CONSOLE_EOF_BYTES);
                    let _unused = writer.flush();
                    return Ok(Continuation::Continue);
                }
                #[cfg(not(target_os = "windows"))]
                return Ok(Continuation::Stop);
            }
        }
        Ok(Continuation::Continue)
    }

    pub fn write_to_pty_with_flush(
        writer: &mut ControllerWriter,
        data: &[u8],
        error_msg: &str,
        output_event_ch_tx_half: &SyncSender<PtyOutputEvent>,
    ) -> miette::Result<()> {
        writer.write_all(data).map_err(|e| {
            let _unused = output_event_ch_tx_half
                .send(PtyOutputEvent::WriteError(format!("Write failed: {e}")));
            miette!("{error_msg}")
        })?;
        writer.flush().map_err(|e| {
            let _unused = output_event_ch_tx_half
                .send(PtyOutputEvent::WriteError(format!("Flush failed: {e}")));
            miette!("{error_msg}")
        })?;
        ok!()
    }
}

// cspell:words findstr
