// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{reader_task::spawn_blocking_reader_task,
            writer_task::spawn_blocking_writer_task};
use crate::{ControlledChild, Controller, PtyControlledChildExitStatus, PtyInputEvent,
            PtyOrchestratorHandle, PtyOutputEvent, PtySessionConfig};
use miette::miette;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{Receiver, Sender};

/// Spawns the **Orchestrator Task** for a [`PtySession`].
///
/// This task is the "Director" of the session. It:
/// 1. Takes the writer and reader from [`Controller`].
/// 2. Performs the Windows [`ConPTY`] initialization handshake (Windows only).
/// 3. Spawns the **Reader Task**.
/// 4. Spawns the **Writer Task**.
/// 5. Waits for the child process to exit.
/// 6. Destroys the pseudo-console controller ([`ClosePseudoConsole`] on Windows) to
///    unblock the reader.
/// 7. Joins both background tasks.
/// 8. Sends the final [`PtyOutputEvent::Exit`] event.
///
/// # Errors
///
/// Returns an [`Err`] if:
/// - Taking the writer from [`Controller`] fails.
/// - Cloning the reader from [`Controller`] fails.
/// - Performing the Windows [`ConPTY`] handshake fails (Windows only).
///
/// For the complete lifecycle architecture, see the [Session Layer] documentation.
///
/// [`ClosePseudoConsole`]:
///     https://learn.microsoft.com/en-us/windows/console/closepseudoconsole
/// [`ConPTY`]:
///     https://learn.microsoft.com/en-us/windows/console/creating-a-pseudoconsole-session
/// [`PtySession`]: crate::PtySession
/// [Session Layer]: mod@crate::pty_session
pub fn spawn_orchestrator_task(
    mut controlled_child: ControlledChild,
    controller: Controller,
    input_event_ch_tx_half: Sender<PtyInputEvent>,
    input_event_ch_rx_half: Receiver<PtyInputEvent>,
    output_event_ch_tx_half: Sender<PtyOutputEvent>,
    arg_config: impl Into<PtySessionConfig>,
) -> miette::Result<PtyOrchestratorHandle> {
    // Take writer.
    let controller_writer = controller
        .take_writer()
        .map_err(|e| miette!("Failed to take writer: {}", e))?;

    // Windows needs mut writer.
    #[cfg(target_os = "windows")]
    let mut controller_writer = controller_writer;

    // Take reader.
    let controller_reader = controller
        .try_clone_reader()
        .map_err(|e| miette!("Failed to clone reader: {}", e))?;

    // Windows needs reader handshake.
    #[cfg(target_os = "windows")]
    let controller_reader = impl_windows_conpty::perform_conpty_handshake(
        controller_reader,
        &mut controller_writer,
        &controlled_child,
    )?;

    let handle = tokio::spawn({
        let config = arg_config.into();
        let input_event_ch_tx_half_clone = input_event_ch_tx_half.clone();
        async move {
            let shared_controller = Arc::new(Mutex::new(Some(controller)));

            // Spawn background tasks.
            let output_reader_task_handle = spawn_blocking_reader_task(
                controller_reader,
                output_event_ch_tx_half.clone(),
                config,
            );

            let input_writer_task_handle = spawn_blocking_writer_task(
                controller_writer,
                shared_controller.clone(),
                input_event_ch_rx_half,
                output_event_ch_tx_half.clone(),
            );

            // Wait for the child process to exit.
            let status = tokio::task::spawn_blocking(move || controlled_child.wait())
                .await
                .map_err(|e| miette!("Wait task failed: {}", e))?
                .map_err(|e| miette!("Child process wait failed: {}", e))?;

            let status = PtyControlledChildExitStatus { inner: status };

            // Child process has terminated. Destroy the pseudo-console controller.
            // On Windows, MasterPty::drop invokes ClosePseudoConsole(), which closes
            // the ConPTY output pipe and delivers EOF (0 bytes or BrokenPipe) to the
            // reader task, allowing the reader task to exit cleanly.
            if let Ok(mut guard) = shared_controller.lock() {
                drop(guard.take());
            }

            // Send Close event to signal writer task to stop (if not already stopped).
            // We do this via the sender side (which we still have a clone of).
            let _unused = input_event_ch_tx_half_clone
                .send(PtyInputEvent::Close)
                .await;

            // Wait for background tasks to finish.
            drop(output_reader_task_handle.await);
            drop(input_writer_task_handle.await);

            // Send the exit event.
            let _unused = output_event_ch_tx_half
                .send(PtyOutputEvent::Exit(status.clone()))
                .await;

            Ok(status)
        }
    });

    Ok(handle)
}

