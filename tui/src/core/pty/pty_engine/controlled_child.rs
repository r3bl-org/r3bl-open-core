// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#[cfg(windows)]
use super::windows_terminate_process::WindowsProcessKiller;
use portable_pty::ChildKiller;
use std::{fmt::{Debug, Formatter},
          ops::{Deref, DerefMut}};

/// Newtype wrapping [`SafeBoxedPtyChild`] representing a child process spawned in a
/// [`PTY`] session.
///
/// This provides a safe, cross-platform interface for child process termination and
/// lifecycle management.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub struct ControlledChild {
    inner: SafeBoxedPtyChild,
}

/// Type alias for a thread-safe boxed [`portable_pty::Child`] process trait object.
pub type SafeBoxedPtyChild = Box<dyn portable_pty::Child + Send + Sync>;

/// Type alias for a controlled child termination handle.
pub type ControlledChildTerminationHandle = Box<dyn ChildKiller + Send + Sync>;

impl ControlledChild {
    /// Creates a safe termination handle for this child process.
    ///
    /// On Windows, this duplicates the process handle to bypass the inverted return
    /// check bug in `portable-pty 0.9.0` (fixed upstream in [wezterm PR
    /// 7709]). On Unix, this delegates directly to
    /// [`portable_pty::ChildKiller::clone_killer`].
    ///
    /// [wezterm PR 7709]: https://github.com/wezterm/wezterm/pull/7709
    #[must_use]
    pub fn clone_termination_handle(&self) -> ControlledChildTerminationHandle {
        #[cfg(windows)]
        if let Some(killer) = WindowsProcessKiller::try_from_child(&*self.inner) {
            return Box::new(killer);
        }
        self.inner.clone_killer()
    }

    /// Alias for [`Self::clone_termination_handle`] that shadows the upstream
    /// [`portable_pty::ChildKiller::clone_killer`] method.
    #[must_use]
    pub fn clone_killer(&self) -> ControlledChildTerminationHandle {
        self.clone_termination_handle()
    }
}

mod impl_controlled_child {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Deref for ControlledChild {
        type Target = dyn portable_pty::Child + Send + Sync;

        fn deref(&self) -> &Self::Target { &*self.inner }
    }

    impl DerefMut for ControlledChild {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut *self.inner }
    }

    impl Debug for ControlledChild {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("ControlledChild")
                .field("process_id", &self.inner.process_id())
                .finish()
        }
    }

    impl From<SafeBoxedPtyChild> for ControlledChild {
        fn from(inner: SafeBoxedPtyChild) -> Self { Self { inner } }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use portable_pty::{Child, ExitStatus};
    use std::io::Result as IoResult;

    #[derive(Debug)]
    struct MockKiller;

    impl ChildKiller for MockKiller {
        fn kill(&mut self) -> IoResult<()> { Ok(()) }

        fn clone_killer(&self) -> Box<dyn ChildKiller + Send + Sync> {
            Box::new(MockKiller)
        }
    }

    #[derive(Debug)]
    struct MockChild {
        pid: Option<u32>,
    }

    impl ChildKiller for MockChild {
        fn kill(&mut self) -> IoResult<()> { Ok(()) }

        fn clone_killer(&self) -> Box<dyn ChildKiller + Send + Sync> {
            Box::new(MockKiller)
        }
    }

    impl Child for MockChild {
        fn wait(&mut self) -> IoResult<ExitStatus> { Ok(ExitStatus::with_exit_code(0)) }

        fn try_wait(&mut self) -> IoResult<Option<ExitStatus>> {
            Ok(Some(ExitStatus::with_exit_code(0)))
        }

        fn process_id(&self) -> Option<u32> { self.pid }

        #[cfg(target_os = "windows")]
        fn as_raw_handle(&self) -> Option<std::os::windows::io::RawHandle> { None }
    }

    #[test]
    fn test_controlled_child_from_and_deref_and_deref_mut() {
        let boxed: SafeBoxedPtyChild = Box::new(MockChild { pid: Some(1234) });
        let mut child = ControlledChild::from(boxed);
        // Test Deref:
        assert_eq!(child.process_id(), Some(1234));
        // Test DerefMut:
        assert!(child.try_wait().is_ok());
    }

    #[test]
    fn test_controlled_child_debug_formatting() {
        let boxed: SafeBoxedPtyChild = Box::new(MockChild { pid: Some(9999) });
        let child = ControlledChild::from(boxed);
        let debug_str = format!("{child:?}");
        assert!(debug_str.contains("ControlledChild"));
        assert!(debug_str.contains("9999"));
    }

    #[test]
    fn test_controlled_child_clone_killer_and_termination_handle() {
        let boxed: SafeBoxedPtyChild = Box::new(MockChild { pid: None });
        let child = ControlledChild::from(boxed);
        let mut killer_1 = child.clone_termination_handle();
        assert!(killer_1.kill().is_ok());

        let mut killer_2 = child.clone_killer();
        assert!(killer_2.kill().is_ok());
    }
}
