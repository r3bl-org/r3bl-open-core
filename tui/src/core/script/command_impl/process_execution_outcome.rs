// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::{io::{Error as IoError, Result as IoResult},
          process::Output};

/// Represents the tri-state outcome of executing an operating system process.
///
/// This enum cleanly separates the following 3 states:
/// 1. Success - Processes that ran to completion with zero exit status.
/// 2. Failure - Processes that ran to completion with non-zero exit status.
/// 3. Failure - Process spawn failures (such as binary not found or permission denied).
///
/// # Design
///
/// Standard library and Tokio process executions return [`IoResult<Output>`]. This
/// representation conflates operating system execution failures with application-level
/// failures. [`ProcessExecutionOutcome`] makes all three states explicit, eliminating nested
/// match expressions and boolean blindness.
///
/// # Examples
///
/// ```no_run
/// use r3bl_tui::ProcessExecutionOutcome;
/// use std::process::Command;
///
/// let res_output = Command::new("echo").arg("hello").output();
/// let outcome = ProcessExecutionOutcome::from(res_output);
/// match outcome {
///     ProcessExecutionOutcome::Success(output) => {
///         assert!(output.status.success());
///     }
///     ProcessExecutionOutcome::NonZeroExit(output) => {
///         eprintln!("Command failed with status: {}", output.status);
///     }
///     ProcessExecutionOutcome::SpawnFailed(err) => {
///         eprintln!("Failed to spawn command: {err}");
///     }
/// }
/// ```
///
/// [`IoResult<Output>`]: std::io::Result
/// [`Output`]: std::process::Output
#[derive(Debug)]
pub enum ProcessExecutionOutcome {
    /// Process was successfully spawned and exited with zero status (success).
    Success(Output),
    /// Process was successfully spawned, but exited with non-zero status.
    NonZeroExit(Output),
    /// OS failed to spawn or execute the process (such as binary not found or permission
    /// denied).
    SpawnFailed(IoError),
}

impl From<IoResult<Output>> for ProcessExecutionOutcome {
    fn from(result: IoResult<Output>) -> Self {
        match result {
            Ok(output) => {
                if output.status.success() {
                    ProcessExecutionOutcome::Success(output)
                } else {
                    ProcessExecutionOutcome::NonZeroExit(output)
                }
            }
            Err(err) => ProcessExecutionOutcome::SpawnFailed(err),
        }
    }
}

impl ProcessExecutionOutcome {
    /// Constructs a [`std::process::ExitStatus`] directly from an integer exit code.
    ///
    /// The Rust standard library does not provide a cross-platform constructor to create
    /// an [`ExitStatus`] directly from an exit code. This function handles POSIX wait
    /// status bit packing and Windows [`DWORD`] representations.
    ///
    /// # Platform Specifics
    ///
    /// ## POSIX Wait Status Word (`#[cfg(unix)]`)
    ///
    /// The exit code is clamped to `0..=255` and shifted left by 8 bits (`exit_code <<
    /// 8`) into bits 8..=15 of the wait status integer, which [`WEXITSTATUS`] extracts:
    ///
    /// ```text
    ///  31                                                                 0
    /// ┌──────────────┬───────────────────┬────────────┬────────────────────┐
    /// │ Bits 31..=16 │ Bits 15..=8       │ Bit 7      │  Bits 6..=0        │
    /// ├──────────────┼───────────────────┼────────────┼────────────────────┤
    /// │ Unused / OS  │ Exit Code (0-255) │ Core Dump  │ Termination Signal │
    /// │   (0x0000)   │   (WEXITSTATUS)   │    Flag    │       (0x00)       │
    /// └──────────────┴───────────────────┴────────────┴────────────────────┘
    /// ```
    ///
    /// ## Windows Exit Code (`#[cfg(windows)]`)
    ///
    /// Windows stores the exit code directly as a 32-bit [`DWORD`] (no bitfield packing):
    ///
    /// ```text
    ///  31                                                                 0
    /// ┌────────────────────────────────────────────────────────────────────┐
    /// │ Bits 31..=0                                                        │
    /// ├────────────────────────────────────────────────────────────────────┤
    /// │                       Exit Code DWORD (u32)                        │
    /// └────────────────────────────────────────────────────────────────────┘
    /// ```
    ///
    /// [`DWORD`]:
    ///     https://learn.microsoft.com/en-us/windows/win32/winprog/windows-data-types
    /// [`ExitStatus`]: std::process::ExitStatus
    /// [`waitpid(2)`]: https://manpages.debian.org/waitpid.2
    /// [`WEXITSTATUS`]:
    ///     https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/sys_wait.h.html
    #[must_use]
    pub fn make_exit_status(code: u32) -> std::process::ExitStatus {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            let clamped = code.min(255);
            let exit_code_i32 = i32::try_from(clamped).unwrap_or(255);
            let raw_wait_status = exit_code_i32 << 8;
            std::process::ExitStatus::from_raw(raw_wait_status)
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::ExitStatusExt;
            std::process::ExitStatus::from_raw(code)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;

    #[test]
    fn test_from_io_result_success() {
        let output = Output {
            status: ProcessExecutionOutcome::make_exit_status(0),
            stdout: b"success output".to_vec(),
            stderr: Vec::new(),
        };
        let outcome = ProcessExecutionOutcome::from(Ok(output));
        match outcome {
            ProcessExecutionOutcome::Success(out) => {
                assert!(out.status.success());
                assert_eq!(out.stdout, b"success output");
            }
            _ => panic!("Expected ProcessExecutionOutcome::Success"),
        }
    }

    #[test]
    fn test_from_io_result_nonzero_exit() {
        let output = Output {
            status: ProcessExecutionOutcome::make_exit_status(1),
            stdout: Vec::new(),
            stderr: b"error output".to_vec(),
        };
        let outcome = ProcessExecutionOutcome::from(Ok(output));
        match outcome {
            ProcessExecutionOutcome::NonZeroExit(out) => {
                assert!(!out.status.success());
                assert_eq!(out.stderr, b"error output");
            }
            _ => panic!("Expected ProcessExecutionOutcome::NonZeroExit"),
        }
    }

    #[test]
    fn test_from_io_result_spawn_failed() {
        let err = IoError::new(ErrorKind::NotFound, "executable not found");
        let outcome = ProcessExecutionOutcome::from(Err(err));
        match outcome {
            ProcessExecutionOutcome::SpawnFailed(err) => {
                assert_eq!(err.kind(), ErrorKind::NotFound);
            }
            _ => panic!("Expected ProcessExecutionOutcome::SpawnFailed"),
        }
    }

    #[test]
    fn test_make_exit_status() {
        let success = ProcessExecutionOutcome::make_exit_status(0);
        assert!(success.success());
        assert_eq!(success.code(), Some(0));

        let failure = ProcessExecutionOutcome::make_exit_status(1);
        assert!(!failure.success());
        assert_eq!(failure.code(), Some(1));

        let custom = ProcessExecutionOutcome::make_exit_status(42);
        assert_eq!(custom.code(), Some(42));
    }
}
