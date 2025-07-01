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

use std::{io::Error, pin::Pin, sync::Arc};

use crossterm::event::Event;
use futures_core::Stream;

/// Disambiguate the type of `StdMutex` from stdlib and tokio to avoid conflicts.
pub type StdMutex<T> = std::sync::Mutex<T>;

/// Type alias for a `Send`-able output device (raw terminal, `SharedWriter`, etc).
pub type SendRawTerminal = dyn std::io::Write + Send;
/// Type alias for a `Send`-able raw terminal wrapped in an `Arc<StdMutex>`.
pub type SafeRawTerminal = Arc<StdMutex<SendRawTerminal>>;

/// Type alias for crossterm streaming (input) event result.
pub type CrosstermEventResult = Result<Event, Error>;
/// Type alias for a pinned stream that is async safe. `T` is usually
/// [`CrosstermEventResult`].
pub type PinnedInputStream<T> = Pin<Box<dyn Stream<Item = T>>>;
