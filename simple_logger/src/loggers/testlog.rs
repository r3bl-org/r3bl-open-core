/*
 *   Copyright (c) 2023 Nazmul Idris
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

//! Module providing the TestLogger Implementation

use super::logging::should_skip;
use crate::{config::TimeFormat, Config, LevelPadding, SharedLogger};
use log::{set_boxed_logger, set_max_level, LevelFilter, Log, Metadata, Record, SetLoggerError};

use std::thread;

/// The TestLogger struct. Provides a very basic Logger implementation that may be captured by cargo.
pub struct TestLogger {
    level: LevelFilter,
    config: Config,
}

impl TestLogger {
    /// init function. Globally initializes the TestLogger as the one and only used log facility.
    ///
    /// Takes the desired `Level` and `Config` as arguments. They cannot be changed later on.
    /// Fails if another Logger was already initialized.
    ///
    /// # Examples
    /// ```
    /// # extern crate r3bl_simple_logger;
    /// # use r3bl_simple_logger::*;
    /// # fn main() {
    /// // another logger
    /// # let _ = TestLogger::init(LevelFilter::Info, Config::default());
    /// let _ = TestLogger::init(LevelFilter::Info, Config::default());
    /// # }
    /// ```
    pub fn init(log_level: LevelFilter, config: Config) -> Result<(), SetLoggerError> {
        set_max_level(log_level);
        set_boxed_logger(TestLogger::new(log_level, config))
    }

    /// allows to create a new logger, that can be independently used, no matter what is globally set.
    ///
    /// no macros are provided for this case and you probably
    /// dont want to use this function, but `init()`, if you dont want to build a `CombinedLogger`.
    ///
    /// Takes the desired `Level` and `Config` as arguments. They cannot be changed later on.
    ///
    /// # Examples
    /// ```
    /// # extern crate r3bl_simple_logger;
    /// # use r3bl_simple_logger::*;
    /// # fn main() {
    /// // another logger
    /// # let test_logger = TestLogger::new(LevelFilter::Info, Config::default());
    /// let test_logger = TestLogger::new(LevelFilter::Info, Config::default());
    /// # }
    /// ```
    #[must_use]
    pub fn new(log_level: LevelFilter, config: Config) -> Box<TestLogger> {
        Box::new(TestLogger {
            level: log_level,
            config,
        })
    }
}

impl Log for TestLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record<'_>) {
        if self.enabled(record.metadata()) {
            log(&self.config, record);
        }
    }

    fn flush(&self) {}
}

impl SharedLogger for TestLogger {
    fn level(&self) -> LevelFilter {
        self.level
    }

    fn config(&self) -> Option<&Config> {
        Some(&self.config)
    }

    fn as_log(self: Box<Self>) -> Box<dyn Log> {
        Box::new(*self)
    }
}

#[inline(always)]
pub fn log(config: &Config, record: &Record<'_>) {
    if should_skip(config, record) {
        return;
    }

    if config.time <= record.level() && config.time != LevelFilter::Off {
        write_time(config);
    }

    if config.level <= record.level() && config.level != LevelFilter::Off {
        write_level(record, config);
    }

    if config.thread < record.level() && config.thread != LevelFilter::Off {
        write_thread_id();
    }

    if config.target <= record.level() && config.target != LevelFilter::Off {
        write_target(record);
    }

    if config.location <= record.level() && config.location != LevelFilter::Off {
        write_location(record);
    }

    if config.module <= record.level() && config.module != LevelFilter::Off {
        write_module(record);
    }

    write_args(record);
}

#[inline(always)]
pub fn write_time(config: &Config) {
    use time::format_description::well_known::*;

    let time = time::OffsetDateTime::now_utc().to_offset(config.time_offset);
    let res = match config.time_format {
        TimeFormat::Rfc2822 => time.format(&Rfc2822),
        TimeFormat::Rfc3339 => time.format(&Rfc3339),
        TimeFormat::Custom(format) => time.format(&format),
    };
    match res {
        Ok(time) => print!("{} ", time),
        Err(err) => panic!("Invalid time format: {}", err),
    };
}

#[inline(always)]
pub fn write_level(record: &Record<'_>, config: &Config) {
    match config.level_padding {
        LevelPadding::Left => print!("[{: >5}] ", record.level()),
        LevelPadding::Right => print!("[{: <5}] ", record.level()),
        LevelPadding::Off => print!("[{}] ", record.level()),
    };
}

#[inline(always)]
pub fn write_thread_id() {
    let id = format!("{:?}", thread::current().id());
    let id = id.replace("ThreadId(", "");
    let id = id.replace(')', "");
    print!("({}) ", id);
}

#[inline(always)]
pub fn write_target(record: &Record<'_>) {
    print!("{}: ", record.target());
}

#[inline(always)]
pub fn write_location(record: &Record<'_>) {
    let file = record.file().unwrap_or("<unknown>");
    if let Some(line) = record.line() {
        print!("[{}:{}] ", file, line);
    } else {
        print!("[{}:<unknown>] ", file);
    }
}

#[inline(always)]
pub fn write_module(record: &Record<'_>) {
    let module = record.module_path().unwrap_or("<unknown>");
    print!("[{}] ", module);
}

#[inline(always)]
pub fn write_args(record: &Record<'_>) {
    println!("{}", record.args());
}
