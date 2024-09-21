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

use std::fmt::Debug;

use r3bl_rs_utils_core::SharedWriter;
use tracing_core::LevelFilter;

/// Configure the tracing logging to suit your needs. You can display the logs to a:
/// 1. file,
/// 2. stdout, stderr, or a shared writer,
/// 3. both.
///
/// This configuration also allows you to set the log level.
///
/// You can use the [crate::init_tracing()] to initialize the tracing system with this
/// configuration.
///
/// Fields:
/// - `writer_config`: [WriterConfig] to choose where to write the logs.
/// - `level`: [tracing::Level] - The log level to use for tracing.
#[derive(Debug)]
pub struct TracingConfig {
    pub writer_config: WriterConfig,
    pub level: tracing::Level,
}

/// - `tracing_log_file_path_and_prefix`: [String] is the file path and prefix to use for
///   the log file. Eg: `/tmp/tcp_api_server` or `tcp_api_server`.
/// - `DisplayPreference`: [DisplayPreference] is the preferred display to use for logging.
#[derive(Debug, Clone)]
pub enum WriterConfig {
    None,
    Display(
        DisplayPreference, /* Stdout, Stderr, SharedWriter(SharedWriter) */
    ),
    File(String /* tracing_log_file_path_and_prefix */),
    DisplayAndFile(
        DisplayPreference,
        String, /* tracing_log_file_path_and_prefix */
    ),
}

#[derive(Clone)]
pub enum DisplayPreference {
    Stdout,
    Stderr,
    SharedWriter(SharedWriter),
}

impl Debug for DisplayPreference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DisplayPreference::Stdout => write!(f, "Stdout"),
            DisplayPreference::Stderr => write!(f, "Stderr"),
            DisplayPreference::SharedWriter(_) => write!(f, "SharedWriter"),
        }
    }
}

impl TracingConfig {
    /// The default configuration for tracing. This will log to both the given
    /// [DisplayPreference] and a file.
    pub fn new_file_and_display(
        filename: Option<String>,
        preferred_display: DisplayPreference,
    ) -> Self {
        Self {
            writer_config: WriterConfig::DisplayAndFile(
                preferred_display,
                filename.unwrap_or_else(|| "tracing_log_file_debug.log".to_string()),
            ),
            level: tracing::Level::DEBUG,
        }
    }

    pub fn new_display(preferred_display: DisplayPreference) -> Self {
        Self {
            writer_config: WriterConfig::Display(preferred_display),
            level: tracing::Level::DEBUG,
        }
    }

    pub fn new_file(filename: Option<String>) -> Self {
        Self {
            writer_config: WriterConfig::File(
                filename.unwrap_or_else(|| "tracing_log_file_debug.log".to_string()),
            ),
            level: tracing::Level::DEBUG,
        }
    }

    pub fn get_writer_config(&self) -> WriterConfig { self.writer_config.clone() }

    pub fn get_level_filter(&self) -> LevelFilter {
        tracing_subscriber::filter::LevelFilter::from_level(self.level)
    }
}
