// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{reader::spawn_pty_reader_thread, writer::spawn_pty_writer_thread};
use crate::{ControlledChild, Controller, PtyControlledChildExitStatus, PtyInputEvent,
            PtyOrchestratorHandle, PtyOutputEvent, PtySessionConfig};
use miette::miette;
use std::sync::{Arc, Mutex,
                mpsc::{Receiver, SyncSender}};

/// Spawns the **Orchestrator Thread** for a [`PtySession`].
///
/// This thread is the "Director" of the session. It runs with the name
/// `pty-orchestrator`. It:
/// 1. Takes the writer and reader from [`Controller`].
/// 2. Performs the Windows [`ConPTY`] initialization handshake (Windows only).
/// 3. Spawns the **Reader Thread**.
/// 4. Spawns the **Writer Thread**.
/// 5. Waits for the child process to exit.
/// 6. Destroys the pseudo-console controller ([`ClosePseudoConsole`] on Windows) to
///    unblock the reader.
/// 7. Joins both background threads.
/// 8. Sends the final [`PtyOutputEvent::Exit`] event.
///
/// # Errors
///
/// Returns an [`Err`] if:
/// - Taking the writer from [`Controller`] fails.
/// - Cloning the reader from [`Controller`] fails.
/// - Performing the Windows [`ConPTY`] handshake fails (Windows only).
/// - Spawning the orchestrator OS thread fails.
///
/// For the complete lifecycle architecture, see the [Session Layer] documentation.
///
/// [`ClosePseudoConsole`]:
///     https://learn.microsoft.com/en-us/windows/console/closepseudoconsole
/// [`ConPTY`]:
///     https://learn.microsoft.com/en-us/windows/console/creating-a-pseudoconsole-session
/// [`PtySession`]: crate::PtySession
/// [Session Layer]: mod@crate::pty_session
pub fn spawn_pty_orchestrator_thread(
    mut controlled_child: ControlledChild,
    controller: Controller,
    input_event_ch_tx_half: SyncSender<PtyInputEvent>,
    input_event_ch_rx_half: Receiver<PtyInputEvent>,
    output_event_ch_tx_half: SyncSender<PtyOutputEvent>,
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

    let config = arg_config.into();
    let input_event_ch_tx_half_clone = input_event_ch_tx_half.clone();

    let handle = std::thread::Builder::new()
        .name("pty-orchestrator".into())
        .spawn(move || -> miette::Result<PtyControlledChildExitStatus> {
            let shared_controller = Arc::new(Mutex::new(Some(controller)));

            // Spawn background threads.
            let reader_thread_handle = spawn_pty_reader_thread(
                controller_reader,
                output_event_ch_tx_half.clone(),
                config,
            )?;

            let writer_thread_handle = spawn_pty_writer_thread(
                controller_writer,
                shared_controller.clone(),
                input_event_ch_rx_half,
                output_event_ch_tx_half.clone(),
            )?;

            // Wait for the child process to exit.
            let status = controlled_child
                .wait()
                .map_err(|e| miette!("Child process wait failed: {}", e))?;

            let status = PtyControlledChildExitStatus { inner: status };

            // Child process has terminated. Destroy the pseudo-console controller.
            // On Windows, MasterPty::drop invokes ClosePseudoConsole(), which closes
            // the ConPTY output pipe and delivers EOF (0 bytes or BrokenPipe) to the
            // reader thread, allowing the reader thread to exit cleanly.
            if let Ok(mut guard) = shared_controller.lock() {
                drop(guard.take());
            }

            // Send Close event to signal writer thread to stop (if not already stopped).
            // We do this via the sender side (which we still have a clone of).
            let _unused = input_event_ch_tx_half_clone.send(PtyInputEvent::Close);

            // Wait for background threads to finish.
            drop(reader_thread_handle.join());
            drop(writer_thread_handle.join());

            // Send the exit event.
            let _unused =
                output_event_ch_tx_half.send(PtyOutputEvent::Exit(status.clone()));

            Ok(status)
        })
        .map_err(|e| miette!("Failed to spawn pty-orchestrator thread: {e}"))?;

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
