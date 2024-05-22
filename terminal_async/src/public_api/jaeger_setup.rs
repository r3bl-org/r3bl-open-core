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

use crate::port_availability;
use miette::IntoDiagnostic;
use opentelemetry::{global, trace::TraceError, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{runtime, trace as sdktrace, Resource};
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
use std::str::FromStr;
use tracing::Subscriber;
use tracing_subscriber::registry::LookupSpan;

/// Hostname for Jaeger.
const HOST: &str = "127.0.0.1";
/// gRPC port for Jaeger.
const PORT: u16 = 4317;

pub fn tcp_addr() -> String {
    format!("{}:{}", HOST, PORT)
}

pub fn url() -> String {
    format!("http://{}:{}", HOST, PORT)
}

pub fn get_socket_addr(
    maybe_otel_collector_endpoint: Option<std::net::SocketAddr>,
) -> miette::Result<std::net::SocketAddr> {
    let addr = match maybe_otel_collector_endpoint {
        Some(it) => it.to_string(),
        None => tcp_addr(),
    };

    let (host, port) = {
        let it = addr.splitn(2, ':').collect::<Vec<&str>>();
        if it.len() != 2 {
            return Err(miette::miette!("Invalid address"));
        }
        let host = std::net::IpAddr::from_str(it[0]).into_diagnostic()?;
        let port = it[1].parse::<u16>().into_diagnostic()?;
        (host, port)
    };

    let socket_addr = std::net::SocketAddr::new(host, port);
    Ok(socket_addr)
}

#[test]
fn test_get_socket_addr() -> miette::Result<()> {
    // Check defaults.
    let addr = get_socket_addr(None)?;
    assert_eq!(addr.port(), PORT);
    assert_eq!(
        addr.ip(),
        std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))
    );

    // Check custom address.
    let ip: std::net::SocketAddr = "127.0.0.1:12".parse().into_diagnostic()?;
    let addr = get_socket_addr(Some(ip))?;
    assert_eq!(addr.port(), 12);
    assert_eq!(
        addr.ip(),
        std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))
    );

    Ok(())
}

/// Check whether port 4317 is open for Jaeger.
pub async fn is_jaeger_up(
    maybe_otel_collector_endpoint: Option<std::net::SocketAddr>,
) -> miette::Result<bool> {
    let socket_addr = get_socket_addr(maybe_otel_collector_endpoint)?;
    match port_availability::check(socket_addr).await? {
        port_availability::Status::Occupied => Ok(true),
        port_availability::Status::Free => Ok(false),
    }
}

/// The OTLP (OpenTelemetry protocol) Exporter supports exporting logs, metrics and traces
/// in the OTLP format to the OpenTelemetry collector or other compatible backend.
///
/// The OpenTelemetry Collector offers a vendor-agnostic implementation on how to receive,
/// process, and export telemetry data. In addition, it removes the need to run, operate,
/// and maintain multiple agents/collectors in order to support open-source telemetry data
/// formats (e.g. Jaeger, Prometheus, etc.) sending to multiple open-source or commercial
/// back-ends.
///
/// Currently, this crate only support sending tracing data or metrics in OTLP via grpc
/// and http (in binary format). Supports for other format and protocol will be added in
/// the future. The details of what’s currently offering in this crate can be found in
/// this doc.
///
/// More info:
/// 1. <https://docs.rs/opentelemetry-otlp/latest/opentelemetry_otlp/#kitchen-sink-full-configuration>
/// 2. <https://broch.tech/posts/rust-tracing-opentelemetry/>
/// 3. <https://github.com/open-telemetry/opentelemetry-rust/blob/main/examples/tracing-jaeger/src/main.rs>
/// 4. <https://www.jaegertracing.io/docs/1.57/getting-started/>
fn try_init_exporter(service_name: &str) -> Result<opentelemetry_sdk::trace::Tracer, TraceError> {
    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(url()),
        )
        .with_trace_config(
            sdktrace::config().with_resource(Resource::new(vec![KeyValue::new(
                SERVICE_NAME,
                service_name.to_string(),
            )])),
        )
        .install_batch(runtime::Tokio)
}

/// 1. This will try and create a Jaeger OTel layer, and if successful, it will return it.
/// 2. If Jaeger is not up, it will return [Option::None].
/// 3. If there is a problem with detecting whether Jaeger is up, it will return an error.
pub async fn try_get_otel_layer<S>(
    service_name: &str,
    maybe_otel_collector_endpoint: Option<std::net::SocketAddr>,
) -> miette::Result<
    std::option::Option<(
        tracing_opentelemetry::OpenTelemetryLayer<S, opentelemetry_sdk::trace::Tracer>,
        DropTracer,
    )>,
>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    if is_jaeger_up(maybe_otel_collector_endpoint).await? {
        if let Ok(tracer) = try_init_exporter(service_name) {
            let it = tracing_opentelemetry::layer().with_tracer(tracer);
            return Ok(Some((it, DropTracer)));
        }
    }
    Ok(None)
}

pub struct DropTracer;

impl Drop for DropTracer {
    fn drop(&mut self) {
        global::shutdown_tracer_provider();
    }
}
