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

//! Module providing the TermLogger Implementation

use std::{io::{Error, Write},
          sync::Mutex};

use log::{set_boxed_logger,
          set_max_level,
          Level,
          LevelFilter,
          Log,
          Metadata,
          Record,
          SetLoggerError};
use termcolor::{BufferedStandardStream, ColorChoice};

use super::logging::{should_skip,
                     write_args,
                     write_level,
                     write_location,
                     write_module,
                     write_target,
                     write_thread_id,
                     write_thread_name,
                     write_time};
use crate::{Config, SharedLogger, ThreadLogMode};

struct OutputStreams {
    err: BufferedStandardStream,
    out: BufferedStandardStream,
}

/// Specifies which streams should be used when logging
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Default)]
pub enum TerminalMode {
    /// Only use Stdout
    Stdout,
    /// Only use Stderr
    Stderr,
    /// Use Stderr for Errors and Stdout otherwise
    #[default]
    Mixed,
}

/// The TermLogger struct. Provides a stderr/out based Logger implementation
///
/// Supports colored output
pub struct TermLogger {
    level: LevelFilter,
    config: Config,
    streams: Mutex<OutputStreams>,
}

impl TermLogger {
    /// init function. Globally initializes the TermLogger as the one and only used log facility.
    ///
    /// Takes the desired `Level` and `Config` as arguments. They cannot be changed later on.
    /// Fails if another Logger was already initialized
    ///
    /// # Examples
    /// ```
    /// # extern crate r3bl_simple_logger;
    /// # use r3bl_simple_logger::*;
    /// # fn main() {
    ///     TermLogger::init(
    ///         LevelFilter::Info,
    ///         Config::default(),
    ///         TerminalMode::Mixed,
    ///         ColorChoice::Auto
    ///     );
    /// # }
    /// ```
    pub fn init(
        log_level: LevelFilter,
        config: Config,
        mode: TerminalMode,
        color_choice: ColorChoice,
    ) -> Result<(), SetLoggerError> {
        let logger = TermLogger::new(log_level, config, mode, color_choice);
        set_max_level(log_level);
        set_boxed_logger(logger)?;
        Ok(())
    }

    /// allows to create a new logger, that can be independently used, no matter whats globally set.
    ///
    /// no macros are provided for this case and you probably
    /// don't want to use this function, but `init()`, if you don't want to build a `CombinedLogger`.
    ///
    /// Takes the desired `Level` and `Config` as arguments. They cannot be changed later on.
    ///
    /// Returns a `Box`ed TermLogger
    ///
    /// # Examples
    /// ```
    /// # extern crate r3bl_simple_logger;
    /// # use r3bl_simple_logger::*;
    /// # fn main() {
    /// let term_logger = TermLogger::new(
    ///     LevelFilter::Info,
    ///     Config::default(),
    ///     TerminalMode::Mixed,
    ///     ColorChoice::Auto
    /// );
    /// # }
    /// ```
    #[must_use]
    pub fn new(
        log_level: LevelFilter,
        config: Config,
        mode: TerminalMode,
        color_choice: ColorChoice,
    ) -> Box<TermLogger> {
        let streams = match mode {
            TerminalMode::Stdout => OutputStreams {
                err: BufferedStandardStream::stdout(color_choice),
                out: BufferedStandardStream::stdout(color_choice),
            },
            TerminalMode::Stderr => OutputStreams {
                err: BufferedStandardStream::stderr(color_choice),
                out: BufferedStandardStream::stderr(color_choice),
            },
            TerminalMode::Mixed => OutputStreams {
                err: BufferedStandardStream::stderr(color_choice),
                out: BufferedStandardStream::stdout(color_choice),
            },
        };

        Box::new(TermLogger {
            level: log_level,
            config,
            streams: Mutex::new(streams),
        })
    }

    fn try_log_term(
        &self,
        record: &Record<'_>,
        term_lock: &mut BufferedStandardStream,
    ) -> Result<(), Error> {
        if self.config.time <= record.level() && self.config.time != LevelFilter::Off {
            write_time(term_lock, &self.config)?;
        }

        if self.config.level <= record.level() && self.config.level != LevelFilter::Off {
            write_level(record, term_lock, &self.config)?;
        }

        if self.config.thread <= record.level() && self.config.thread != LevelFilter::Off
        {
            match self.config.thread_log_mode {
                ThreadLogMode::IDs => {
                    write_thread_id(term_lock, &self.config)?;
                }
                ThreadLogMode::Names | ThreadLogMode::Both => {
                    write_thread_name(term_lock, &self.config)?;
                }
            }
        }

        if self.config.target <= record.level() && self.config.target != LevelFilter::Off
        {
            write_target(record, term_lock, &self.config)?;
        }

        if self.config.location <= record.level()
            && self.config.location != LevelFilter::Off
        {
            write_location(record, term_lock)?;
        }

        if self.config.module <= record.level() && self.config.module != LevelFilter::Off
        {
            write_module(record, term_lock)?;
        }

        write_args(record, term_lock)?;

        // The log crate holds the logger as a `static mut`, which isn't dropped
        // at program exit: https://doc.rust-lang.org/reference/items/static-items.html
        // Sadly, this means we can't rely on the BufferedStandardStreams flushing
        // themselves on the way out, so to avoid the Case of the Missing 8k,
        // flush each entry.
        term_lock.flush()
    }

    #[allow(clippy::unwrap_in_result)]
    fn try_log(&self, record: &Record<'_>) -> Result<(), Error> {
        if self.enabled(record.metadata()) {
            if should_skip(&self.config, record) {
                return Ok(());
            }

            let mut streams = self.streams.lock().unwrap();

            if record.level() == Level::Error {
                self.try_log_term(record, &mut streams.err)
            } else {
                self.try_log_term(record, &mut streams.out)
            }
        } else {
            Ok(())
        }
    }
}

impl Log for TermLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool { metadata.level() <= self.level }

    fn log(&self, record: &Record<'_>) { let _ = self.try_log(record); }

    fn flush(&self) {
        let mut streams = self.streams.lock().unwrap();
        let _ = streams.out.flush();
        let _ = streams.err.flush();
    }
}

impl SharedLogger for TermLogger {
    fn level(&self) -> LevelFilter { self.level }

    fn config(&self) -> Option<&Config> { Some(&self.config) }

    fn as_log(self: Box<Self>) -> Box<dyn Log> { Box::new(*self) }
}
