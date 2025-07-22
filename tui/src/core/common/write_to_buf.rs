/*
 *   Copyright (c) 2023-2025 R3BL LLC
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

use std::fmt::{Formatter, Result};

/// Buffer for building text efficiently.
///
/// We use `String` as the backing storage after performance testing showed:
/// - `SmallString<[u8; 64]>` had slightly worse performance due to stack allocation
///   overhead.
/// - `SmallString<[u8; 256]>` had even worse performance for small strings.
/// - Plain `String` provides the best balance of performance across all test cases.
///
/// This type alias allows us to easily experiment with different string-like data
/// structures in the future (e.g., `SmallString`, `String`, custom implementations)
/// without impacting the rest of the codebase.
pub type BufTextStorage = String;

/// Trait for efficiently writing text to a buffer.
///
/// ## Why `WriteToBuf` instead of Display/Formatter?
///
/// The standard [`std::fmt::Display`] trait uses [`std::fmt::Formatter`] which has
/// significant overhead:
/// 1. **Formatter State Machine**: Each [`write!`] call goes through the formatter's
///    internal state machine, checking formatting flags (alignment, padding, precision,
///    etc).
/// 2. **Multiple Function Calls**: Each [`write!`] has method call overhead, vtable
///    lookups for trait objects, and repeated bounds checking.
/// 3. **Buffer Management**: The formatter may need to reallocate its internal buffer
///    multiple times for many small writes.
///
/// By using `WriteToBuf` with a [`BufTextStorage`] buffer, we:
/// - Make direct string concatenations without formatter overhead.
/// - Batch all content into a single buffer.
/// - Make only ONE write to the formatter in the Display implementation using
///   [`core::fmt::Formatter::write_str`].
/// - Reduce the overhead from ~16% to ~5-8% in performance profiles.
///
/// The [`std::fmt::Display`] trait implementations still exist for API compatibility but
/// delegate to `WriteToBuf`. The [`std::fmt::Display`] trait will have to use use
/// [`core::fmt::Formatter::write_str`] to actually write the `acc` buffer.
pub trait WriteToBuf {
    /// Write the formatted representation to the provided buffer. You might want to
    /// call [`WriteToBuf::write_buf_to_fmt()`] when you are ready to actually write the
    /// buffer to the formatter if you are implementing the [`std::fmt::Display`] trait.
    ///
    /// # Errors
    ///
    /// Returns an error if the formatting operation fails.
    fn write_to_buf(&self, acc: &mut BufTextStorage) -> Result;

    /// Use [`core::fmt::Formatter::write_str`] to actually write the `acc` buffer when
    /// implementing the [`std::fmt::Display`] trait.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the formatter fails.
    fn write_buf_to_fmt(&self, acc: &BufTextStorage, f: &mut Formatter<'_>) -> Result {
        f.write_str(acc)
    }
}
