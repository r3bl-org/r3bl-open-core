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

use std::{error::Error,
          fmt::{Display, Result}};

use r3bl_core::CommonResult;

use super::FlexBox;

/// Main error struct.
/// <https://learning-rust.github.io/docs/e7.custom_error_types.html>
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LayoutError {
    pub error_type: LayoutErrorType,
    pub error_message: Option<String>,
}

/// Specific types of errors.
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub enum LayoutErrorType {
    MismatchedSurfaceEnd,
    MismatchedSurfaceStart,
    MismatchedBoxEnd,
    StackOfBoxesShouldNotBeEmpty,
    InvalidSizePercentage,
    ErrorCalculatingNextBoxPos,
    ContainerBoxBoundsUndefined,
    BoxCursorPositionUndefined,
    ContentCursorPositionUndefined,
}

/// Implement [`Error`] trait.
impl Error for LayoutError {}

/// Implement [`Display`] trait (needed by [`Error`] trait).
impl Display for LayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result { write!(f, "{self:?}") }
}

/// Implement constructor that is compatible w/ [`CommonResult<T>`].
impl LayoutError {
    /// Only [LayoutError::error_type] available, and no [LayoutError::error_message].
    pub fn new_error_result_with_only_type<T>(
        err_type: LayoutErrorType,
    ) -> CommonResult<T> {
        core::result::Result::Err(miette::miette!(LayoutError {
            error_type: err_type,
            error_message: None,
        }))
    }

    /// Both [LayoutError::error_type] and [LayoutError::error_message] available.
    pub fn new_error_result<T>(
        err_type: LayoutErrorType,
        msg: String,
    ) -> CommonResult<T> {
        core::result::Result::Err(miette::miette!(LayoutError {
            error_type: err_type,
            error_message: Some(msg),
        }))
    }

    pub fn format_msg_with_stack_len(stack_of_boxes: &[FlexBox], msg: &str) -> String {
        format!("{msg}, stack_of_boxes.len(): {}", stack_of_boxes.len())
    }
}

/// Unwrap the `$option`, and if `None` then return the given `$err_type`.
/// Otherwise return the unwrapped `$option`. This macro must be called in a
/// block that returns a `CommonResult<T>`.
#[macro_export]
macro_rules! unwrap_or_err {
    ($option:expr, $err_type:expr) => {
        match $option {
            Some(value) => value,
            None => return $crate::LayoutError::new_error_result_with_only_type($err_type),
        }
    };

    ($option:expr, $err_type:expr, $msg:expr) => {
        match $option {
            Some(value) => value,
            None => return $crate::LayoutError::new_error_result($err_type, $msg.to_string()),
        }
    };

    ($option:expr, $err_type:expr, $msg:expr, $($arg:tt)*) => {
        match $option {
            Some(value) => value,
            None => return $crate::LayoutError::new_error_result($err_type, format!($msg, $($arg)*)),
        }
    };
}
