/*
 *   Copyright (c) 2024 R3BL LLC
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

/// This is the default capacity for a new vector.
pub const DEFAULT_VEC_CAPACITY: usize = 32;

pub const DEFAULT_NORMAL_STRING_SIZE: usize = 96;

pub const DEFAULT_LARGE_STRING_SIZE: usize = 128;

pub const DEFAULT_SMALL_STRING_SIZE: usize = 32;

pub const DEFAULT_TINY_STRING_SIZE: usize = 8;

pub const DEFAULT_MICRO_STRING_SIZE: usize = 4;

/// This is similar to [DEFAULT_VEC_CAPACITY], but for pre-allocated vectors on the stack.
pub const DEFAULT_SMALL_VEC_SIZE: usize = 32;

/// This is copied in other crates: `r3bl_analytics_schema`, `r3bl_ansi_color`.
pub const DEFAULT_TINY_VEC_SIZE: usize = 16;

pub const DEFAULT_MICRO_VEC_SIZE: usize = 8;
