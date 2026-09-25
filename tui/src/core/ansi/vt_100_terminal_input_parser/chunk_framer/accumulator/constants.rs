// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Pre-allocated buffer capacities and safety thresholds for
//! [`ChunkFramer`].
//!
//! [`ChunkFramer`]: super::super::ChunkFramer

/// Initial pre-allocated byte capacity for [`ChunkFramer`]'s accumulator
/// buffer.
///
/// [`ChunkFramer`]: super::super::ChunkFramer
pub const ACCUMULATOR_INITIAL_CAPACITY: usize = 256;

/// Initial pre-allocated capacity for [`ChunkFramer`]'s internal event
/// queue.
///
/// [`ChunkFramer`]: super::super::ChunkFramer
pub const INTERNAL_EVENTS_INITIAL_CAPACITY: usize = 128;

/// Safety maximum byte length for an accumulated unparsed escape sequence.
///
/// Valid [`ANSI`]/[`CSI`]/[`SS3`] keyboard sequences rarely exceed 6-10 bytes, and mouse
/// tracking sequences rarely exceed 12-16 bytes. A length threshold of `64` provides an
/// abundant safety margin while preventing unbounded memory growth or permanent input
/// freeze if corrupted or malformed byte streams never terminate.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`CSI`]: crate::CsiSequence
/// [`SS3`]: https://en.wikipedia.org/wiki/ANSI_escape_code#SS3
pub const MAX_ESCAPE_SEQUENCE_LENGTH: usize = 64;
