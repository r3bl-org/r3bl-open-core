// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::fmt::{self, Debug};

use super::Pc;

/// Represents a percentage value for the requested size. It is used to calculate the
/// requested size as a percentage of the parent size.
///
/// # How to use it
///
/// You can create it either of the following ways:
/// 1. Use the [`crate::req_size_pc`!] macro. It uses the [`crate::pc`!] macro to do the
///    [`crate::Pc`] conversion. Make sure to call this macro from a block that returns a
///    [Result] type, since the `?` operator is used here.
/// 2. Directly create it using the [`ReqSizePc`] struct with [`crate::Pc`] values.
///
/// Note that [`crate::Size`], defined as:
/// - height or [`crate::Size::row_height`],
/// - width or [`crate::Size::col_width`].
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
