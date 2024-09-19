/*
 *   Copyright (c) 2024 R3BL LLC
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

use tracing_core::LevelFilter;
use tracing_subscriber::{registry::LookupSpan, Layer};

use super::DisplayPreference;
use crate::{tracing_logging::{rolling_file_appender_impl, writer_arg::WriterArg},
            tracing_setup::*};

/// Avoid gnarly type annotations by using a macro to create the `fmt` layer.
#[macro_export]
macro_rules! create_fmt {
    () => {
        tracing_subscriber::fmt::layer()
            .compact()
            .without_time()
            .with_thread_ids(true)
            .with_thread_names(false)
            .with_target(false)
            .with_file(false)
            .with_line_number(false)
            .with_ansi(true)
    };
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WriterConfig {
    Display,
    File,
    DisplayAndFile,
}

/// 1. It is expected that arguments are passed in via the command line (using `clap`).
/// 1. These are then converted into a list of [WriterArg]s.
/// 1. Which are then converted into a [WriterConfig] using this trait implementation.
impl TryFrom<&[WriterArg]> for WriterConfig {
    type Error = &'static str;

    fn try_from(writers: &[WriterArg]) -> Result<Self, Self::Error> {
        let contains_file_writer = writers.contains(&WriterArg::File);
        let contains_stdout_writer = writers.contains(&WriterArg::Stdout);
        match (contains_file_writer, contains_stdout_writer) {
            (true, true) => Ok(WriterConfig::DisplayAndFile),
            (true, false) => Ok(WriterConfig::File),
            (false, true) => Ok(WriterConfig::Display),
            (false, false) => Err("No valid writer configuration found"),
        }
    }
}

impl WriterConfig {
    /// This erases the concrete type of the writer, and returns a boxed layer. This
    /// is useful for composition of layers. There's more info in the docs
    /// [here](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/index.html#runtime-configuration-with-layers).
    pub fn try_create_display_layer<S>(
        self,
        level_filter: LevelFilter,
        preferred_display: DisplayPreference,
    ) -> miette::Result<Option<Box<DynLayer<S>>>>
    where
        S: tracing_core::Subscriber,
        for<'a> S: LookupSpan<'a>,
    {
        // Shared configuration regardless of where logs are output to.
        let fmt_layer = create_fmt!();

        // Configure the writer based on the desired log target, and return it.
        Ok(match self {
            WriterConfig::DisplayAndFile | WriterConfig::Display => {
                match preferred_display {
                    DisplayPreference::Stdout => Some(Box::new(
                        fmt_layer
                            .with_writer(std::io::stdout)
                            .with_filter(level_filter),
                    )),
                    DisplayPreference::Stderr => Some(Box::new(
                        fmt_layer
                            .with_writer(std::io::stderr)
                            .with_filter(level_filter),
                    )),
                    DisplayPreference::SharedWriter(shared_writer) => {
                        let tracing_writer = move || -> Box<dyn std::io::Write> {
                            Box::new(shared_writer.clone())
                        };
                        Some(Box::new(
                            fmt_layer
                                .with_writer(tracing_writer)
                                .with_filter(level_filter),
                        ))
                    }
                }
            }
            _ => None,
        })
    }

    /// This erases the concrete type of the writer, and returns a boxed layer. This is
    /// useful for composition of layers. There's more info in the docs
    /// [here](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/index.html#runtime-configuration-with-layers).
    pub fn try_create_file_layer<S>(
        self,
        level_filter: LevelFilter,
        tracing_log_file_path_and_prefix: String,
    ) -> miette::Result<Option<Box<DynLayer<S>>>>
    where
        S: tracing_core::Subscriber,
        for<'a> S: LookupSpan<'a>,
    {
        // Shared configuration regardless of where logs are output to.
        let fmt_layer = create_fmt!();

        // Configure the writer based on the desired log target, and return it.
        Ok(match self {
            WriterConfig::DisplayAndFile | WriterConfig::File => {
                let file = rolling_file_appender_impl::try_create(
                    tracing_log_file_path_and_prefix.as_str(),
                )?;
                Some(Box::new(
                    fmt_layer.with_writer(file).with_filter(level_filter),
                ))
            }
            _ => None,
        })
    }
}