#[cfg(target_os = "windows")]
mod impl_windows_conpty {
    use crate::{ControlledChild, ControllerReader, ControllerWriter,
                DSR_CURSOR_POSITION_ORIGIN_RESPONSE, DSR_CURSOR_POSITION_REQUEST};
    use miette::miette;
    use std::io::{Cursor, Read, Write};

    /// Performs the Windows [`ConPTY`] initialization handshake.
    ///
    /// When [`portable_pty`] allocates a pseudoconsole with
    /// `PSEUDOCONSOLE_INHERIT_CURSOR`, `conhost.exe` transmits a cursor position request
    /// ([`DSR`]) through the output pipe and halts input processing until the terminal
    /// controller replies with a cursor position report ([`origin response`]).
    ///
    /// This function reads from `reader` until [`DSR`] is detected, sends the
    /// [`origin response`] via `writer`, and strips [`DSR`] from the stream. Any
    /// preceding or trailing bytes are preserved and prepended to the returned
    /// [`ControllerReader`].
    ///
    /// [`ConPTY`]:
    ///     https://learn.microsoft.com/en-us/windows/console/creating-a-pseudoconsole-session
    /// [`DSR`]: crate::DSR_CURSOR_POSITION_REQUEST
    /// [`origin response`]: crate::DSR_CURSOR_POSITION_ORIGIN_RESPONSE
    pub fn perform_conpty_handshake(
        mut reader: ControllerReader,
        writer: &mut ControllerWriter,
        controlled_child: &ControlledChild,
    ) -> miette::Result<ControllerReader> {
        let mut buf = [0u8; 1024];
        let mut leftover = Vec::new();
        let expected_dsr = DSR_CURSOR_POSITION_REQUEST.as_bytes();

        loop {
            if is_child_terminated(controlled_child) {
                break;
            }

            let bytes_read = reader
                .read(&mut buf)
                .map_err(|e| miette!("Failed to read ConPTY handshake: {}", e))?;

            if bytes_read == 0 {
                break;
            }

            let size = expected_dsr.len();
            if let Some(dsr_start_idx) = buf[..bytes_read]
                .windows(size)
                .position(|byte_chunk| byte_chunk == expected_dsr)
            {
                writer
                    .write_all(DSR_CURSOR_POSITION_ORIGIN_RESPONSE.as_bytes())
                    .map_err(|e| {
                        miette!("Failed to write ConPTY handshake response: {}", e)
                    })?;
                writer.flush().map_err(|e| {
                    miette!("Failed to flush ConPTY handshake response: {}", e)
                })?;

                // Preserve any bytes before or after the DSR request.
                leftover.extend_from_slice(&buf[..dsr_start_idx]);
                let dsr_end_idx = dsr_start_idx + expected_dsr.len();
                if dsr_end_idx < bytes_read {
                    leftover.extend_from_slice(&buf[dsr_end_idx..bytes_read]);
                }
                break;
            }

            leftover.extend_from_slice(&buf[..bytes_read]);

            if is_child_terminated(controlled_child) {
                break;
            }
        }

        if leftover.is_empty() {
            Ok(reader)
        } else {
            let chained = Cursor::new(leftover).chain(reader);
            Ok(Box::new(chained))
        }
    }

    /// Polls the child process handle without blocking to check if it has exited.
    fn is_child_terminated(child: &ControlledChild) -> bool {
        use std::os::windows::io::RawHandle;

        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn WaitForSingleObject(hHandle: RawHandle, dwMilliseconds: u32) -> u32;
        }
        const WAIT_OBJECT_0: u32 = 0;

        if let Some(raw_handle) = child.as_raw_handle() {
            unsafe { WaitForSingleObject(raw_handle, 0) == WAIT_OBJECT_0 }
        } else {
            false
        }
    }
}

// cspell:words pseudoconsole PSEUDOCONSOLE conhost
