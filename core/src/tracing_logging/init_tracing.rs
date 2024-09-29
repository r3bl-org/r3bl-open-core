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

//! # [init_tracing]
//!
//! This is a convenience method to setup Tokio [`tracing_subscriber`] with `stdout` as
//! the output destination. This method also ensures that the [`crate::SharedWriter`] is
//! used for concurrent writes to `stdout`. You can also use the [`TracingConfig`] struct
//! to customize the behavior of the tracing setup, by choosing whether to display output
//! to `stdout`, `stderr`, or a [`crate::SharedWriter`]. By default, both display and file
//! logging are enabled. You can also customize the log level, and the file path and
//! prefix for the log file.

use tracing::dispatcher;
use tracing_core::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt,
                         registry::LookupSpan,
                         util::SubscriberInitExt,
                         Layer};

use super::{DisplayPreference, TracingScope, WriterConfig};
use crate::tracing_logging::{rolling_file_appender_impl, tracing_config::TracingConfig};

/// Avoid gnarly type annotations by using a macro to create the `fmt` layer. Note that
/// [tracing_subscriber::fmt::format::Pretty] and
/// [tracing_subscriber::fmt::format::Compact] are mutually exclusive.
#[macro_export]
macro_rules! create_fmt {
    () => {
        tracing_subscriber::fmt::layer()
            .compact()
            .without_time()
            .with_thread_ids(false)
            .with_thread_names(false)
            .with_target(false)
            .with_file(false)
            .with_line_number(false)
            .with_ansi(true)
    };
}

/// Type alias for a boxed layer.
pub type DynLayer<S> = dyn Layer<S> + Send + Sync + 'static;

/// Simply initialize the tracing system with the provided [TracingConfig]. This will set
/// either (depending on its [TracingScope]):
/// 1. Global default subscriber, which once set, can't be unset or changed.
///    - This is great for apps.
///    - Docs for [Global default tracing
///      subscriber](https://docs.rs/tracing/latest/tracing/subscriber/fn.set_global_default.html)
/// 2. Thread local subscriber, which is thread local, and you can assign different ones
///    to different threads.
///    - This is great for tests.
///    - Docs for [Thread local tracing
///      subscriber](https://docs.rs/tracing/latest/tracing/subscriber/fn.set_default.html)
///
/// # Return
/// 1. If you set the [TracingScope] to [TracingScope::ThreadLocal], then this function
///    will return a [tracing::dispatcher::DefaultGuard]. You should drop this guard when
///    you're done with the tracing system. This will reset the tracing system to its
///    previous state for that thread.
/// 2. If you set the [TracingScope] to [TracingScope::Global], then this function will
///    return [`None`].
pub fn init_tracing(
    tracing_config: TracingConfig,
) -> miette::Result<Option<dispatcher::DefaultGuard>> {
    let scope = tracing_config.scope;
    try_create_layers(tracing_config).map(|layers| match scope {
        TracingScope::Global => {
            tracing_subscriber::registry().with(layers).init();
            None
        }
        TracingScope::ThreadLocal => {
            let it = tracing_subscriber::registry().with(layers).set_default();
            Some(it)
        }
    })
}

/// Returns the layers. This does not initialize the tracing system. Don't forget to do
/// this manually, by calling `init` on the returned layers.
///
/// For example, once you have the layers, you can run the following:
/// `try_create_layers(..).map(|layers| tracing_subscriber::registry().with(layers).init());`
pub fn try_create_layers(
    tracing_config: TracingConfig,
) -> miette::Result<Option<Vec<Box<DynLayer<tracing_subscriber::Registry>>>>> {
    // Create the layers based on the writer configuration.
    let layers = {
        let mut return_it: Vec<Box<DynLayer<tracing_subscriber::Registry>>> = vec![];

        // Set the level filter from the tracing configuration. This is needed if you add more
        // layers, like OpenTelemetry, which don't have a level filter.
        return_it.push(Box::new(tracing_config.get_level_filter()));

        // The following is another way of setting the level filter, if you want to
        // specify log level using env vars, as an override for the cli args.
        // ```
        // use tracing_subscriber::EnvFilter;
        // layers.push(Box::new(
        //     EnvFilter::from_default_env().add_directive(tracing_config.level),
        // ));
        // ``

        let _ = try_create_display_layer(
            tracing_config.get_level_filter(),
            tracing_config.get_writer_config(),
        )?
        .map(|layer| return_it.push(layer));

        let _ = try_create_file_layer(
            tracing_config.get_level_filter(),
            tracing_config.get_writer_config(),
        )?
        .map(|layer| return_it.push(layer));

        return_it
    };

    // Return the layers.
    Ok(Some(layers))
}

