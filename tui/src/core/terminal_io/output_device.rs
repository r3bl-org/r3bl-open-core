/*
 *   Copyright (c) 2024-2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

use std::sync::Arc;

use crate::{SafeRawTerminal, SendRawTerminal, StdMutex};

pub type LockedOutputDevice<'a> = &'a mut dyn std::io::Write;

/// Macro to simplify locking and getting a mutable reference to the output device.
/// Don't call this again in the same scope, it will deadlock! A safe approach is
/// to use this macro in a separate block scope.
///
/// Usage example:
/// ```
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

    #[must_use]
    pub fn new_stderr() -> Self {
        Self {
            resource: Arc::new(StdMutex::new(std::io::stderr())),
            is_mock: false,
        }
    }
}

impl OutputDevice {
    /// Locks the output device for writing. To use it, use the following code:
    ///
    /// ```
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
    /// # Panics
    ///
    /// This method will panic if the mutex is poisoned, which can happen if a thread
    /// panics while holding the lock. To avoid panics, ensure that the code that
    /// locks the mutex does not panic while holding the lock.
    pub fn lock(&self) -> std::sync::MutexGuard<'_, SendRawTerminal> {
        self.resource.lock().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdout_output_device() {
        let output_device = OutputDevice::new_stdout();
        let mut_ref: LockedOutputDevice<'_> = lock_output_device_as_mut!(output_device);
        drop(mut_ref.write_all(b"Hello, world!\n"));
        assert!(!output_device.is_mock);
    }

    #[test]
    fn test_stdout_output_device_is_not_mock() {
        let device = OutputDevice::new_stdout();
        assert!(!device.is_mock);
    }
}
