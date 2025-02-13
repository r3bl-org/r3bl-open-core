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

use std::{fs::OpenOptions, io::Write, ops::Add, path::Path};

use r3bl_core::ok;
use tracing::dispatcher;

use crate::{DisplayPreference, TracingConfig, WriterConfig};

// XMARK: Clever Rust, use of `impl Into<ConfigStruct>` for elegant constructor config options.
/// This module makes it easier to configure the logging system. Instead of having lots of
/// complex arguments to the [try_initialize_logging_global] and
/// [try_initialize_logging_thread_local] functions, they both receive a type that
/// implements the [`Into<TracingConfig>`] trait. This is a very powerful pattern since it
/// can be used to convert many different types into a [TracingConfig] type while
/// retaining a simple function signature. Here are some examples of what is possible:
///
/// ```no_run
/// use r3bl_log::{
///     TracingConfig, DisplayPreference, WriterConfig,
///     try_initialize_logging_global, try_initialize_logging_thread_local
/// };
///
/// let level = tracing::Level::DEBUG;
/// let config_1: TracingConfig = level.into();
///
/// let level_filter = tracing_core::LevelFilter::DEBUG;
/// let config_2: TracingConfig = level_filter.into();
///
/// let preferred_display = DisplayPreference::Stdout;
/// let config_3: TracingConfig = preferred_display.into();
///
/// let writer_config = WriterConfig::File("log.txt".to_string());
/// let config_4: TracingConfig = writer_config.into();
///
/// let config_compose: TracingConfig = config_2 + config_3;
///
/// try_initialize_logging_global(config_compose);
/// try_initialize_logging_thread_local(config_1 + config_4);
/// ```
pub mod tracing_config_options {
    use super::*;

    pub const DEFAULT_LOG_FILE_NAME: &str = "log.txt";

    impl From<tracing::Level> for TracingConfig {
        fn from(level: tracing::Level) -> Self {
            Self {
                level_filter: level.into(),
                writer_config: WriterConfig::File(DEFAULT_LOG_FILE_NAME.to_string()),
            }
        }
    }

    impl From<tracing_core::LevelFilter> for TracingConfig {
        fn from(level_filter: tracing_core::LevelFilter) -> Self {
            Self {
                level_filter,
                writer_config: WriterConfig::File(DEFAULT_LOG_FILE_NAME.to_string()),
            }
        }
    }

    impl From<DisplayPreference> for TracingConfig {
        fn from(preferred_display: DisplayPreference) -> Self {
            Self {
                level_filter: tracing_core::LevelFilter::DEBUG,
                writer_config: WriterConfig::Display(preferred_display),
            }
        }
    }

    impl From<WriterConfig> for TracingConfig {
        fn from(writer_config: WriterConfig) -> Self {
            Self {
                level_filter: tracing_core::LevelFilter::DEBUG,
                writer_config,
            }
        }
    }

    /// Merge two [TracingConfig] instances together. This is useful when you want to
    /// combine multiple configurations into one.
    impl Add<TracingConfig> for TracingConfig {
        type Output = Self;

        fn add(self, rhs: Self) -> Self::Output {
            Self {
                level_filter: self.level_filter.max(rhs.level_filter),
                writer_config: match self.writer_config {
                    WriterConfig::None => rhs.writer_config,
                    _ => self.writer_config + rhs.writer_config,
                },
            }
        }
    }

    /// Merge two [WriterConfig] instances together. The `rhs` will clobber the `self` if
    /// it has a "some" value. That is, the value in `rhs` has higher specificity.
    ///
    /// Here are some examples:
    /// - `{a: "foo"} + {a: "bar"} = {a: "bar"}`.
    /// - `{a: None } + {a: "bar"} = {a: "bar"}`.
    /// - `{a: "foo"} + {a: None } = {a: "foo"}`.
    ///
    /// If a field is not set (set to `None`) and it is on the `lhs`, it will be
    /// overwritten with a non-`None` value. This is a largely arbitrary decision, but
    /// there has to be a single rule to reason about the code and compose the
    /// configurations.
    impl Add<WriterConfig> for WriterConfig {
        type Output = Self;

