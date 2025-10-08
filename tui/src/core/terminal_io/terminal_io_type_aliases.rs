// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crossterm::event::Event;
use futures_core::Stream;
use std::{io::Error, pin::Pin, sync::Arc};

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
