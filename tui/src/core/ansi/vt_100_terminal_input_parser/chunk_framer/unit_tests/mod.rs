// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Unit test suite for [`ChunkFramer`].
//!
//! [`ChunkFramer`]: super::ChunkFramer

// Attach. Crate-wide (Intra-crate).
pub(crate) mod osc_and_unrecognized_tests; // Cross-folder backend_compat link.

// Attach. Folder-local (Intra-module).
mod basic_and_special_keys_tests;
mod esc_and_chunked_tests;
mod test_fixtures;
mod utf8_tests;
