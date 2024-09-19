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

use tracing_core::LevelFilter;

use crate::{tracing_logging::writer_arg::WriterArg, SharedWriter};

/// Fields:
/// - `writers`: Vec<[WriterArg]> - Zero or more writers to use for
///   tracing.
/// - `level`: [tracing::Level] - The log level to use for tracing.
/// - `tracing_log_file_path_and_prefix`: [String] - The file path and prefix to use for
///   the log file. Eg: `/tmp/tcp_api_server` or `tcp_api_server`.
#[derive(Clone, Debug)]
pub struct TracingConfig {
    pub writer_args: Vec<WriterArg>,
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
    pub fn new(preferred_display: DisplayPreference) -> Self {
        Self {
            writer_args: vec![WriterArg::File, WriterArg::Stdout],
            level: tracing::Level::DEBUG,
            tracing_log_file_path_and_prefix: "tracing_log_file_debug.log".to_string(),
            preferred_display,
        }
    }

    pub fn get_level_filter(&self) -> LevelFilter {
        tracing_subscriber::filter::LevelFilter::from_level(self.level)
    }
}
