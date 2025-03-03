/*
 *   Copyright (c) 2022-2025 R3BL LLC
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

//! For more information on error types, see:
//!
//! 1. [Article](https://developerlife.com/2024/06/10/rust-miette-error-handling/)
//! 2. [Video](https://youtu.be/TmLF7vI8lKk)

use std::{error::Error,
          fmt::{Debug, Display, Formatter, Result}};

/// Type alias to make it easy to work with:
/// 1. [`core::result::Result`]
/// 2. [miette::Result] and [miette::Report], which are [std::error::Error] wrappers.
///
/// - It is basically `miette::Result<T, miette::Report>`.
/// - Works hand in hand w/ [CommonError] and any other type of error.
///
/// # Example
///
/// ```
/// use r3bl_core::{CommonResult, CommonError, CommonErrorType, Pc};
/// pub fn try_from_pair(pair: (i32, i32)) -> CommonResult<(Pc, Pc)> {
///   let first = pair.0.try_into();
///   let second = pair.0.try_into();
///
///   match (first, second) {
///     (Ok(first), Ok(second)) => Ok((first, second)),
///     _ => {
///       let err_msg = format!("Invalid Pcage values in tuple: {:?}", pair);
///       CommonError::new_error_result(CommonErrorType::ValueOutOfRange, &err_msg)
///     }
///   }
/// }
/// ```
pub type CommonResult<T> = miette::Result<T>;

/// Common error struct. Read custom error docs
/// [here](https://learning-rust.github.io/docs/e7.custom_error_types.html).
///
/// # Example
///
/// ```
/// use r3bl_core::{CommonResult, CommonError, CommonErrorType, Pc};
/// pub fn try_from_pair(pair: (i32, i32)) -> CommonResult<(Pc, Pc)> {
///   let first = pair.0.try_into();
///   let second = pair.1.try_into();
///
///   match (first, second) {
///     (Ok(first), Ok(second)) => Ok((first, second)),
///     _ => {
///       let err_msg = format!("Invalid Pcage values in tuple: {:?}", pair);
///       CommonError::new_error_result(CommonErrorType::ValueOutOfRange, &err_msg)
///     }
///   }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct CommonError {
    pub error_type: CommonErrorType,
    pub error_message: Option<String>,
}

/// Some common errors that can occur.
#[non_exhaustive]
#[derive(Default, Debug, Clone, Copy)]
pub enum CommonErrorType {
    #[default]
    General,
    ExitLoop,
    DisplaySizeTooSmall,
    InvalidArguments,
    InvalidResult,
    InvalidState,
    StackOverflow,
    StackUnderflow,
    ParsingError,
    IOError,
    ValueOutOfRange,
    InvalidValue,
    DoesNotApply,
    IndexOutOfBounds,
    InvalidRgbColor,
    InvalidHexColorFormat,
    NotFound,
    CommandExecutionError,
    ConfigFolderCountNotBeCreated,
    ConfigFolderPathCouldNotBeGenerated,
}

/// Implement [`Error`] trait.
impl Error for CommonError {}

/// Implement [`Display`] trait (needed by [`Error`] trait). This is the same as the
/// [`Debug`] implementation (which is derived above).
impl Display for CommonError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { Debug::fmt(self, f) }
}

impl CommonError {
    /// Both [CommonError::error_type] and [CommonError::error_message] available.
    #[allow(clippy::all)]
    pub fn new_error_result<T>(err_type: CommonErrorType, msg: &str) -> CommonResult<T> {
        Err(miette::miette!(CommonError {
            error_type: err_type,
            error_message: Some(msg.to_string()),
        }))
    }

    /// Only [CommonError::error_type] available, and no [CommonError::error_message].
    pub fn new_error_result_with_only_type<T>(
        err_type: CommonErrorType,
    ) -> CommonResult<T> {
        Err(miette::miette!(CommonError {
            error_type: err_type,
            error_message: None,
        }))
    }

    /// Only [CommonError::error_message] available, and no [CommonError::error_type].
    pub fn new_error_result_with_only_msg<T>(msg: &str) -> CommonResult<T> {
        Err(miette::miette!(CommonError {
            error_type: CommonErrorType::default(),
            error_message: Some(msg.to_string()),
        }))
    }
}
