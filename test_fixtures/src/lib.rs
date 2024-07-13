/*
 *   Copyright (c) 2024 R3BL LLC
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

//! This is a test fixtures library that provides reusable components for testing. It is
//! meant to be used by all the crates in the `r3bl-open-core` monorepo. This crate is
//! intended to be a
//! [`dev-dependency`](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#dev-dependencies)
//! for other creates in the monorepo.
//!
//! It provides fixtures to test async streams and stdout. This allows TUI frameworks to
//! be tested "end to end".
//! 1. The async stream fixtures are used to test the input stream of a TUI framework.
//! 2. The stdout fixtures are used to test the output of a TUI framework.
//!
//! ## async_stream_fixtures
//!
//! Here's an example of how create a stream of `T` from a `Vec<T>`.
//!
//! ```
//! #[tokio::test]
//! async fn test_gen_input_stream() {
//!     use futures_util::StreamExt;
//!     use r3bl_test_fixtures::gen_input_stream;
//!
//!     let mut input_stream = gen_input_stream(vec![1, 2, 3]);
//!     for _ in 1..=3 {
//!         input_stream.next().await;
//!     }
//!     pretty_assertions::assert_eq!(input_stream.next().await, None);
//! }
//! ```
//!
//! Here's another example of how to use this with a delay.
//!
//! ```
//! #[tokio::test]
//! async fn test_gen_input_stream_with_delay() {
//!     use futures_util::StreamExt;
//!     use r3bl_test_fixtures::gen_input_stream_with_delay;
//!
//!     let delay = 100;
//!
//!     // Start timer.
//!     let start_time = std::time::Instant::now();
//!
//!     let mut input_stream = gen_input_stream_with_delay(vec![1, 2, 3], Duration::from_millis(delay));
//!     for _ in 1..=3 {
//!         input_stream.next().await;
//!     }
//!
//!     // End timer.
//!     let end_time = std::time::Instant::now();
//!
//!     pretty_assertions::assert_eq!(input_stream.next().await, None);
//!
//!     assert!(end_time - start_time >= Duration::from_millis(delay * 3));
//! }
//! ```
//!
//! ## stdout_fixtures
//!
//! Here's an example of how to use this.
//!
//! ```
//! #[tokio::test]
//! async fn test_stdout_mock_no_strip_ansi() {
//!     use strip_ansi_escapes::strip;
//!
//!     use super::*;
//!     use std::{
//!         io::{Result, Write},
//!         sync::Arc,
//!     };
//!
//!     let mut stdout_mock = StdoutMock::default();
//!     let stdout_mock_clone = stdout_mock.clone(); // Points to the same inner value as `stdout_mock`.
//!
//!     let normal_text = "hello world";
//!
//!     stdout_mock.write_all(normal_text.as_bytes()).unwrap();
//!     stdout_mock.flush().unwrap();
//!
//!     pretty_assertions::assert_eq!(stdout_mock.get_copy_of_buffer_as_string(), normal_text);
//!     pretty_assertions::assert_eq!(
//!         stdout_mock_clone.get_copy_of_buffer_as_string(),
//!         normal_text
//!     );
//! }
//! ```

use futures_core::Stream;
use std::pin::Pin;

// Type aliases.
pub type StdMutex<T> = std::sync::Mutex<T>;
pub type PinnedInputStream<T> = Pin<Box<dyn Stream<Item = T>>>;

// Attach sources.
pub mod async_stream_fixtures;
pub mod stdout_fixtures;

// Re-export.
pub use async_stream_fixtures::*;
pub use stdout_fixtures::*;
