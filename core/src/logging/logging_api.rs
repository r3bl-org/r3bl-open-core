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
//! You can use the functions in this module or just use the
//! [crate::tracing_logging::init_tracing()] function directly, along with using
//! [tracing::info!], [tracing::debug!], etc. macros.
//!
//! This file is here as a convenience for backward compatibility w/ the old logging
//! system.

use crate::{init_tracing,
            ok,
            tracing_logging::TracingScope,
            TracingConfig,
            WriterConfig};

const LOG_FILE_NAME: &str = "log.txt";

/// Logging is **DISABLED** by **default**.
///
/// If you don't call this function w/ a value other than
/// [tracing_core::LevelFilter::OFF], then logging won't be enabled. It won't matter if
/// you call any of the other logging functions in this module, or directly use the
/// [tracing::info!], [tracing::debug!], etc. macros.
pub fn try_to_set_log_level(
    level_filter: tracing_core::LevelFilter,
) -> miette::Result<()> {
    // Early return if the level filter is off.
    if matches!(level_filter, tracing_core::LevelFilter::OFF) {
        return ok!();
    }

    // Try to initialize the tracing system w/ (rolling) file log output.
    init_tracing(TracingConfig {
        scope: TracingScope::Global,
        level_filter,
        writer_config: WriterConfig::File(LOG_FILE_NAME.to_string()),
    })?;

    ok!()
}

pub fn log_info(arg: String) {
    tracing::info!(arg);
}

pub fn log_debug(arg: String) {
    tracing::debug!(arg);
}

pub fn log_warn(arg: String) {
    tracing::warn!(arg);
}

pub fn log_trace(arg: String) {
    tracing::trace!(arg);
}

pub fn log_error(arg: String) {
    tracing::error!(arg);
}
