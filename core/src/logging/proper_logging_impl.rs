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

//! # How to log things, and simply use the logging facilities
//!
//! The simplest way to use this crate is to look at the [try_to_set_log_level] function
//! as the main entry point. By default, logging is disabled even if you call all the
//! functions in this module: [log_debug], [log_info], [log_trace], etc.
//!
//! Note that, although read and write methods require a `&mut File`, because of the
//! interfaces for `Read` and `Write`, the holder of a `&File` can still modify the file,
//! either through methods that take `&File` or by retrieving the underlying OS object and
//! modifying the file that way. Additionally, many operating systems allow concurrent
//! modification of files by different processes. Avoid assuming that holding a `&File`
//! means that the file will not change.
//!
//! # How to change how logging is implemented under the hood
//!
//! Under the hood the [`simplelog`](https://crates.io/crates/simplelog) crate is forked
//! and modified for use in the
//! [r3bl_simple_logger](https://crates.io/crates/r3bl_simple_logger) crate. For people
//! who want to work on changing the underlying behavior of the logging engine itself it
//! is best to look in that crate and make changes there.
//!

use std::{fs::File, sync::Once};

use chrono::Local;
use r3bl_simple_logger::*;
use time::UtcOffset;

use crate::*;

pub static mut LOG_LEVEL: LevelFilter = LevelFilter::Off;
pub static mut FILE_PATH: &str = "log.txt";
static mut FILE_LOGGER_INIT_OK: bool = false;
static FILE_LOGGER_INIT_FN: Once = Once::new();

const ENABLE_MULTITHREADED_LOG_WRITING: bool = false;

/// If you don't call this function w/ a value other than [LevelFilter::Off], then logging
/// is **DISABLED** by **default**. It won't matter if you call any of the other logging
/// functions in this module.
///
/// It does not matter how many times you call this function, it will only set the log
/// level once. If you want to change the default file that is used for logging take a
/// look at [try_to_set_log_file_path].
///
/// If you want to override the default log level [LOG_LEVEL], you can use this function. If the
/// logger has already been initialized, then it will return a [CommonErrorType::InvalidState]
/// error. To disable logging simply set the log level to [LevelFilter::Off].
///
/// If you would like to ignore the error just call `ok()` on the result that's returned. [More
/// info](https://doc.rust-lang.org/std/result/enum.Result.html#method.ok).
pub fn try_to_set_log_level(level: LevelFilter) -> CommonResult<String> {
    unsafe {
        match FILE_LOGGER_INIT_OK {
            true => CommonError::new(
                CommonErrorType::InvalidState,
                "Logger already initialized, can't set log level",
            ),
            false => {
                LOG_LEVEL = level;
                Ok(level.to_string())
            }
        }
    }
}

/// Please take a look at [try_to_set_log_level] to enable or disable logging.
///
/// If you want to override the default log file path (stored in [FILE_PATH]), you can use this
/// function. If the logger has already been initialized, then it will return a
/// [CommonErrorType::InvalidState] error.
///
/// If you would like to ignore the error just call `ok()` on the result that's returned. [More
/// info](https://doc.rust-lang.org/std/result/enum.Result.html#method.ok).
pub fn try_to_set_log_file_path(path: &'static str) -> CommonResult<String> {
    unsafe {
        match FILE_LOGGER_INIT_OK {
            true => CommonError::new(
                CommonErrorType::InvalidState,
                "Logger already initialized, can't set log file path",
            ),
            false => {
                FILE_PATH = path;
                Ok(path.to_string())
            }
        }
    }
}

/// Please take a look at [try_to_set_log_level] to enable or disable logging.
///
/// Log the message to the `INFO` log level using a file logger. There could be issues w/ accessing
/// this file; if it fails this function will not propagate the log error.
pub fn log_info(arg: String) {
    if init_file_logger_once().is_err() {
        eprintln!(
            "Error initializing file logger due to {}",
            init_file_logger_once().unwrap_err()
        );
    } else {
        match ENABLE_MULTITHREADED_LOG_WRITING {
            true => {
                std::thread::spawn(move || {
                    log::info!("{}", arg);
                });
            }
            false => {
                log::info!("{}", arg);
            }
        }
    }
}

/// Please take a look at [try_to_set_log_level] to enable or disable logging.
///
/// Log the message to the `DEBUG` log level using a file logger. There could be issues w/ accessing
/// this file; if it fails this function will not propagate the log error.
pub fn log_debug(arg: String) {
    if init_file_logger_once().is_err() {
        eprintln!(
            "Error initializing file logger due to {}",
            init_file_logger_once().unwrap_err()
        );
    } else {
        match ENABLE_MULTITHREADED_LOG_WRITING {
            true => {
                std::thread::spawn(move || {
                    log::debug!("{}", arg);
                });
            }
            false => {
                log::debug!("{}", arg);
            }
        }
    }
}

