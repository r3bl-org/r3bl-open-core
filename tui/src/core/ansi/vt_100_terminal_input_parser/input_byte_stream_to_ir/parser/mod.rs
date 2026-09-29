// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Stateful parser for terminal input bytes. See [`InputByteStreamToIrParser`].

// Attach source files.
mod classification;
mod constants;
mod parser_struct;

#[cfg(any(test, doc))]
pub mod unit_tests;

// Public re-exports (barrel export pattern).
pub use classification::*;
pub use constants::*;
pub use parser_struct::*;
