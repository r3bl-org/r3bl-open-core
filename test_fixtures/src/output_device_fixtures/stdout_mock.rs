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

use std::{io::{Result, Write},
          sync::Arc};

use r3bl_core::{StdMutex, VecArray};
use smallvec::smallvec;
use strip_ansi_escapes::strip;

/// You can safely clone this struct, since it only contains an `Arc<StdMutex<Vec<u8>>>`.
/// The inner `buffer` will not be cloned, just the [Arc] will be cloned.
///
/// The main constructors are:
/// - [StdoutMock::default]
/// - [StdoutMock::new]
/// - [super::OutputDeviceExt::new_mock()]
#[derive(Clone)]
pub struct StdoutMock {
    pub buffer: Arc<StdMutex<VecArray<u8>>>,
}

impl Default for StdoutMock {
    fn default() -> Self {
        Self {
            buffer: Arc::new(StdMutex::new(smallvec![])),
        }
    }
}

impl StdoutMock {
    pub fn new() -> Self { Self::default() }
}

impl StdoutMock {
    pub fn get_copy_of_buffer(&self) -> VecArray<u8> {
        self.buffer.lock().unwrap().clone()
    }

    pub fn get_copy_of_buffer_as_string(&self) -> String {
        let buffer_data = self.buffer.lock().unwrap();
        String::from_utf8(buffer_data.to_vec()).expect("utf8")
    }

    pub fn get_copy_of_buffer_as_string_strip_ansi(&self) -> String {
        let buffer_data = self.buffer.lock().unwrap();
        let buffer_data = strip(buffer_data.to_vec());
        String::from_utf8(buffer_data).expect("utf8")
    }
}

impl Write for StdoutMock {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.buffer.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

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
    let red_text = format!("\x1b[31m{}\x1b[0m", normal_text); // Resets color after.

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
