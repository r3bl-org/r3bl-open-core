/*
 *   Copyright (c) 2022 R3BL LLC
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

//! This module contains a lot of utility functions that are meant to:
//! 1. Increase the ergonomics of using wrapped values in Rust
//! 2. Colorizing console output.
//! 3. Easy to work w/ lazy hash maps.
//! 4. Easy to work w/ readline.
//! 5. Interrogation of types.

// Attach sources.
pub mod lazy_field;
pub mod lazy_hash_map;
pub mod safe_unwrap;
pub mod type_utils;

// Re-export.
pub use lazy_field::*;
pub use lazy_hash_map::*;
pub use safe_unwrap::*;
pub use type_utils::*;