        fn add(self, rhs: WriterConfig) -> Self::Output {
            use WriterConfig::*;

            match (self, rhs) {
                // No collision merge.
                (None, wc_rhs) => wc_rhs,
                (wc_lhs, None) => wc_lhs,
                (Display(dp_lhs), File(f_rhs)) => DisplayAndFile(dp_lhs, f_rhs),
                (File(f_lhs), Display(dp_rhs)) => DisplayAndFile(dp_rhs, f_lhs),

                // Collision (rhs has higher specificity). DisplayPreference can't be merged.
                (Display(_dp_lhs), DisplayAndFile(dp_rhs, f_rhs)) => {
                    DisplayAndFile(dp_rhs, f_rhs)
                }

                // Collision (rhs has higher specificity). DisplayPreference can't be merged.
                (Display(_dp_lhs), Display(dp_rhs)) => Display(dp_rhs),

                // Collision (rhs has higher specificity). String can't be merged.
                (File(_f_lhs), File(f_rhs)) => File(f_rhs),

                // Collision (rhs has higher specificity). String can't be merged.
                (File(_f_lhs), DisplayAndFile(dp_rhs, f_rhs)) => {
                    DisplayAndFile(dp_rhs, f_rhs)
                }

                // Collision (rhs has higher specificity). DisplayPreference can't be merged.
                (DisplayAndFile(_dp_lhs, f_rhs), Display(dp_rhs)) => {
                    DisplayAndFile(dp_rhs, f_rhs)
                }

                // Collision (rhs has higher specificity). String can't be merged.
                (DisplayAndFile(dp_lhs, _f_lhs), File(f_rhs)) => {
                    DisplayAndFile(dp_lhs, f_rhs)
                }

                // Collision (rhs has higher specificity). DisplayPreference and String can't be merged.
                (DisplayAndFile(_dp_lhs, _f_lhs), DisplayAndFile(dp_rhs, f_rhs)) => {
                    DisplayAndFile(dp_rhs, f_rhs)
                }
            }
        }
    }

    #[cfg(test)]
    mod tests_add_writer_configs {
        use r3bl_core::SharedWriter;

        #[test]
        fn test_add_writer_configs() {
            use super::*;

            // Underlying data.
            let (line_sender, _) = tokio::sync::mpsc::channel(1_000);
            let shared_writer = SharedWriter::new(line_sender);
            let dp_shared = DisplayPreference::SharedWriter(shared_writer);
            let dp_stdout = DisplayPreference::Stdout;
            let dp_stderr = DisplayPreference::Stderr;
            let fname = "log.txt".to_string();

            // Setup test configs.
            let none = WriterConfig::None;
            let display_stdout = WriterConfig::Display(dp_stdout.clone());
            let display_stderr = WriterConfig::Display(dp_stderr.clone());
            let display_sharedwriter = WriterConfig::Display(dp_shared.clone());
            let file = WriterConfig::File(fname.clone());
            let display_stdout_and_file =
                WriterConfig::DisplayAndFile(dp_stdout, fname.clone());
            let display_stderr_and_file =
                WriterConfig::DisplayAndFile(dp_stderr, fname.clone());
            let display_sharedwriter_and_file =
                WriterConfig::DisplayAndFile(dp_shared, fname.clone());

            // No collision merge.
            assert_eq!(none.clone() + none.clone(), none);
            assert_eq!(display_stdout.clone() + none.clone(), display_stdout);
            assert_eq!(none.clone() + display_stdout.clone(), display_stdout);
            assert_eq!(
                display_stdout.clone() + display_stderr.clone(),
                display_stderr
            );
            assert_eq!(
                display_stderr.clone() + display_stdout.clone(),
                display_stdout
            );
            assert_eq!(file.clone() + none.clone(), file);
            assert_eq!(none.clone() + file.clone(), file);
            assert_eq!(
                display_stdout_and_file.clone() + none.clone(),
                display_stdout_and_file
            );
            assert_eq!(
                none.clone() + display_stdout_and_file.clone(),
                display_stdout_and_file
            );
            assert_eq!(
                display_stdout_and_file.clone() + display_stderr.clone(),
                display_stderr_and_file
            );
            assert_eq!(
                display_stderr.clone() + display_stdout_and_file.clone(),
                display_stdout_and_file
            );
            assert_eq!(
                display_sharedwriter.clone() + none.clone(),
                display_sharedwriter
            );
            assert_eq!(
                none.clone() + display_sharedwriter.clone(),
                display_sharedwriter
            );
            assert_eq!(
                display_sharedwriter.clone() + display_stdout.clone(),
                display_stdout
            );
            assert_eq!(
                display_stdout.clone() + display_sharedwriter.clone(),
                display_sharedwriter
            );
            assert_eq!(
                display_sharedwriter.clone() + file.clone(),
                display_sharedwriter_and_file.clone()
            );
            assert_eq!(
                file.clone() + display_sharedwriter.clone(),
                display_sharedwriter_and_file.clone()
            );

            // Collision (rhs has higher specificity). DisplayPreference can't be merged.
            assert_eq!(
                display_stdout.clone() + display_stderr_and_file.clone(),
                display_stderr_and_file
            );
            assert_eq!(
                display_stderr_and_file.clone() + display_stdout.clone(),
                display_stdout_and_file
            );
            assert_eq!(
                display_stdout.clone() + display_stderr.clone(),
                display_stderr
            );
            assert_eq!(
                display_stderr.clone() + display_stdout.clone(),
                display_stdout
            );
            assert_eq!(
                display_sharedwriter.clone() + display_stdout_and_file.clone(),
                display_stdout_and_file
            );
            assert_eq!(
                display_stdout_and_file.clone() + display_sharedwriter.clone(),
                display_sharedwriter_and_file
            );

            // Collision (rhs has higher specificity). String can't be merged.
            assert_eq!(
                file.clone() + display_stderr_and_file.clone(),
                display_stderr_and_file
            );
            assert_eq!(
                display_stderr_and_file.clone() + file.clone(),
                display_stderr_and_file
            );
            assert_eq!(
                file.clone() + display_stdout.clone(),
                display_stdout_and_file
            );
            assert_eq!(
                display_stdout_and_file.clone() + file.clone(),
                display_stdout_and_file
            );
            assert_eq!(
                display_sharedwriter_and_file.clone() + file.clone(),
                display_sharedwriter_and_file
            );
            assert_eq!(
                file.clone() + display_sharedwriter_and_file.clone(),
                display_sharedwriter_and_file
            );

            // Collision (rhs has higher specificity). DisplayPreference and String can't be merged.
            assert_eq!(
                display_stdout_and_file.clone() + display_stderr_and_file.clone(),
                display_stderr_and_file
            );
            assert_eq!(
                display_sharedwriter_and_file.clone() + display_stdout_and_file.clone(),
                display_stdout_and_file
            );
        }
    }
}

