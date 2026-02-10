// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Common I/O implementation shared between PTY session types.
//!
//! This module provides the core I/O functionality used by both read-only and
//! read-write PTY sessions:
//! - PTY pair creation and configuration
//! - Async task spawning for I/O operations
//! - Input/output event handling
//! - Resource management and cleanup

use super::pty_core::pty_types::{Controlled, ControlledChild, Controller, PtyCommand};
use portable_pty::{PtySize, native_pty_system};

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

        #[cfg(unix)]
        let command = {
            let mut cmd = PtyCommand::new("echo");
            cmd.arg("test");
            cmd
        };
        #[cfg(windows)]
        let command = {
            let mut cmd = PtyCommand::new("cmd");
            cmd.args(["/c", "echo", "test"]);
            cmd
        };

        let result = spawn_command_in_pty(&controlled, command);
        assert!(result.is_ok());
    }
}
