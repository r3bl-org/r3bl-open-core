// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! System-level [`PTY`] tests - end-to-end validation of spinners in real pseudoterminals.
//!
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal

#![rustfmt::skip]

#[cfg(any(all(unix, doc), test))]
pub mod pty_spinner_test;
