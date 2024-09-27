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

use r3bl_rs_utils_core::{init_tracing,
                         DisplayPreference,
                         TracingConfig,
                         TracingScope,
                         WriterConfig};
use tracing_core::LevelFilter;

/// `assert_cmd` : <https://docs.rs/assert_cmd/latest/assert_cmd/index.html>
///
/// This is the binary under test, which is tested by the `test_tracing_stdout` test.
/// It takes 1 argument: "stdout" or "stderr". Depending on the argument, it will
/// display the logs to stdout or stderr.
///
/// See: `init_tracing.rs` and `test_tracing_bin_stdio()` test.
fn main() {
    // Get the argument passed to the binary.
    let arg = std::env::args().nth(1).unwrap_or_default();
    let display_preference = match arg.as_str() {
        "stdout" => DisplayPreference::Stdout,
        "stderr" => DisplayPreference::Stderr,
        _ => DisplayPreference::Stdout,
    };

    // Create a new tracing layer with stdout.
    let default_guard = init_tracing(TracingConfig {
        scope: TracingScope::ThreadLocal,
        writer_config: WriterConfig::Display(display_preference),
        level_filter: LevelFilter::DEBUG,
    })
    .unwrap()
    .unwrap();

    // Log some messages.
    tracing::error!("error");
    tracing::warn!("warn");
    tracing::info!("info");
    tracing::debug!("debug");
    tracing::trace!("trace");

    drop(default_guard);
}
