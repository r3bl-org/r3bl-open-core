// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{reader_task::spawn_blocking_reader_task,
            writer_task::spawn_blocking_writer_task};
use crate::{ControlledChild, Controller, ControllerReader, PtyControlledChildExitStatus,
            PtyInputEvent, PtyOutputEvent, PtySessionConfig};
use miette::miette;
use tokio::sync::mpsc::{Receiver, Sender};

/// Spawns the **Orchestrator Task** for a [`PtySession`].
///
/// This task is the "Director" of the session. It:
/// 1. Spawns the **Reader Task**.
/// 2. Spawns the **Writer Task**.
/// 3. Waits for the child process to exit.
/// 4. Joins both background tasks.
/// 5. Sends the final [`PtyOutputEvent::Exit`] event.
///
/// [`PtySession`]: crate::PtySession
#[must_use]
pub fn spawn_orchestrator_task(
    mut controlled_child: ControlledChild,
    controller_reader: ControllerReader,
    controller: Controller,
    input_event_ch_tx_half: Sender<PtyInputEvent>,
    input_event_ch_rx_half: Receiver<PtyInputEvent>,
    output_event_ch_tx_half: Sender<PtyOutputEvent>,
    arg_config: impl Into<PtySessionConfig>,
) -> tokio::task::JoinHandle<miette::Result<PtyControlledChildExitStatus>> {
    let config = arg_config.into();
    let input_event_ch_tx_half_clone = input_event_ch_tx_half.clone();
    tokio::spawn(async move {
        // 1. Spawn background tasks.
        let output_reader_task_handle = spawn_blocking_reader_task(
            controller_reader,
            output_event_ch_tx_half.clone(),
            config,
        );

        let input_writer_task_handle = spawn_blocking_writer_task(
            controller,
            input_event_ch_rx_half,
            output_event_ch_tx_half.clone(),
        );

        // 2. Wait for the child process to exit.
        let status = tokio::task::spawn_blocking(move || controlled_child.wait())
            .await
            .map_err(|e| miette!("Wait task failed: {}", e))?
            .map_err(|e| miette!("Child process wait failed: {}", e))?;

        let status = PtyControlledChildExitStatus { inner: status };

        // 3. Send Close event to signal writer task to stop.
        // We do this via the sender side (which we still have a clone of).
        let _unused = input_event_ch_tx_half_clone
            .send(PtyInputEvent::Close)
            .await;

        // 4. Wait for background tasks to finish.
        drop(output_reader_task_handle.await);
        drop(input_writer_task_handle.await);

        // 5. Send the exit event.
        let _unused = output_event_ch_tx_half
            .send(PtyOutputEvent::Exit(status.clone()))
            .await;

        Ok(status)
    })
}
