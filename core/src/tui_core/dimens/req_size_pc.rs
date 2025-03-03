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

use super::Pc;

/// Represents a percentage value for the requested size. It is used to calculate the
/// requested size as a percentage of the parent size.
///
/// # How to use it
///
/// You can create it either of the following ways:
/// 1. Use the [crate::req_size_pc!] macro. It uses the [crate::pc!] macro to do
///    the [crate::Pc] conversion. Make sure to call this macro from a block that
///    returns a [Result] type, since the `?` operator is used here.
/// 2. Directly create it using the [ReqSizePc] struct with [crate::Pc] values.
///
/// Note that [crate::Dim], defined as:
/// - height or [crate::Dim::row_height],
/// - width or [crate::Dim::col_width].
#[derive(Copy, Clone, Default, PartialEq, Eq, Hash)]
pub struct ReqSizePc {
    pub width_pc: Pc,
    pub height_pc: Pc,
}

/// This must be called from a block that returns a [Result] type. Since the `?` operator
/// is used here.
#[macro_export]
macro_rules! req_size_pc {
    (
        width:  $arg_width: expr,
        height: $arg_height: expr
    ) => {
        $crate::ReqSizePc {
            width_pc: $crate::pc!($arg_width)?,
            height_pc: $crate::pc!($arg_height)?,
        }
    };
}

impl Debug for ReqSizePc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[width:{w:?}, height:{h:?}]",
            w = self.width_pc,
            h = self.height_pc
        )
    }
}
