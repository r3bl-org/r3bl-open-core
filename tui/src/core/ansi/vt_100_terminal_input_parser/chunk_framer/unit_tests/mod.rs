// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Unit test suite for [`ChunkFramer`].
//!
//! [`ChunkFramer`]: super::ChunkFramer

pub mod test_fixtures;

pub mod basic_and_special_keys_tests;
pub mod esc_and_chunked_tests;
pub mod osc_and_unrecognized_tests;
pub mod utf8_tests;

pub use test_fixtures::*;

