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

//! This is just a shim (thin wrapper) around the [crate::tracing_logging] module.
//!
//! You can use the functions in this module or just use the [mod@crate::init_tracing]
//! functions directly, along with using [tracing::info!], [tracing::debug!], etc. macros.
//!
//! This file is here as a convenience for backward compatibility w/ the old logging
//! system.

use crate::{ok, TracingConfig, WriterConfig};

const LOG_FILE_NAME: &str = "log.txt";

/// Logging is **DISABLED** by **default**.
///
/// If you don't call this function w/ a value other than
/// [tracing_core::LevelFilter::OFF], then logging won't be enabled. It won't matter if
/// you call any of the other logging functions in this module, or directly use the
/// [tracing::info!], [tracing::debug!], etc. macros.
///
/// This is a convenience method to setup Tokio [`tracing_subscriber`] with `stdout` as
/// the output destination. This method also ensures that the [`crate::SharedWriter`] is
/// used for concurrent writes to `stdout`. You can also use the [`TracingConfig`] struct
/// to customize the behavior of the tracing setup, by choosing whether to display output
/// to `stdout`, `stderr`, or a [`crate::SharedWriter`]. By default, both display and file
/// logging are enabled. You can also customize the log level, and the file path and
/// prefix for the log file.
pub fn try_initialize_global_logging(
    level_filter: tracing_core::LevelFilter,
) -> miette::Result<()> {
    // Early return if the level filter is off.
    if matches!(level_filter, tracing_core::LevelFilter::OFF) {
        return ok!();
    }

    // Try to initialize the tracing system w/ (rolling) file log output.
    TracingConfig {
        level_filter,
        writer_config: WriterConfig::File(LOG_FILE_NAME.to_string()),
    }
    .install_global()?;

    ok!()
}
