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
          fmt::{Display, Result as FmtResult},
          result::Result as OGResult};

/// Type alias to make it easy to work with [`Result`]s. Works hand in hand w/ [CommonError].
/// Here's an example.
/// ```ignore
/// pub fn try_from_pair(pair: Pair) -> CommonResult<(Percent, Percent)> {
///   let first = pair.first.try_into();
///   let second = pair.second.try_into();
///
///   match (first, second) {
///     (Ok(first), Ok(second)) => Ok((first, second)),
///     _ => {
///       let err_msg = format!("Invalid percentage values in tuple: {:?}", pair);
///       CommonError::new(CommonErrorType::ValueOutOfRange, &err_msg)
///     }
///   }
/// }
/// ```
pub type CommonResult<T> = OGResult<T, Box<dyn Error + Send + Sync>>;

/// Common error struct. Read custom error docs
/// [here](https://learning-rust.github.io/docs/e7.custom_error_types.html).
///
/// Here's an example.
/// ```ignore
/// pub fn try_from_pair(pair: Pair) -> CommonResult<(Percent, Percent)> {
///   let first = pair.first.try_into();
///   let second = pair.second.try_into();
///
///   match (first, second) {
///     (Ok(first), Ok(second)) => Ok((first, second)),
///     _ => {
///       let err_msg = format!("Invalid percentage values in tuple: {:?}", pair);
///       CommonError::new(CommonErrorType::ValueOutOfRange, &err_msg)
///     }
///   }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct CommonError {
    pub err_type: CommonErrorType,
    pub err_msg: Option<String>,
}

/// Some common errors that can occur.
#[non_exhaustive]
#[derive(Default, Debug, Clone, Copy)]
pub enum CommonErrorType {
    ExitLoop,
    DisplaySizeTooSmall,
    #[default]
    General,
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
}

/// Implement [`Error`] trait.
impl Error for CommonError {}

/// Implement [`Display`] trait (needed by [`Error`] trait).
impl Display for CommonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> FmtResult { write!(f, "{self:?}") }
}

impl CommonError {
    /// Constructor that is compatible w/ [`CommonResult`].
    #[allow(clippy::all)]
    pub fn new<T>(err_type: CommonErrorType, msg: &str) -> CommonResult<T> {
        Self::from_err(CommonError {
            err_type,
            err_msg: msg.to_string().into(),
        })
    }

    /// Constructor that is compatible w/ [`CommonResult`].
    pub fn new_err_with_only_type<T>(err_type: CommonErrorType) -> CommonResult<T> {
        CommonError::from_err_type_and_msg(err_type, None)
    }

    /// Constructor that is compatible w/ [`CommonResult`].
    pub fn new_err_with_only_msg<T>(msg: &str) -> CommonResult<T> {
        CommonError::from_err_type_and_msg(CommonErrorType::General, Some(msg.to_string()))
    }

    /// Private helper method.
    fn from_err_type_and_msg<T>(err_type: CommonErrorType, msg: Option<String>) -> CommonResult<T> {
        Self::from_err(CommonError {
            err_type,
            err_msg: msg,
        })
    }

    /// Private helper method.
    fn from_err<T>(err: CommonError) -> CommonResult<T> { Err(Box::new(err)) }
}
