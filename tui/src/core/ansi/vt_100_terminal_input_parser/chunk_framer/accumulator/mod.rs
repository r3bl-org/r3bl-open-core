// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Stateful accumulator buffer and sequence classification engine for terminal input
//! bytes. See [`ChunkAccumulator`].

// Attach.
pub(super) mod constants;

mod accumulator_struct;
mod unparsed_buffer_action;

// Re-export.
pub use accumulator_struct::*;
pub use constants::*;
pub use unparsed_buffer_action::*;
