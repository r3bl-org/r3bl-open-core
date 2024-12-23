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

use std::fmt::{self, Debug};

use serde::{Deserialize, Serialize};

use super::Percent;

/// Represents a percentage value for the requested size. It is used to calculate the
/// requested size as a percentage of the parent size.
///
/// # How to use it
///
/// You can create it either of the following ways:
/// 1. Use the [crate::requested_size_percent!] macro. It uses the [crate::percent!] macro
///    to do the [crate::Percent] conversion. Make sure to call this macro from a block
///    that returns a `Result` type, since the `?` operator is used here.
/// 2. Directly create it using the [RequestedSizePercent] struct with [crate::Percent]
///    values.
///
/// Note that [crate::Size], defined as:
/// - height or [crate::Size::row_count],
/// - width or [crate::Size::col_count].
#[derive(Copy, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct RequestedSizePercent {
    pub width_pc: Percent,
    pub height_pc: Percent,
}

/// This must be called from a block that returns a `Result` type. Since the `?` operator
/// is used here.
#[macro_export]
macro_rules! requested_size_percent {
    (
        width:  $arg_width: expr,
        height: $arg_height: expr
    ) => {
        $crate::RequestedSizePercent {
            width_pc: $crate::percent!($arg_width)?,
            height_pc: $crate::percent!($arg_height)?,
        }
    };
}

impl Debug for RequestedSizePercent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[width:{}, height:{}]", self.width_pc, self.height_pc)
    }
}
