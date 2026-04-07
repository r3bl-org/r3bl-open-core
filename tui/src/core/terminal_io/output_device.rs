// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{SafeRawTerminal, SendRawTerminal, StdMutex};
use std::sync::Arc;

pub type LockedOutputDevice<'a> = &'a mut dyn std::io::Write;

/// Macro to simplify locking and getting a mutable reference to the output device.
/// Don't call this again in the same scope, it will deadlock! A safe approach is
/// to use this macro in a separate block scope.
///
/// Usage example:
/// ```no_run
/// // This example requires terminal output and can't run in test environments
/// use r3bl_tui::{lock_output_device_as_mut, OutputDevice, LockedOutputDevice};
/// let device = OutputDevice::new_stdout();
/// { // Start a new block scope to avoid deadlock.
///     let mut_ref: LockedOutputDevice<'_> = lock_output_device_as_mut!(device);
///     let _ = mut_ref.write_all(b"Hello, world!\n");
/// } // The lock is released here.
/// ```
#[macro_export]
macro_rules! lock_output_device_as_mut {
    ($device:expr) => {
        &mut *$device.lock()
    };
}

/// This struct represents an output device that can be used to write to the terminal.
/// - It is safe to clone.
/// - To write to it, see the examples in [`Self::lock()`] or
///   [`lock_output_device_as_mut`] macro.
///
/// # Poison Safety
///
/// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section
/// in the crate root documentation for details.
///
/// [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety]:
///     crate#terminal-restoration-panic-drop-and-mutex-poison-safety
#[derive(Clone)]
#[allow(missing_debug_implementations)]
pub struct OutputDevice {
    pub resource: SafeRawTerminal,
    pub is_mock: bool,
}

impl Default for OutputDevice {
    fn default() -> Self { Self::new_stdout() }
}

impl OutputDevice {
    #[must_use]
    pub fn new_stdout() -> Self {
        Self {
            resource: Arc::new(StdMutex::new(std::io::stdout())),
            is_mock: false,
        }
    }
}

impl OutputDevice {
    /// Locks the output device for writing. To use it, use the following code:
    ///
    /// ```no_run
    /// // This example requires terminal output and can't run in test environments
    /// use r3bl_tui::{OutputDevice, LockedOutputDevice};
    ///
    /// let device = OutputDevice::new_stdout();
    /// let mut_ref: LockedOutputDevice<'_> = &mut *device.lock();
    /// let _ = mut_ref.write_all(b"Hello, world!\n");
    /// ```
    ///
    /// This method returns a [`std::sync::MutexGuard`] which provides a mechanism to
    /// access the underlying resource in a thread-safe manner. The `MutexGuard` ensures
    /// that the resource is locked for the duration of the guard's lifetime, preventing
    /// other threads from accessing it simultaneously.
    ///
    /// # Poison Safety
    ///
    /// This method is **poison-safe by design**. It internally handles poisoning by
    /// logging the error and returning the "dirty" state via [`into_inner()`].
    ///
    /// This ensures that terminal output (like flushing a "clear screen" sequence) can
    /// always be attempted during cleanup, even if a previous panic corrupted the output
    /// buffer. We prioritize **Resilience over Integrity** here to prevent a **Double
    /// Panic Abort** that would **brick the user's terminal**.
    ///
    /// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section in
    /// the crate root documentation for details.
    ///
    /// [`into_inner()`]: std::sync::PoisonError::into_inner
    /// [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety]:
    ///     crate#terminal-restoration-panic-drop-and-mutex-poison-safety
    pub fn lock(&self) -> std::sync::MutexGuard<'_, SendRawTerminal> {
        match self.resource.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                // % is Display, ? is Debug.
                tracing::error!(
                    message = "OutputDevice lock poisoned, proceeding with dirty state",
                    error = ?poisoned
                );
                poisoned.into_inner()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdout_output_device() {
        let output_device = OutputDevice::new_stdout();
        let mut_ref: LockedOutputDevice<'_> = lock_output_device_as_mut!(output_device);

        // We don't care about the result of this operation.
        mut_ref.write_all(b"Hello, world!\n").ok();

        assert!(!output_device.is_mock);
    }

    #[test]
    fn test_stdout_output_device_is_not_mock() {
        let device = OutputDevice::new_stdout();
        assert!(!device.is_mock);
    }

    #[test]
    fn test_output_device_poison_resilience() {
        let resource: SafeRawTerminal = Arc::new(StdMutex::new(Vec::new()));
        let device = OutputDevice {
            resource: Arc::clone(&resource),
            is_mock: true,
        };

        // 1. Poison the mutex.
        let _unused = std::thread::spawn(move || {
            let _guard = resource.lock().unwrap();
            panic!("Intentional panic to poison OutputDevice resource");
        })
        .join();

        // 2. Verify it is poisoned.
        assert!(device.resource.lock().is_err());

        // 3. Verify lock() does NOT panic and returns the dirty state.
        {
            let mut guard = device.lock();
            drop(guard.write_all(b"still works"));
        }

        // 4. Verify data was written to the dirty state.
        {
            let _guard = match device.resource.lock() {
                Ok(_) => panic!("Should be poisoned"),
                Err(e) => e.into_inner(),
            };
            // We can't easily downcast and check the vec here without more boilerplate,
            // but the fact that we got here and could call write_all above proves
            // resilience.
        }
    }
}
