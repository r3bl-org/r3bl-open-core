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

use crate::SharedWriter;
use crossterm::style::Stylize;
use miette::IntoDiagnostic;
use std::{path::PathBuf, str::FromStr};
use tracing::info;
use tracing_subscriber::fmt::writer::MakeWriterExt;

/// Fields:
/// - `writers`: Vec<[tracing_writer_config::Writer]> - Zero or more writers to use for
///   tracing.
/// - `level`: [tracing::Level] - The log level to use for tracing.
/// - `tracing_log_file_path_and_prefix`: [String] - The file path and prefix to use for
///   the log file. Eg: `/tmp/tcp_api_server` or `tcp_api_server`.
#[derive(Clone)]
pub struct TracingConfig {
    pub writers: Vec<tracing_writer_config::Writer>,
    pub level: tracing::Level,
    pub tracing_log_file_path_and_prefix: String,
    pub preferred_display: DisplayPreference,
}

#[derive(Clone)]
pub enum DisplayPreference {
    Stdout,
    Stderr,
    SharedWriter(SharedWriter),
}

mod tracing_config_impl {
    use super::*;

    impl TracingConfig {
        /// The default configuration for tracing. This will log to both the given
        /// [DisplayPreference] and a file.
        pub fn new(preferred_display: DisplayPreference) -> Self {
            Self {
                writers: vec![
                    tracing_writer_config::Writer::File,
                    tracing_writer_config::Writer::Stdout,
                ],
                level: tracing::Level::DEBUG,
                tracing_log_file_path_and_prefix: "tracing_log_file_debug.log".to_string(),
                preferred_display,
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WriterConfig {
    Display,
    File,
    DisplayAndFile,
    None,
}

impl From<&Vec<tracing_writer_config::Writer>> for WriterConfig {
    fn from(writers: &Vec<tracing_writer_config::Writer>) -> Self {
        let contains_file_writer = writers.contains(&tracing_writer_config::Writer::File);
        let contains_stdout_writer = writers.contains(&tracing_writer_config::Writer::Stdout);
        match (contains_file_writer, contains_stdout_writer) {
            (true, true) => WriterConfig::DisplayAndFile,
            (true, false) => WriterConfig::File,
            (false, true) => WriterConfig::Display,
            (false, false) => WriterConfig::None,
        }
    }
}

/// Initialize the global tracing subscriber with the given writers, level, and file path.
///
/// More info:
/// - [Setup tracing](https://tokio.rs/tokio/topics/tracing)
/// - [Configure
///   subscriber](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/index.html#configuration)
/// - [Rolling file appender](https://docs.rs/tracing-appender/latest/tracing_appender/)
/// - [Examples](https://github.com/tokio-rs/tracing/blob/master/examples/examples/appender-multifile.rs)
/// - [MakeWriter](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/trait.MakeWriter.html)
pub fn init(tracing_config: TracingConfig) -> miette::Result<()> {
    let TracingConfig {
        writers,
        level,
        tracing_log_file_path_and_prefix,
        preferred_display,
    } = tracing_config;

    let writer_config = WriterConfig::from(&writers);
    if writer_config == WriterConfig::None {
        return Ok(());
    }

    let builder = tracing_subscriber::fmt()
        .compact() /* one line output */
        // .pretty() /* multi line pretty output */
        .with_max_level(level)
        .without_time()
        .with_thread_ids(true)
        .with_thread_names(false)
        .with_target(false)
        .with_file(false)
        .with_line_number(false)
        .with_ansi(true);

    match writer_config {
        // Both file & stdout writer.
        WriterConfig::DisplayAndFile => {
            let writer_log = init_impl::try_create_rolling_file_appender(
                tracing_log_file_path_and_prefix.as_str(),
            )?
            .with_max_level(level);

            match preferred_display {
                DisplayPreference::Stdout => {
                    let writer_stdout =
                        move || -> Box<dyn std::io::Write> { Box::new(std::io::stdout()) };
                    let both = writer_log.and(writer_stdout);
                    let subscriber = builder.with_writer(both).finish();
                    tracing::subscriber::set_global_default(subscriber).into_diagnostic()?;
                }
                DisplayPreference::Stderr => {
                    let writer_stderr =
                        move || -> Box<dyn std::io::Write> { Box::new(std::io::stderr()) };
                    let both = writer_log.and(writer_stderr);
                    let subscriber = builder.with_writer(both).finish();
                    tracing::subscriber::set_global_default(subscriber).into_diagnostic()?;
                }
                // Cloning a `shared_writer` makes it fail silently. This means that the
                // tokio subscriber won't complain with `[tracing-subscriber] Unable to
                // write an event to the Writer for this Subscriber! Error: SharedWriter
                // Receiver has closed` if the receiver end of the channel is closed.
                DisplayPreference::SharedWriter(shared_writer) => {
                    let writer_shared_writer =
                        move || -> Box<dyn std::io::Write> { Box::new(shared_writer.clone()) };
                    let both = writer_log.and(writer_shared_writer);
                    let subscriber = builder.with_writer(both).finish();
                    tracing::subscriber::set_global_default(subscriber).into_diagnostic()?;
                }
            }
        }
        // Just file writer.
        WriterConfig::File => {
            let writer_log = init_impl::try_create_rolling_file_appender(
                tracing_log_file_path_and_prefix.as_str(),
            )?
            .with_max_level(level);
            let subscriber = builder.with_writer(writer_log).finish();
            tracing::subscriber::set_global_default(subscriber).into_diagnostic()?;
        }
        // Just stdout writer.
        WriterConfig::Display => match preferred_display {
            DisplayPreference::Stdout => {
                let writer_stdout =
                    move || -> Box<dyn std::io::Write> { Box::new(std::io::stdout()) };
                let subscriber = builder.with_writer(writer_stdout).finish();
                tracing::subscriber::set_global_default(subscriber).into_diagnostic()?;
            }
            DisplayPreference::Stderr => {
                let writer_stderr =
                    move || -> Box<dyn std::io::Write> { Box::new(std::io::stderr()) };
                let subscriber = builder.with_writer(writer_stderr).finish();
                tracing::subscriber::set_global_default(subscriber).into_diagnostic()?;
            }
            // Cloning a `shared_writer` makes it fail silently. This means that the
            // tokio subscriber won't complain with `[tracing-subscriber] Unable to
            // write an event to the Writer for this Subscriber! Error: SharedWriter
            // Receiver has closed` if the receiver end of the channel is closed.
            DisplayPreference::SharedWriter(shared_writer) => {
                let writer_stdout =
                    move || -> Box<dyn std::io::Write> { Box::new(shared_writer.clone()) };
                let subscriber = builder.with_writer(writer_stdout).finish();
                tracing::subscriber::set_global_default(subscriber).into_diagnostic()?;
            }
        },
        WriterConfig::None => {
            unreachable!()
        }
    }

    info!(
        "tracing enabled {}",
        format!(
            "{:?}, {:?}, {:?}",
            writers, level, tracing_log_file_path_and_prefix
        )
        .cyan()
        .bold()
    );

    Ok(())
}

mod init_impl {
    use super::*;

    /// Note that if you wrap this up in a non blocking writer, as shown here, it doesn't work:
    /// `tracing_appender::non_blocking(try_create_rolling_file_appender("foo")?);`
    pub fn try_create_rolling_file_appender(
        path_str: &str,
    ) -> miette::Result<tracing_appender::rolling::RollingFileAppender> {
        let path = PathBuf::from(&path_str);

        let parent = path.parent().ok_or_else(|| {
            miette::miette!(
                format!("Can't access current folder {}. It might not exist, or don't have required permissions.", path.display())
            )
        })?;

        let file_stem = path.file_name().ok_or_else(|| {
            miette::miette!(format!(
            "Can't access file name {}. It might not exist, or don't have required permissions.",
            path.display()
        ))
        })?;

        Ok(tracing_appender::rolling::never(parent, file_stem))
    }
}

pub mod tracing_writer_config {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    pub enum Writer {
        Stdout,
        File,
        None,
    }

    impl FromStr for Writer {
        type Err = String;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                "stdout" => Ok(Writer::Stdout),
                "file" => Ok(Writer::File),
                "none" => Ok(Writer::None),
                "" => Ok(Writer::None),
                _ => Err(format!("{} is not a valid tracing writer", s)),
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_from_str() {
            assert_eq!(Writer::from_str("stdout").unwrap(), Writer::Stdout);
            assert_eq!(Writer::from_str("file").unwrap(), Writer::File);
        }

        #[test]
        fn test_invalid_from_str() {
            assert!(Writer::from_str("invalid").is_err());
        }
    }
}
