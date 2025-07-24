/*
 *   Copyright (c) 2025 R3BL LLC
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

// Attach.
pub mod bounds_check;
pub mod byte_index;
pub mod ch_unit;
pub mod index;
pub mod length;
pub mod unit_check_overflows; // Don't re-export.

// Re-export.
pub use bounds_check::*;
pub use byte_index::*;
pub use ch_unit::*;
pub use index::*;
pub use length::*;
