// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::any::Any;

// cspell:words FAILCRITICALERRORS HKLM NOGPFAULTERRORBOX

// Attach sources.
pub mod input_device_fixtures;
pub mod isolated_process_fixtures;
pub mod output_device_fixtures;
pub mod pty_test_fixtures;
pub mod retry;
pub mod tcp_stream_fixtures;

// Re-export.
pub use input_device_fixtures::*;
pub use isolated_process_fixtures::*;
pub use output_device_fixtures::*;
pub use pty_test_fixtures::*;
pub use retry::*;
pub use tcp_stream_fixtures::*;

/// Type alias for the result of a caught panic.
///
/// The following methods have the same return type, `Result<(), dyn Any + Send>`:
/// 1. [`std::thread::JoinHandle::join`]
/// 2. [`std::panic::catch_unwind`]
///
/// The `Err(dyn Any + Send)` variant contains the payload captured during the panic.
/// Panicking generates a string message or payload.
///
/// For example, when a [`Mutex`] `lock().unwrap()` panics due to lock poisoning, the
/// [`Result::Err(PoisonError)`] gets formatted to a string and the type is erased (just a
/// string is left).
///
/// # Example
///
/// ```no_run
/// use r3bl_tui::CaughtPanicResult;
///
/// // 1. Using catch_unwind.
/// let result: CaughtPanicResult = std::panic::catch_unwind(|| {
///     panic!("intentional panic");
/// });
///
/// // 2. Using thread join.
/// let result: CaughtPanicResult = std::thread::spawn(|| {
///     panic!("intentional panic");
/// }).join();
/// ```
///
/// [`Mutex`]: std::sync::Mutex
/// [`Result::Err(PoisonError)`]: std::sync::PoisonError
pub type CaughtPanicResult<T = ()> = Result<T, Box<dyn Any + Send>>;

/// Extracts the message (as a [`String`]) from a [`CaughtPanicResult`].
///
/// This function attempts to downcast the generic panic payload into a
/// human-readable [`String`]. It handles both literal string slices ([`&str`]) and
/// heap-allocated [`String`]s.
///
/// # Example
///
/// ```no_run
/// use r3bl_tui::{CaughtPanicResult, extract_panic_message};
///
/// let result: CaughtPanicResult = std::panic::catch_unwind(|| {
///     panic!("intentional panic");
/// });
/// let message = extract_panic_message(result);
/// assert_eq!(message, "intentional panic");
/// ```
///
/// # Panics
///
/// Panics if the provided `result` is [`Ok`]. This function is only intended to be
/// called on results that are known to contain a panic (e.g., in a test after a
/// suspected failure).
///
/// # Returns
///
/// - The extracted message as a [`String`].
/// - A default "Unknown panic payload" message if the payload type is neither [`&str`]
///   nor [`String`].
pub fn extract_panic_message<T>(result: CaughtPanicResult<T>) -> String {
    let panic_payload = result.err().expect("Expected a panic but found Ok");
    // Try cast to &str.
    if let Some(s) = panic_payload.downcast_ref::<&str>() {
        return s.to_string();
    }
    // Try cast to String.
    if let Some(s) = panic_payload.downcast_ref::<String>() {
        return s.clone();
    }
    // Both attempts at casting failed.
    format!("Unknown panic payload: {panic_payload:?}")
}

/// Creates a [`std::process::Command`] for the current test executable, configured
/// for isolated test runner usage. On Windows, sets [`CREATE_NO_WINDOW`] to prevent
/// console window flashing during child process spawns.
///
/// # Panics
///
/// Panics if [`std::env::current_exe()`] fails to determine the test binary path.
///
/// [`CREATE_NO_WINDOW`]: https://learn.microsoft.com/en-us/windows/win32/procthread/process-creation-flags
#[must_use]
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
/// When running `cargo test` over [`SSH`] on Windows, one `cmd.exe` "application error"
/// dialog may still appear per test run. This is caused by the test harness process
/// itself (which is spawned by the [`SSH`] shell, not by our code) and cannot be
/// suppressed from within test code. It is cosmetic and does not affect test results.
/// To suppress it system-wide, set this registry key:
///
/// ```text
/// reg add "HKLM\SOFTWARE\Microsoft\Windows\Windows Error Reporting" /v DontShowUI /t REG_DWORD /d 1 /f
/// ```
///
/// [`SSH`]: https://en.wikipedia.org/wiki/Secure_Shell
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_panic_message() {
        let payload_str: Box<dyn Any + Send> = Box::new("test str");
        assert_eq!(
            extract_panic_message(Result::<(), _>::Err(payload_str)),
            "test str"
        );

        let payload_string: Box<dyn Any + Send> = Box::new("test string".to_string());
        assert_eq!(
            extract_panic_message(Result::<(), _>::Err(payload_string)),
            "test string"
        );

        let payload_other: Box<dyn Any + Send> = Box::new(42);
        assert!(
            extract_panic_message(Result::<(), _>::Err(payload_other))
                .contains("Unknown panic payload")
        );
    }
}
