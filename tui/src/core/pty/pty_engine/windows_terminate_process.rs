// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.
use portable_pty::{Child, ChildKiller};
use std::{io::Result,
          os::windows::io::{AsRawHandle, BorrowedHandle, OwnedHandle, RawHandle}};

#[link(name = "kernel32")]
unsafe extern "system" {
    fn TerminateProcess(hProcess: RawHandle, uExitCode: u32) -> i32;
}

/// Windows process killer that directly invokes the Win32 [`TerminateProcess`] API on an
/// independent, duplicated process handle.
///
/// # Upstream Bug Rationale
///
/// In `portable-pty 0.9.0`, `WinChildKiller::kill()` contains an inverted return value
/// check (`if res != 0 { Err(err) } else { Ok(()) }`). Because [`TerminateProcess`]
/// returns nonzero on success, `WinChildKiller::kill()` erroneously returns an [`Err`]
/// result even when the child process was terminated successfully.
///
/// This upstream defect was fixed in [wezterm PR 7709], but as of version 0.9.0 that fix
/// is not yet published to crates.io. By duplicating the raw process handle into an owned
/// handle and managing termination directly, this struct ensures that termination
/// succeeds reliably without waiting for an upstream release.
///
/// [`TerminateProcess`]:
///     https://docs.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-terminateprocess
/// [wezterm PR 7709]: https://github.com/wezterm/wezterm/pull/7709
#[derive(Debug)]
pub struct WindowsProcessKiller(pub OwnedHandle);

impl WindowsProcessKiller {
    /// Attempts to duplicate the process handle from a [`Child`] process.
    #[must_use]
    pub fn try_from_child(child: &(dyn Child + Send + Sync)) -> Option<Self> {
        let raw = child.as_raw_handle()?;
        unsafe { BorrowedHandle::borrow_raw(raw) }
            .try_clone_to_owned()
            .ok()
            .map(Self)
    }
}

impl ChildKiller for WindowsProcessKiller {
    fn kill(&mut self) -> Result<()> {
        // Win32 TerminateProcess returns nonzero on success, zero on failure.
        let res = unsafe { TerminateProcess(self.0.as_raw_handle(), 1) };
        if res == 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    fn clone_killer(&self) -> Box<dyn ChildKiller + Send + Sync> {
        let handle = self
            .0
            .try_clone()
            .expect("Failed to duplicate Windows process handle");
        Box::new(WindowsProcessKiller(handle))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{os::windows::{io::AsRawHandle, process::CommandExt},
              process::Command};

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    #[derive(Debug)]
    struct MockChildWithHandle {
        handle: Option<RawHandle>,
    }

    unsafe impl Send for MockChildWithHandle {}
    unsafe impl Sync for MockChildWithHandle {}

    impl ChildKiller for MockChildWithHandle {
        fn kill(&mut self) -> Result<()> { Ok(()) }

        fn clone_killer(&self) -> Box<dyn ChildKiller + Send + Sync> {
            Box::new(MockChildWithHandle { handle: None })
        }
    }

    impl Child for MockChildWithHandle {
        fn wait(&mut self) -> Result<portable_pty::ExitStatus> {
            Ok(portable_pty::ExitStatus::with_exit_code(0))
        }

        fn try_wait(&mut self) -> Result<Option<portable_pty::ExitStatus>> {
            Ok(Some(portable_pty::ExitStatus::with_exit_code(0)))
        }

        fn process_id(&self) -> Option<u32> { None }

        fn as_raw_handle(&self) -> Option<RawHandle> { self.handle }
    }

    #[test]
    fn test_try_from_child_none_when_no_handle() {
        let child = MockChildWithHandle { handle: None };
        assert!(WindowsProcessKiller::try_from_child(&child).is_none());
    }

    #[test]
    fn test_windows_process_killer_kill_and_clone() {
        let mut child = Command::new("powershell")
            .args(["-NoProfile", "-Command", "Start-Sleep -Seconds 10"])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .expect("Failed to spawn test process");

        let raw = AsRawHandle::as_raw_handle(&child);
        let mock_child = MockChildWithHandle { handle: Some(raw) };

        let killer = WindowsProcessKiller::try_from_child(&mock_child);
        assert!(killer.is_some());

        let mut killer = killer.expect("Killer handle duplication failed");
        let mut cloned = killer.clone_killer();

        // 1. Cloned killer successfully terminates the running process (res != 0 -> Ok).
        assert!(cloned.kill().is_ok());

        // Wait for process to terminate.
        drop(child.wait());

        // 2. Killing an already terminated process returns an error (res == 0 -> Err).
        assert!(killer.kill().is_err());
    }
}
