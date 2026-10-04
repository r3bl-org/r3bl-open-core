// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Unit test suite for [`InputByteStreamToIrParser`].
//!
//! [`InputByteStreamToIrParser`]: super::InputByteStreamToIrParser

pub mod test_fixtures;

mod basic_and_special_keys_tests;
mod esc_and_chunked_tests;
mod osc_and_unrecognized_tests;
mod utf8_tests;

pub use test_fixtures::*;
