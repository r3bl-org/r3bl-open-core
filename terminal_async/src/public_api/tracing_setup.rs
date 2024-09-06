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
use std::{io, path::PathBuf, str::FromStr};
use tracing_core::LevelFilter;
use tracing_subscriber::{
    layer::SubscriberExt, registry::LookupSpan, util::SubscriberInitExt, Layer,
};

/// Fields:
/// - `writers`: Vec<[WriterArg]> - Zero or more writers to use for
///   tracing.
/// - `level`: [tracing::Level] - The log level to use for tracing.
/// - `tracing_log_file_path_and_prefix`: [String] - The file path and prefix to use for
///   the log file. Eg: `/tmp/tcp_api_server` or `tcp_api_server`.
#[derive(Clone)]
pub struct TracingConfig {
    pub writers: Vec<WriterArg>,
    pub level: tracing::Level,
    pub tracing_log_file_path_and_prefix: String,
    pub preferred_display: DisplayPreference,
}

mod tracing_config_impl {
    use super::*;

    impl TracingConfig {
        /// The default configuration for tracing. This will log to both the given
        /// [DisplayPreference] and a file.
        pub fn new(preferred_display: DisplayPreference) -> Self {
            Self {
                writers: vec![WriterArg::File, WriterArg::Stdout],
                level: tracing::Level::DEBUG,
                tracing_log_file_path_and_prefix: "tracing_log_file_debug.log".to_string(),
                preferred_display,
            }
        }

        pub fn get_level_filter(&self) -> LevelFilter {
            tracing_subscriber::filter::LevelFilter::from_level(self.level)
        }
    }
}

/// Use to parse the command line arguments (provided by `clap` crate.
#[derive(Clone, Debug, PartialEq)]
pub enum WriterArg {
    Stdout,
    File,
    None,
}

/// Handle converting parsed command line arguments (via `clap` crate) into a [WriterArg].
///
/// - This is an intermediate representation (IR),
/// - which is ultimately converted into [WriterConfig] by [writer_config_impl::from]
///   before it used in the rest of the system.
pub mod writer_arg_impl {
    use super::*;

    /// The `clap` crate parses this into a string. This function convert it into a
    /// [WriterArg].
    impl FromStr for WriterArg {
        type Err = String;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                "stdout" => Ok(WriterArg::Stdout),
                "file" => Ok(WriterArg::File),
                "none" => Ok(WriterArg::None),
                "" => Ok(WriterArg::None),
                _ => Err(format!("{} is not a valid tracing writer", s)),
            }
        }
    }
}

#[derive(Clone)]
pub enum DisplayPreference {
    Stdout,
    Stderr,
    SharedWriter(SharedWriter),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WriterConfig {
    Display,
    File,
    DisplayAndFile,
}

type DynLayer<S> = dyn Layer<S> + Send + Sync + 'static;

pub mod writer_config_impl {
    use super::*;

    pub fn from(writers: &[WriterArg]) -> Option<WriterConfig> {
        let contains_file_writer = writers.contains(&WriterArg::File);
        let contains_stdout_writer = writers.contains(&WriterArg::Stdout);
        match (contains_file_writer, contains_stdout_writer) {
            (true, true) => Some(WriterConfig::DisplayAndFile),
            (true, false) => Some(WriterConfig::File),
            (false, true) => Some(WriterConfig::Display),
            (false, false) => None,
        }
    }

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
                WriterConfig::DisplayAndFile | WriterConfig::Display => match preferred_display {
                    DisplayPreference::Stdout => Some(Box::new(
                        fmt_layer.with_writer(io::stdout).with_filter(level_filter),
                    )),
                    DisplayPreference::Stderr => Some(Box::new(
                        fmt_layer.with_writer(io::stderr).with_filter(level_filter),
                    )),
                    DisplayPreference::SharedWriter(shared_writer) => {
                        let tracing_writer =
                            move || -> Box<dyn std::io::Write> { Box::new(shared_writer.clone()) };
                        Some(Box::new(
                            fmt_layer
                                .with_writer(tracing_writer)
                                .with_filter(level_filter),
                        ))
                    }
                },
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
}

/// Returns the layers. Once you have the layers, you can run the following:
/// `init(..).map(|layers| tracing_subscriber::registry().with(layers).init());`
pub fn init(tracing_config: TracingConfig) -> miette::Result<()> {
    try_create_layers(tracing_config)
        .map(|layers| tracing_subscriber::registry().with(layers).init())
}

/// Returns the layers. Once you have the layers, you can run the following:
/// `create_layers(..).map(|layers| tracing_subscriber::registry().with(layers).init());`
pub fn try_create_layers(
    tracing_config: TracingConfig,
) -> miette::Result<Option<Vec<Box<DynLayer<tracing_subscriber::Registry>>>>> {
    // Transform the `clap` crate's parsed command line arguments into a `WriterConfig`.
    let writer_config = match writer_config_impl::from(&tracing_config.writers) {
        Some(it) => it,
        None => return Ok(None),
    };

    let level_filter = tracing_config.get_level_filter();

    // Create the layers based on the writer configuration.
    let layers = {
        let mut return_it: Vec<Box<DynLayer<tracing_subscriber::Registry>>> = vec![];

        // Set the level filter from the tracing configuration. This is needed if you add more
        // layers, like OpenTelemetry, which don't have a level filter.
        return_it.push(Box::new(level_filter));
        // The following is another way of setting the level filter, if you want to
        // specify log level using env vars, as an override for the cli args.
        // ```
        // use tracing_subscriber::EnvFilter;
        // layers.push(Box::new(
        //     EnvFilter::from_default_env().add_directive(tracing_config.level),
        // ));
        // ``

        let _ = writer_config
            .try_create_display_layer(level_filter, tracing_config.preferred_display.clone())?
            .map(|layer| return_it.push(layer));

        let _ = writer_config
            .try_create_file_layer(
                level_filter,
                tracing_config.tracing_log_file_path_and_prefix.clone(),
            )?
            .map(|layer| return_it.push(layer));

        return_it
    };

    // Return the layers.
    Ok(Some(layers))
}

mod rolling_file_appender_impl {
    use super::*;

    /// Note that if you wrap this up in a non blocking writer, as shown here, it doesn't work:
    /// `tracing_appender::non_blocking(try_create_rolling_file_appender("foo")?);`
    pub fn try_create(
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

#[cfg(test)]
mod tests_writer_arg {
    use super::*;
    use crate::WriterArg;

    #[test]
    fn test_from_str() {
        assert_eq!(WriterArg::from_str("stdout").unwrap(), WriterArg::Stdout);
        assert_eq!(WriterArg::from_str("file").unwrap(), WriterArg::File);
    }

    #[test]
    fn test_invalid_from_str() {
        assert!(WriterArg::from_str("invalid").is_err());
    }
}