/// This erases the concrete type of the writer, and returns a boxed layer.
///
/// This is useful for composition of layers. There's more info in the docs
/// [here](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/index.html#runtime-configuration-with-layers).
pub fn try_create_display_layer<S>(
    level_filter: LevelFilter,
    writer_config: WriterConfig,
) -> miette::Result<Option<Box<DynLayer<S>>>>
where
    S: tracing_core::Subscriber,
    for<'a> S: LookupSpan<'a>,
{
    // Shared configuration regardless of where logs are output to.
    let fmt_layer = create_fmt!();

    // Configure the writer based on the desired log target, and return it.
    Ok(match writer_config {
        WriterConfig::DisplayAndFile(display_pref, _)
        | WriterConfig::Display(display_pref) => match display_pref {
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
        },
        _ => None,
    })
}

/// This erases the concrete type of the writer, and returns a boxed layer.
///
/// This is useful for composition of layers. There's more info in the docs
/// [here](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/index.html#runtime-configuration-with-layers).
pub fn try_create_file_layer<S>(
    level_filter: LevelFilter,
    writer_config: WriterConfig,
) -> miette::Result<Option<Box<DynLayer<S>>>>
where
    S: tracing_core::Subscriber,
    for<'a> S: LookupSpan<'a>,
{
    // Shared configuration regardless of where logs are output to.
    let fmt_layer = create_fmt!();

    // Configure the writer based on the desired log target, and return it.
    Ok(match writer_config {
        WriterConfig::DisplayAndFile(_, tracing_log_file_path_and_prefix)
        | WriterConfig::File(tracing_log_file_path_and_prefix) => {
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

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_try_create_display_layer() {
        let level_filter = LevelFilter::DEBUG;
        let writer_config = WriterConfig::Display(DisplayPreference::Stdout);
        let layer: Option<Box<DynLayer<tracing_subscriber::Registry>>> =
            try_create_display_layer(level_filter, writer_config).unwrap();

        assert!(layer.is_some());
    }

    #[test]
    fn test_try_create_file_layer() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("my_temp_log_file.log");
        let file_path = file_path.to_str().unwrap().to_string();

        println!("file_path: {}", file_path);

        let level_filter = LevelFilter::DEBUG;
        let writer_config = WriterConfig::File(file_path.clone());
        let layer: Option<Box<DynLayer<tracing_subscriber::Registry>>> =
            try_create_file_layer(level_filter, writer_config).unwrap();

        assert!(layer.is_some());
        assert!(std::path::Path::new(&file_path).exists());
    }

    #[test]
    fn test_try_create_both_layers() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("my_temp_log_file.log");
        let file_path = file_path.to_str().unwrap().to_string();

        let tracing_config = TracingConfig {
            scope: TracingScope::ThreadLocal,
            writer_config: WriterConfig::DisplayAndFile(
                DisplayPreference::Stdout,
                file_path.clone(),
            ),
            level_filter: LevelFilter::DEBUG,
        };

        let layers = try_create_layers(tracing_config).unwrap().unwrap();
        assert_eq!(layers.len(), 3);
        assert!(std::path::Path::new(&file_path).exists());
    }
}

/// This test works with the binary under test, which is `tracing_stdout_test_bin`. That
/// binary takes 1 string argument: "stdout" or "stderr".
///
/// If tests in this module fail, then make sure that the binary under test has been, in
/// fact, built. So, make sure to run `cargo build && cargo test` rather than just `cargo
/// test`.`
///
/// See: `tracing_stdout_test_bin.rs`
#[cfg(test)]
mod test_tracing_bin_stdio {
    use assert_cmd::Command;

    const EXPECTED: [&str; 4] = ["error", "warn", "info", "debug"];

    #[test]
    fn stdout() {
        let output = Command::cargo_bin("tracing_test_bin")
            .unwrap()
            .arg("stdout")
            .ok()
            .unwrap();

        let output = String::from_utf8_lossy(output.stdout.as_slice());
        for it in EXPECTED.iter() {
            assert!(output.contains(it));
        }
    }

    #[test]
    fn stderr() {
        let output = Command::cargo_bin("tracing_test_bin")
            .unwrap()
            .arg("stderr")
            .ok()
            .unwrap();

        let output = String::from_utf8_lossy(output.stderr.as_slice());
        for it in EXPECTED.iter() {
            assert!(output.contains(it));
        }
    }
}

#[cfg(test)]
mod test_tracing_shared_writer_output {
    use super::*;
    use crate::{LineStateControlSignal, SharedWriter};

    const EXPECTED: [&str; 4] = ["error", "warn", "info", "debug"];

    #[tokio::test]
    #[allow(clippy::needless_return)]
    async fn test_shared_writer_output() {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1_000);

        // Create a new tracing layer with stdout.
        let display_pref = DisplayPreference::SharedWriter(SharedWriter::new(sender));
        let default_guard = init_tracing(TracingConfig {
            scope: TracingScope::ThreadLocal,
            writer_config: WriterConfig::Display(display_pref),
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

        // Shutdown the channel.
        receiver.close();

        // Check the output.
        let mut output = vec![];
        while let Some(LineStateControlSignal::Line(line)) = receiver.recv().await {
            output.push(String::from_utf8_lossy(&line).trim().to_string());
        }
        let output = output.join("\n");

        println!("output: {}", output);

        for it in EXPECTED.iter() {
            assert!(output.contains(it));
        }

        drop(default_guard)
    }
}
