/*
 *   Copyright (c) 2023 R3BL LLC
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

//! Module providing the FileLogger Implementation

use std::{io::Write, sync::Mutex};

use log::{set_boxed_logger,
          set_max_level,
          LevelFilter,
          Log,
          Metadata,
          Record,
          SetLoggerError};

use super::logging::try_log;
use crate::{Config, SharedLogger};

/// The WriteLogger struct. Provides a Logger implementation for structs implementing `Write`, e.g. File
pub struct WriteLogger<W: Write + Send + 'static> {
    level: LevelFilter,
    config: Config,
    writable: Mutex<W>,
}

impl<W: Write + Send + 'static> WriteLogger<W> {
    /// init function. Globally initializes the WriteLogger as the one and only used log facility.
    ///
    /// Takes the desired `Level`, `Config` and `Write` struct as arguments. They cannot be changed later on.
    /// Fails if another Logger was already initialized.
    ///
    /// # Examples
    /// ```
    /// # extern crate r3bl_simple_logger;
    /// # use r3bl_simple_logger::*;
    /// # use std::fs::File;
    /// # fn main() {
    /// let _ = WriteLogger::init(LevelFilter::Info, Config::default(), File::create("my_rust_bin.log").unwrap());
    /// # }
    /// ```
    pub fn init(
        log_level: LevelFilter,
        config: Config,
        writable: W,
    ) -> Result<(), SetLoggerError> {
        set_max_level(log_level);
        set_boxed_logger(WriteLogger::new(log_level, config, writable))
    }

    /// allows to create a new logger, that can be independently used, no matter what is globally set.
    ///
    /// no macros are provided for this case and you probably
    /// dont want to use this function, but `init()`, if you dont want to build a `CombinedLogger`.
    ///
    /// Takes the desired `Level`, `Config` and `Write` struct as arguments. They cannot be changed later on.
    ///
    /// # Examples
    /// ```
    /// # extern crate r3bl_simple_logger;
    /// # use r3bl_simple_logger::*;
    /// # use std::fs::File;
    /// # fn main() {
    /// let file_logger = WriteLogger::new(LevelFilter::Info, Config::default(), File::create("my_rust_bin.log").unwrap());
    /// # }
    /// ```
    #[must_use]
    pub fn new(
        log_level: LevelFilter,
        config: Config,
        writable: W,
    ) -> Box<WriteLogger<W>> {
        Box::new(WriteLogger {
            level: log_level,
            config,
            writable: Mutex::new(writable),
        })
    }
}

impl<W: Write + Send + 'static> Log for WriteLogger<W> {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool { metadata.level() <= self.level }

    fn log(&self, record: &Record<'_>) {
        if self.enabled(record.metadata()) {
            let mut write_lock = self.writable.lock().unwrap();
            let _ = try_log(&self.config, record, &mut *write_lock);
        }
    }

    fn flush(&self) { let _ = self.writable.lock().unwrap().flush(); }
}

impl<W: Write + Send + 'static> SharedLogger for WriteLogger<W> {
    fn level(&self) -> LevelFilter { self.level }

    fn config(&self) -> Option<&Config> { Some(&self.config) }

    fn as_log(self: Box<Self>) -> Box<dyn Log> { Box::new(*self) }
}
