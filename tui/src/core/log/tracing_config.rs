/*
 *   Copyright (c) 2024-2025 R3BL LLC
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

use tracing::dispatcher;
use tracing_core::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use super::try_create_layers;
use crate::SharedWriter;

/// - `tracing_log_file_path_and_prefix`: [String] is the file path and prefix to use for
///   the log file. Eg: `/tmp/tcp_api_server` or `tcp_api_server`.
/// - `DisplayPreference`: [`DisplayPreference`] is the preferred display to use for
///   logging.
#[derive(Debug, Clone, PartialEq)]
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

/// This is needed for testing.
impl PartialEq for DisplayPreference {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (DisplayPreference::Stdout, DisplayPreference::Stdout)
            | (DisplayPreference::Stderr, DisplayPreference::Stderr) => true,
            (
                DisplayPreference::SharedWriter(sw_lhs),
                DisplayPreference::SharedWriter(sw_rhs),
            ) => sw_lhs == sw_rhs,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests_partial_eq_display_preference {
    use super::*;

    fn make_shared_writer() -> SharedWriter {
        let (line_sender, _) = tokio::sync::mpsc::channel(1_000);
        SharedWriter::new(line_sender)
    }

    #[test]
    fn test_display_preference_partial_eq_stdout() {
        assert_eq!(DisplayPreference::Stdout, DisplayPreference::Stdout);
        assert_ne!(DisplayPreference::Stdout, DisplayPreference::Stderr);
        assert_ne!(
            DisplayPreference::Stdout,
            DisplayPreference::SharedWriter(make_shared_writer())
        );
    }

    #[test]
    fn test_display_preference_partial_eq_stderr() {
        assert_eq!(DisplayPreference::Stderr, DisplayPreference::Stderr);
        assert_ne!(DisplayPreference::Stderr, DisplayPreference::Stdout);
        assert_ne!(
            DisplayPreference::Stderr,
            DisplayPreference::SharedWriter(make_shared_writer())
        );
    }

    #[test]
    fn test_display_preference_partial_eq_shared_writer() {
        let writer_1 = make_shared_writer();
        let writer_2 = make_shared_writer();
        assert_eq!(
            DisplayPreference::SharedWriter(writer_1.clone()),
            DisplayPreference::SharedWriter(writer_1.clone())
        );
        assert_ne!(
            DisplayPreference::SharedWriter(writer_1.clone()),
            DisplayPreference::SharedWriter(writer_2.clone())
        );
        assert_ne!(
            DisplayPreference::SharedWriter(writer_1.clone()),
            DisplayPreference::Stdout
        );
        assert_ne!(
            DisplayPreference::SharedWriter(writer_1.clone()),
            DisplayPreference::Stderr
        );
    }
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

/// Configure the tracing logging to suit your needs. You can display the logs to a:
/// 1. file,
/// 2. stdout, stderr, or a shared writer,
/// 3. both.
///
/// This configuration also allows you to set the log level.
///
/// # Fields
/// - `writer_config`: [`WriterConfig`] to choose where to write the logs.
/// - `level`: [`LevelFilter`] - The log level to use for tracing.
///
/// # Methods
/// You can use the following methods to initialize the tracing system with this
/// configuration:
/// - [`Self::install_global()`]: This will install the global tracing subscriber. There
///   can only be one, and it can't be unset, once set, or changed.
/// - [`Self::install_thread_local()`]: This will install the tracing subscriber for the
///   current thread.
#[derive(Debug)]
pub struct TracingConfig {
    pub writer_config: WriterConfig,
    pub level_filter: LevelFilter,
}

/// Simply initialize the tracing system with the provided [`TracingConfig`]. You can
/// either use a local or global subscriber, based on your needs.
///
/// 1. Global default subscriber, which once set, can't be unset or changed.
///    - This is great for apps.
///    - Docs for [Global default tracing subscriber](https://docs.rs/tracing/latest/tracing/subscriber/fn.set_global_default.html)
/// 2. Thread local subscriber, which is thread local, and you can assign different ones
///    to different threads.
///    - This is great for tests.
///    - Docs for [Thread local tracing subscriber](https://docs.rs/tracing/latest/tracing/subscriber/fn.set_default.html)
impl TracingConfig {
    /// Thread local is use in tests, where each test should have its own log file or
    /// stdout, etc. This is set per thread. So you can have more than one, assuming you
    /// have more than one thread.
    ///
    /// This function will return a [`tracing::dispatcher::DefaultGuard`]. You should drop
    /// this guard when you're done with the tracing system. This will reset the tracing
    /// system to its previous state for that thread.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file writer cannot be created (invalid path, permissions)
    /// - The tracing layer cannot be initialized
    pub fn install_thread_local(self) -> miette::Result<dispatcher::DefaultGuard> {
        try_create_layers(&self)
            .map(|layers| tracing_subscriber::registry().with(layers).set_default())
    }

    /// Global scope is used in production, for an app that needs to log to a file or
    /// stdout, etc. Once set, this can't be unset or changed.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file writer cannot be created (invalid path, permissions)
    /// - The tracing layer cannot be initialized
    /// - A global subscriber has already been set
    pub fn install_global(self) -> miette::Result<()> {
        try_create_layers(&self)
            .map(|layers| tracing_subscriber::registry().with(layers).init())
    }
}

impl TracingConfig {
    #[must_use]
    pub fn get_writer_config(&self) -> WriterConfig { self.writer_config.clone() }

    #[must_use]
    pub fn get_level_filter(&self) -> LevelFilter { self.level_filter }
}