/// Global default subscriber, which once set, can't be unset or changed.
/// - This is great for apps.
/// - Docs for [Global default tracing
///   subscriber](https://docs.rs/tracing/latest/tracing/subscriber/fn.set_global_default.html)
/// - Configure this using the [mod@tracing_config_options] module (which converts any number of
///   arguments into [`Into<TracingConfig>`]. Look at this module for default configuration.
///
/// Logging is **DISABLED** by **default**.
///
/// If you don't call this function w/ a value other than
/// [tracing_core::LevelFilter::OFF], then logging won't be enabled. It won't matter if
/// you call any of the other logging functions in this module, or directly use the
/// [tracing::info!], [tracing::debug!], etc. macros.
///
/// This is a convenience method to setup Tokio [`tracing_subscriber`] with `stdout` as
/// the output destination. This method also ensures that the [`r3bl_core::SharedWriter`]
/// is used for concurrent writes to `stdout`. You can also use the [`TracingConfig`]
/// struct to customize the behavior of the tracing setup, by choosing whether to display
/// output to `stdout`, `stderr`, or a [`r3bl_core::SharedWriter`]. By default, both
/// display and file logging are enabled. You can also customize the log level, and the
/// file path and prefix for the log file.
///
/// You can use the functions in this module or just use the [mod@crate::log_support]
/// functions directly, along with using [tracing::info!], [tracing::debug!], etc. macros.
///
/// If you don't want to use sophisticated logging, you can use the [file_log] function to
/// log messages to a file.
pub fn try_initialize_logging_global(
    options: impl Into<TracingConfig>,
) -> miette::Result<()> {
    let it: TracingConfig = options.into();

    // Early return if the level filter is off.
    if matches!(it.get_level_filter(), tracing_core::LevelFilter::OFF) {
        return ok!();
    }

    // Try to initialize the tracing system w/ (rolling) file log output.
    it.install_global()
}

/// Thread local subscriber, which is thread local, and you can assign different ones
/// to different threads.
/// - This is great for tests.
/// - Docs for [Thread local tracing
///   subscriber](https://docs.rs/tracing/latest/tracing/subscriber/fn.set_default.html)
/// - Configure this using the [mod@tracing_config_options] module (which converts any number of
///   arguments into [`Into<TracingConfig>`]. Look at this module for default configuration.
///
/// Logging is **DISABLED** by **default**.
///
/// If you don't call this function w/ a value other than
/// [tracing_core::LevelFilter::OFF], then logging won't be enabled. It won't matter if
/// you call any of the other logging functions in this module, or directly use the
/// [tracing::info!], [tracing::debug!], etc. macros.
///
/// Unlike [try_initialize_logging_global], this function initializes the logging system
/// per thread. This is useful when you want to have different log levels for different
/// threads, eg in different tests.
///
/// If you don't want to use sophisticated logging, you can use the [file_log] function to
/// log messages to a file.
pub fn try_initialize_logging_thread_local(
    options: impl Into<TracingConfig>,
) -> miette::Result<Option<dispatcher::DefaultGuard>> {
    let it: TracingConfig = options.into();

    // Early return if the level filter is off.
    if matches!(it.get_level_filter(), tracing_core::LevelFilter::OFF) {
        return Ok(None);
    }

    // Try to initialize the tracing system w/ (rolling) file log output.
    it.install_thread_local().map(Some)
}

/// This is a simple function that logs a message to a file. This is meant to be used when
/// there are no other logging facilities available, such as
/// [try_initialize_logging_global] or [try_initialize_logging_thread_local].
///
/// # Arguments
/// * `file_path` - The path to the file to log to. If `None`, the default path is
///   [tracing_config_options::DEFAULT_LOG_FILE_NAME].
/// * `message` - The message to log.
pub fn file_log(file_path: Option<&Path>, message: &str) {
    let file_path =
        file_path.unwrap_or(Path::new(tracing_config_options::DEFAULT_LOG_FILE_NAME));
    let message = if message.ends_with('\n') {
        message.to_string()
    } else {
        format!("{}\n", message)
    };
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)
        .unwrap();
    file.write_all(message.as_bytes()).unwrap();
}