/// Please take a look at [try_to_set_log_level] to enable or disable logging.
///
/// Log the message to the `WARN` log level using a file logger. There could be issues w/ accessing
/// this file; if it fails this function will not propagate the log error.
pub fn log_warn(arg: String) {
    if init_file_logger_once().is_err() {
        eprintln!(
            "Error initializing file logger due to {}",
            init_file_logger_once().unwrap_err()
        );
    } else {
        match ENABLE_MULTITHREADED_LOG_WRITING {
            true => {
                std::thread::spawn(move || {
                    log::warn!("{}", arg);
                });
            }
            false => {
                log::warn!("{}", arg);
            }
        }
    }
}

/// Please take a look at [try_to_set_log_level] to enable or disable logging.
///
/// Log the message to the `TRACE` log level using a file logger. There could be issues w/ accessing
/// this file; if it fails this function will not propagate the log error.
pub fn log_trace(arg: String) {
    if init_file_logger_once().is_err() {
        eprintln!(
            "Error initializing file logger due to {}",
            init_file_logger_once().unwrap_err()
        );
    } else {
        match ENABLE_MULTITHREADED_LOG_WRITING {
            true => {
                std::thread::spawn(move || {
                    log::trace!("{}", arg);
                });
            }
            false => {
                log::trace!("{}", arg);
            }
        }
    }
}

/// Please take a look at [try_to_set_log_level] to enable or disable logging.
///
/// Log the message to the `ERROR` log level using a file logger. There could be issues w/ accessing
/// this file; if it fails this function will not propagate the log error.
pub fn log_error(arg: String) {
    if init_file_logger_once().is_err() {
        eprintln!(
            "Error initializing file logger due to {}",
            init_file_logger_once().unwrap_err()
        );
    } else {
        match ENABLE_MULTITHREADED_LOG_WRITING {
            true => {
                std::thread::spawn(move || {
                    log::error!("{}", arg);
                });
            }
            false => {
                log::error!("{}", arg);
            }
        }
    }
}

/// Simply open the file (location stored in [FILE_PATH] static above) and write the log message to
/// it. This will be opened once per session (i.e. program execution). It is destructively opened,
/// meaning that it will be rewritten when used in the next session.
///
/// # Docs
///
/// - Log
///   - [`CombinedLogger`], [`WriteLogger`], [`ConfigBuilder`]
/// - `format_description!`: <https://time-rs.github.io/book/api/format-description.html>
#[allow(static_mut_refs)]
fn init_file_logger_once() -> CommonResult<()> {
    unsafe {
        if LOG_LEVEL == LevelFilter::Off {
            FILE_LOGGER_INIT_OK = true;
            return Ok(());
        }
    }

    // Run the lambda once & save bool to static `FILE_LOGGER_INIT_OK`.
    FILE_LOGGER_INIT_FN.call_once(actually_init_file_logger);

    // Use saved bool in static `FILE_LOGGER_INIT_OK` to throw error if needed.
    unsafe {
        return match FILE_LOGGER_INIT_OK {
            true => Ok(()),
            false => {
                let msg = format!("Failed to initialize file logger {FILE_PATH}");
                return CommonError::new(CommonErrorType::IOError, &msg);
            }
        };
    }

    /// [FILE_LOGGER_INIT_OK] is `false` at the start. Only this function (if it succeeds) can set
    /// that to `true`. This function does *not* panic if there's a problem initializing the logger.
    /// It just prints a message to stderr & returns.
    fn actually_init_file_logger() {
        unsafe {
            let maybe_new_file = File::create(FILE_PATH);
            if let Ok(new_file) = maybe_new_file {
                let config = new_config();
                let level = LOG_LEVEL;
                let file_logger = WriteLogger::new(level, config, new_file);
                let maybe_logger_init_err = CombinedLogger::init(vec![file_logger]);
                if let Err(e) = maybe_logger_init_err {
                    eprintln!("Failed to initialize file logger {FILE_PATH} due to {e}");
                } else {
                    FILE_LOGGER_INIT_OK = true
                }
            }
        }
    }

    /// Try to make a [`Config`] with local timezone offset. If that fails, return a default. The
    /// implementation used here works w/ Tokio.
    fn new_config() -> Config {
        let mut builder = ConfigBuilder::new();

        let offset_in_sec = Local::now().offset().local_minus_utc();
        let utc_offset_result = UtcOffset::from_whole_seconds(offset_in_sec);
        if let Ok(utc_offset) = utc_offset_result {
            builder.set_time_offset(utc_offset);
        }

        builder.set_time_format_custom(format_description!(
            "[hour repr:12]:[minute] [period]"
        ));

        builder.build()
    }
}
