// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::{io::{Result, Write},
          sync::Arc};

use smallvec::smallvec;
use strip_ansi_escapes::strip;

use crate::{InlineVec, StdMutex};

/// You can safely clone this struct, since it only contains an `Arc<StdMutex<Vec<u8>>>`.
/// The inner `buffer` will not be cloned, just the [Arc] will be cloned.
///
/// The main constructors are:
/// - [`StdoutMock::default`]
/// - [`StdoutMock::new`]
/// - [`super::OutputDeviceExt::new_mock()`]
#[derive(Clone, Debug)]
pub struct StdoutMock {
    pub buffer: Arc<StdMutex<InlineVec<u8>>>,
}

impl Default for StdoutMock {
    fn default() -> Self {
        Self {
            buffer: Arc::new(StdMutex::new(smallvec![])),
        }
    }
}

impl StdoutMock {
    #[must_use]
    pub fn new() -> Self { Self::default() }
}

impl StdoutMock {
    /// # Panics
    ///
    /// This method will panic if the lock is poisoned, which can happen if a thread
    /// panics while holding the lock. To avoid panics, ensure that the code that
    /// locks the mutex does not panic while holding the lock.
    #[must_use]
    pub fn get_copy_of_buffer(&self) -> InlineVec<u8> {
        self.buffer.lock().unwrap().clone()
    }

    /// # Panics
    ///
    /// This method will panic if the lock is poisoned, which can happen if a thread
    /// panics while holding the lock. To avoid panics, ensure that the code that
    /// locks the mutex does not panic while holding the lock.
    #[must_use]
    pub fn get_copy_of_buffer_as_string(&self) -> String {
        let buffer_data = self.buffer.lock().unwrap();
        String::from_utf8(buffer_data.to_vec()).expect("utf8")
    }

    /// # Panics
    ///
    /// This method will panic if the lock is poisoned, which can happen if a thread
    /// panics while holding the lock. To avoid panics, ensure that the code that
    /// locks the mutex does not panic while holding the lock.
    #[must_use]
    pub fn get_copy_of_buffer_as_string_strip_ansi(&self) -> String {
        let buffer_data = self.buffer.lock().unwrap();
        let buffer_data = strip(buffer_data.to_vec());
        String::from_utf8(buffer_data).expect("utf8")
    }
}

impl Write for StdoutMock {
    #[allow(clippy::unwrap_in_result)] /* unwrap is ok to use here, since it is for lock. */
    #[allow(clippy::missing_errors_doc)]
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.buffer.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    #[allow(clippy::missing_errors_doc)]
    fn flush(&mut self) -> Result<()> { Ok(()) }
}

#[tokio::test]
#[allow(clippy::needless_return)]
async fn test_stdout_mock_no_strip_ansi() {
    let mut stdout_mock = StdoutMock::default();
    let stdout_mock_clone = stdout_mock.clone(); // Points to the same inner value as `stdout_mock`.

    let normal_text = "hello world";

    stdout_mock.write_all(normal_text.as_bytes()).unwrap();
    stdout_mock.flush().unwrap();

    pretty_assertions::assert_eq!(
        stdout_mock.get_copy_of_buffer_as_string(),
        normal_text
    );
    pretty_assertions::assert_eq!(
        stdout_mock_clone.get_copy_of_buffer_as_string(),
        normal_text
    );
}

#[tokio::test]
#[allow(clippy::needless_return)]
async fn test_stdout_mock_strip_ansi() {
    let mut stdout_mock = StdoutMock::default();
    let stdout_mock_clone = stdout_mock.clone(); // Points to the same inner value as `stdout_mock`.

    let normal_text = "hello world";
    let red_text = format!("\x1b[31m{normal_text}\x1b[0m"); // Resets color after.

    stdout_mock.write_all(red_text.as_bytes()).unwrap();
    stdout_mock.flush().unwrap();

    pretty_assertions::assert_eq!(
        stdout_mock.get_copy_of_buffer_as_string_strip_ansi(),
        normal_text
    );
    pretty_assertions::assert_eq!(
        stdout_mock_clone.get_copy_of_buffer_as_string_strip_ansi(),
        normal_text
    );
}
