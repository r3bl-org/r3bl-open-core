// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Attach sources.
pub mod input_device_fixtures;
pub mod output_device_fixtures;
pub mod pty_test_fixtures;
pub mod tcp_stream_fixtures;

// Re-export.
pub use input_device_fixtures::*;
pub use output_device_fixtures::*;
pub use pty_test_fixtures::*;
pub use tcp_stream_fixtures::*;

/// Create a [`std::process::Command`] for the current test executable, configured
/// for isolated test runner usage. On Windows, sets [`CREATE_NO_WINDOW`] to prevent
/// console window flashing during child process spawns.
///
/// [`CREATE_NO_WINDOW`]: https://learn.microsoft.com/en-us/windows/win32/procthread/process-creation-flags
pub fn new_isolated_test_command() -> std::process::Command {
    #[allow(unused_mut)]
    let mut cmd =
        std::process::Command::new(std::env::current_exe().expect("current_exe"));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Suppress Windows Error Reporting (WER) dialog boxes in the current process.
/// Call this at the top of any test function that spawns child processes to prevent
/// WER crash/error dialogs from blocking test execution. Child processes inherit the
/// error mode, so calling this once in the parent covers all descendants.
///
/// This is a no-op on non-Windows platforms.
///
/// # Known limitation
///
/// When running `cargo test` over SSH on Windows, one `cmd.exe` "application error"
/// dialog may still appear per test run. This is caused by the test harness process
/// itself (which is spawned by the SSH shell, not by our code) and cannot be
/// suppressed from within test code. It is cosmetic and does not affect test results.
/// To suppress it system-wide, set this registry key:
///
/// ```text
/// reg add "HKLM\SOFTWARE\Microsoft\Windows\Windows Error Reporting" /v DontShowUI /t REG_DWORD /d 1 /f
/// ```
pub fn suppress_wer_dialogs() {
    #[cfg(windows)]
    {
        // SEM_FAILCRITICALERRORS (0x0001): Don't show critical error message boxes.
        // SEM_NOGPFAULTERRORBOX (0x0002): Don't show GP fault error box (WER).
        const SEM_FAILCRITICALERRORS: u32 = 0x0001;
        const SEM_NOGPFAULTERRORBOX: u32 = 0x0002;
        unsafe extern "system" {
            fn SetErrorMode(mode: u32) -> u32;
        }
        unsafe {
            SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX);
        }
    }
}
