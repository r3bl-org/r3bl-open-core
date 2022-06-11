/*
 *   Copyright (c) 2022 R3BL LLC
 *   All rights reserved.

 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at

 *   http://www.apache.org/licenses/LICENSE-2.0

 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
*/

use crate::*;
use chrono::Local;
use log::LevelFilter;
use simplelog::*;
use std::fs::File;
use std::sync::Once;
use time::UtcOffset;

static mut FILE_PATH: &str = "log.txt";
static mut FILE_LOGGER_INIT_OK: bool = false;
static FILE_LOGGER_INIT_FN: Once = Once::new();

/// # Example
///
/// ```ignore
/// use r3bl_rs_utils::{init_file_logger_once, log, ResultCommon};
/// fn run() -> ResultCommon<()> {
///   let msg = "foo";
///   let msg_2 = "bar";
///   log!(INFO, "This is a info message");
///   log!(WARN, "This is a warning message {}", msg);
///   log!(ERROR, "This is a error message {} {}", msg, msg_2);
///   Ok(())
/// }
/// ```
///
/// # Docs for log crate
///
/// - [`log::info!`], [`log::warn!`], [`log::error!`]: https://docs.rs/log/latest/log/
#[macro_export]
macro_rules! log {
  (INFO, $($arg:tt)*) => {{
    init_file_logger_once()?;
    log::info!($($arg)*);
  }};
  (WARN, $($arg:tt)*) => {{
    init_file_logger_once()?;
    log::warn!($($arg)*);
  }};
  (ERROR, $($arg:tt)*) => {{
    init_file_logger_once()?;
    log::error!($($arg)*);
  }};
}

/// If you want to override the default log file path, you can use this function. If the
/// logger has already been initialized, then it will return a
/// [CommonErrorType::InvalidState] error.
pub fn try_to_set_log_file_path(path: &'static str) -> CommonResult<String> {
  unsafe {
    return match FILE_LOGGER_INIT_OK {
      true => CommonError::new_err_with_only_type(CommonErrorType::InvalidState),
      false => {
        FILE_PATH = path;
        Ok(path.to_string())
      }
    };
  }
}

/// This is very similar to [log!] except that if it fails, it will not propagate the log error.
#[macro_export]
macro_rules! log_no_err {
  (INFO, $($arg:tt)*) => {{
    if init_file_logger_once().is_err() {
      eprintln!("Error initializing file logger due to {}", init_file_logger_once().unwrap_err());
    } else {
      log::info!($($arg)*);
    }
  }};
  (WARN, $($arg:tt)*) => {{
    if init_file_logger_once().is_err() {
      eprintln!("Error initializing file logger due to {}", init_file_logger_once().unwrap_err());
    } else {
      log::warn!($($arg)*);
    }
  }};
  (ERROR, $($arg:tt)*) => {{
    if init_file_logger_once().is_err() {
      eprintln!("Error initializing file logger due to {}", init_file_logger_once().unwrap_err());
  } else {
      log::error!($($arg)*);
    }
  }};
}

/// Simply open the [`FILE_PATH`] file and write the log message to it. This will be
/// opened once per session (i.e. program execution). It is destructively opened, meaning
/// that it will be rewritten when used in the next session.
///
/// # Docs
/// - [`CombinedLogger`], [`WriteLogger`], [`ConfigBuilder`]: https://github.com/drakulix/simplelog.rs
/// - [`format_description!`]: https://time-rs.github.io/book/api/format-description.html
pub fn init_file_logger_once() -> CommonResult<()> {
  // Run the lambda once & save bool to static `FILE_LOGGER_INIT_OK`.
  FILE_LOGGER_INIT_FN.call_once(|| actually_init_file_logger());

  // Use saved bool in static `FILE_LOGGER_INIT_OK` to throw error if needed.
  unsafe {
    return match FILE_LOGGER_INIT_OK {
      true => Ok(()),
      false => {
        let msg = format!("Failed to initialize file logger {}", FILE_PATH);
        return CommonError::new(CommonErrorType::IOError, &msg);
      }
    };
  }

  /// [FILE_LOGGER_INIT_OK] is `false` at the start. Only this function (if it succeeds) can
  /// set that to `true`. This function does *not* panic if there's a problem
  /// initializing the logger. It just prints a message to stderr & returns.
  fn actually_init_file_logger() {
    unsafe {
      let maybe_new_file = File::create(FILE_PATH);
      if let Ok(new_file) = maybe_new_file {
        let config = new_config();
        let level = LevelFilter::Info;
        let file_logger = WriteLogger::new(level, config, new_file);
        let maybe_logger_init_err = CombinedLogger::init(vec![file_logger]);
        if let Err(e) = maybe_logger_init_err {
          eprintln!("Failed to initialize file logger {} due to {}", FILE_PATH, e);
        } else {
          FILE_LOGGER_INIT_OK = true
        }
      }
    }
  }

  /// Try to make a [`Config`] with local timezone offset. If that fails, return a default.
  /// The implementation used here works w/ Tokio.
  fn new_config() -> Config {
    let mut builder = ConfigBuilder::new();

    let offset_in_sec = Local::now().offset().local_minus_utc();
    let utc_offset_result = UtcOffset::from_whole_seconds(offset_in_sec);
    if let Ok(utc_offset) = utc_offset_result {
      builder.set_time_offset(utc_offset);
    }

    builder.set_time_format_custom(format_description!("[hour repr:12]:[minute] [period]"));

    builder.build()
  }
}
