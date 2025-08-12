// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::{error::Error,
          fmt::{Debug, Display, Result}};

use super::FlexBox;
use crate::CommonResult;

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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result { Debug::fmt(self, f) }
}

/// Implement constructor that is compatible w/ [`CommonResult<T>`].
impl LayoutError {
    /// Only [`LayoutError::error_type`] available, and no [`LayoutError::error_message`].
    ///
    /// # Errors
    ///
    /// Always returns an error with the given error type.
    pub fn new_error_result_with_only_type<T>(
        err_type: LayoutErrorType,
    ) -> CommonResult<T> {
        core::result::Result::Err(miette::miette!(LayoutError {
            error_type: err_type,
            error_message: None,
        }))
    }

    /// Both [`LayoutError::error_type`] and [`LayoutError::error_message`] available.
    ///
    /// # Errors
    ///
    /// Always returns an error with the given error type and message.
    pub fn new_error_result<T>(
        err_type: LayoutErrorType,
        msg: String,
    ) -> CommonResult<T> {
        core::result::Result::Err(miette::miette!(LayoutError {
            error_type: err_type,
            error_message: Some(msg),
        }))
    }

    #[must_use]
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
