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

use smallstr::SmallString;
use smallvec::SmallVec;

use super::{DEFAULT_TINY_STRING_SIZE, DEFAULT_TINY_VEC_SIZE};
use crate::{DEFAULT_LARGE_STRING_SIZE,
            DEFAULT_MICRO_STRING_SIZE,
            DEFAULT_MICRO_VEC_SIZE,
            DEFAULT_NORMAL_STRING_SIZE,
            DEFAULT_SMALL_STRING_SIZE,
            DEFAULT_SMALL_VEC_SIZE};

pub type SmallStringBackingStore = SmallString<[u8; DEFAULT_SMALL_STRING_SIZE]>;

pub type LargeStringBackingStore = SmallString<[u8; DEFAULT_LARGE_STRING_SIZE]>;

pub type NormalStringBackingStore = SmallString<[u8; DEFAULT_NORMAL_STRING_SIZE]>;

pub type TinyStringBackingStore = SmallString<[u8; DEFAULT_TINY_STRING_SIZE]>;

pub type MicroStringBackingStore = SmallString<[u8; DEFAULT_MICRO_STRING_SIZE]>;

pub type SmallVecBackingStore<T> = SmallVec<[T; DEFAULT_SMALL_VEC_SIZE]>;

/// This is copied in other crates: `r3bl_analytics_schema`, `r3bl_ansi_color`.
pub type TinyVecBackingStore<T> = SmallVec<[T; DEFAULT_TINY_VEC_SIZE]>;

pub type MicroVecBackingStore<T> = SmallVec<[T; DEFAULT_MICRO_VEC_SIZE]>;
