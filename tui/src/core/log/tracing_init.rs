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

use tracing_core::LevelFilter;
use tracing_subscriber::{registry::LookupSpan, Layer};

use super::{DisplayPreference, WriterConfig};
use crate::log::{rolling_file_appender_impl, tracing_config::TracingConfig};

/// Avoid gnarly type annotations by using a macro to create the `fmt` layer. Note that
/// [tracing_subscriber::fmt::format::Pretty] and
/// [tracing_subscriber::fmt::format::Compact] are mutually exclusive.
#[macro_export]
macro_rules! create_fmt {
    () => {
        tracing_subscriber::fmt::layer()
            .event_format($crate::CustomEventFormatter::default())
        //     .compact()
        //     .without_time()
        //     .with_thread_ids(false)
        //     .with_thread_names(false)
        //     .with_target(false)
        //     .with_file(false)
        //     .with_line_number(false)
        //     .with_ansi(true)
    };
}

/// Type alias for a boxed layer.
pub type DynLayer<S> = dyn Layer<S> + Send + Sync + 'static;

/// Returns the layers. This does not initialize the tracing system. Don't forget to do
/// this manually, by calling `init` on the returned layers.
///
/// For example, once you have the layers, you can run the following:
/// `try_create_layers(..).map(|layers|
/// tracing_subscriber::registry().with(layers).init());`
pub fn try_create_layers(
    tracing_config: TracingConfig,
) -> miette::Result<Option<Vec<Box<DynLayer<tracing_subscriber::Registry>>>>> {
    // Create the layers based on the writer configuration.
    let layers = {
        let mut return_it: Vec<Box<DynLayer<tracing_subscriber::Registry>>> = vec![];

        // Set the level filter from the tracing configuration. This is needed if you add
        // more layers, like OpenTelemetry, which don't have a level filter.
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
    use super::*;
    use crate::try_create_temp_dir;

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
        let dir = try_create_temp_dir().unwrap();
        let file_path = dir.join("my_temp_log_file.log");
        let file_path = file_path.to_str().unwrap().to_string();

        println!("file_path: {file_path}");

        let level_filter = LevelFilter::DEBUG;
        let writer_config = WriterConfig::File(file_path.clone());
        let layer: Option<Box<DynLayer<tracing_subscriber::Registry>>> =
            try_create_file_layer(level_filter, writer_config).unwrap();

        assert!(layer.is_some());
        assert!(std::path::Path::new(&file_path).exists());
    }

    #[test]
    fn test_try_create_both_layers() {
        let dir = try_create_temp_dir().unwrap();
        let file_path = dir.join("my_temp_log_file.log");
        let file_path = file_path.to_str().unwrap().to_string();

        let tracing_config = TracingConfig {
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

#[cfg(test)]
mod fixtures {
    use crate::custom_event_formatter_constants::*;

    /// See [crate::CustomEventFormatter] for more details.
    pub fn get_expected() -> Vec<String> {
        vec![
            format!("{ERROR_SIGIL}{LEVEL_SUFFIX}"),
            format!("{ERROR_SIGIL}{LEVEL_SUFFIX}"),
            format!("{ERROR_SIGIL}{LEVEL_SUFFIX}"),
            format!("{ERROR_SIGIL}{LEVEL_SUFFIX}"),
        ]
    }
}

#[cfg(test)]
mod test_tracing_shared_writer_output {
    use smallvec::smallvec;

    use super::{fixtures::get_expected, *};
    use crate::{join, InlineString, InlineVec, LineStateControlSignal, SharedWriter};

    #[tokio::test]
    #[allow(clippy::needless_return)]
    async fn test_shared_writer_output() {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1_000);

        // Create a new tracing layer with stdout.
        let display_pref = DisplayPreference::SharedWriter(SharedWriter::new(sender));
        let default_guard = TracingConfig {
            writer_config: WriterConfig::Display(display_pref),
            level_filter: LevelFilter::DEBUG,
        }
        .install_thread_local()
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
        let mut output: InlineVec<InlineString> = smallvec![];
        while let Some(LineStateControlSignal::Line(line)) = receiver.recv().await {
            let it = line.trim();
            output.push(it.into());
        }

        let output = join!(
            from: output,
            each: item,
            delim: "\n",
            format: "{item}",
        );

        println!("output: {output}");

        for it in get_expected().iter() {
            assert!(output.contains(it));
        }

        drop(default_guard)
    }
}
