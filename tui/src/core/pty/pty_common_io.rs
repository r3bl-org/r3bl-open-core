// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Common I/O implementation shared between PTY session types.
//!
//! This module provides the core I/O functionality used by both read-only and
//! read-write PTY sessions:
//! - PTY pair creation and configuration
//! - Async task spawning for I/O operations
//! - Input/output event handling
//! - Resource management and cleanup

use portable_pty::{Child, MasterPty, PtySize, SlavePty, native_pty_system};

use crate::PtyCommand;

/// Buffer size for reading from PTY.
pub const READ_BUFFER_SIZE: usize = 4096;

/// Type alias for the controller half of the PTY (master).
///
/// The controller is the "master" side that your program interacts with.
/// It can read output from and write input to the spawned process.
pub type Controller = Box<dyn MasterPty + Send>;

/// Type alias for the controlled half of the PTY (slave).
///
/// The controlled is the "slave" side that the spawned process uses as its terminal.
/// The spawned process reads from and writes to this side, believing it has a real
/// terminal.
pub type Controlled = Box<dyn SlavePty + Send>;

/// Type alias for the child process spawned in the PTY.
pub type ControlledChild = Box<dyn Child + Send + Sync>;

/// Creates a PTY pair with the specified size.
///
/// # Errors
/// Returns an error if the PTY system fails to open a PTY pair.
pub fn create_pty_pair(pty_size: PtySize) -> miette::Result<(Controller, Controlled)> {
    let pty_system = native_pty_system();
    let pty_pair = pty_system
        .openpty(pty_size)
        .map_err(|e| miette::miette!("Failed to open PTY: {}", e))?;

    Ok((pty_pair.master, pty_pair.slave))
}

/// Spawns a command in the PTY.
///
/// # Errors
/// Returns an error if the command fails to spawn in the PTY.
pub fn spawn_command_in_pty(
    controlled: &Controlled,
    command: PtyCommand,
) -> miette::Result<ControlledChild> {
    controlled
        .spawn_command(command)
        .map_err(|e| miette::miette!("Failed to spawn command: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_pty_pair() {
        let pty_size = PtySize::default();
        let result = create_pty_pair(pty_size);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_pty_pair_with_custom_size() {
        let pty_size = PtySize {
            rows: 30,
            cols: 100,
            pixel_width: 0,
            pixel_height: 0,
        };
        let result = create_pty_pair(pty_size);
        assert!(result.is_ok());
    }

    #[test]
    fn test_spawn_command_in_pty() {
        let pty_size = PtySize::default();
        let (_controller, controlled) = create_pty_pair(pty_size).unwrap();

        let mut command = PtyCommand::new("echo");
        command.arg("test");

        let result = spawn_command_in_pty(&controlled, command);
        assert!(result.is_ok());
    }
}
