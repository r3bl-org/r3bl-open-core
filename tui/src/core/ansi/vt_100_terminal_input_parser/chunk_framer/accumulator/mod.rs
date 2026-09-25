// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Stateful accumulator buffer and sequence classification engine for terminal input
//! bytes. See [`ChunkAccumulator`].

// Attach source files.
mod accumulator_struct;

#[cfg(any(test, doc))]
pub mod unparsed_buffer_action;
#[cfg(not(any(test, doc)))]
mod unparsed_buffer_action;

pub mod constants;

// Public re-exports (barrel export pattern).
pub use accumulator_struct::*;
pub use constants::*;
pub use unparsed_buffer_action::*;
