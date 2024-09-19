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

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

use super::WriterConfig;
use crate::{tracing_logging::tracing_config::TracingConfig, SharedWriter};

#[derive(Clone)]
pub enum DisplayPreference {
    Stdout,
    Stderr,
    SharedWriter(SharedWriter),
}

pub type DynLayer<S> = dyn Layer<S> + Send + Sync + 'static;

/// Simply initialize the tracing system with the provided [TracingConfig].
pub fn init(tracing_config: TracingConfig) -> miette::Result<()> {
    try_create_layers(tracing_config)
        .map(|layers| tracing_subscriber::registry().with(layers).init())
}

/// Returns the layers. This does not initialize the tracing system. Don't forget to do
/// this manually, by calling `init` on the returned layers.
///
/// For example, once you have the layers, you can run the following:
/// `create_layers(..).map(|layers| tracing_subscriber::registry().with(layers).init());`
pub fn try_create_layers(
    tracing_config: TracingConfig,
) -> miette::Result<Option<Vec<Box<DynLayer<tracing_subscriber::Registry>>>>> {
    // Transform the `clap` crate's parsed command line arguments into a `WriterConfig`.
    let writer_config =
        match WriterConfig::try_from(tracing_config.writer_args.as_slice()) {
            Ok(it) => it,
            Err(_) => return Ok(None),
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
            .try_create_display_layer(
                level_filter,
                tracing_config.preferred_display.clone(),
            )?
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
