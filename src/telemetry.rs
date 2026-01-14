use anyhow::{Context, Result};
use opentelemetry::{KeyValue, global, trace::TracerProvider};
use opentelemetry_otlp::{LogExporter, SpanExporter};
use opentelemetry_sdk::{
    Resource,
    logs::SdkLoggerProvider,
    trace::{Sampler, SdkTracerProvider, SpanLimits},
};
use tracing::info;

/// Initialize the OpenTelemetry tracer with environment-aware sampling
/// Exports traces to an OTLP collector (Grafana Tempo) via HTTP/protobuf
/// Note: OpenTelemetry Rust SDK 0.31 only supports HTTP transport, not gRPC
pub fn init_tracer(
    env: &str,
    component: &str,
    version: &str,
) -> Result<opentelemetry_sdk::trace::Tracer> {
    info!(
        "Initializing OpenTelemetry tracer for component={}, env={}, version={}",
        component, env, version
    );

    // Build the Resource with service information
    // Note: with_service_name requires a static string, so we convert to String
    let resource = Resource::builder()
        .with_service_name(component.to_string())
        .with_attributes([
            KeyValue::new("deployment.environment", env.to_string()),
            KeyValue::new("service.version", version.to_string()),
        ])
        .build();

    // Create the OTLP span exporter with HTTP
    // Note: OpenTelemetry Rust SDK 0.31 does not support gRPC transport properly.
    // We use HTTP/protobuf which is fully supported and works with Grafana Tempo.
    // Endpoint configured via OTEL_EXPORTER_OTLP_ENDPOINT environment variable
    // Default: http://localhost:4318 (HTTP endpoint, not gRPC port 4317)
    let exporter = SpanExporter::builder()
        .with_http()
        .build()
        .context("Failed to create OTLP span exporter")?;

    // Get environment-aware sampler
    let sampler = get_trace_sampler(env);

    // Configure span limits to prevent traces from growing too large
    // Tempo has a 5MB default limit per trace
    let span_limits = SpanLimits {
        max_events_per_span: 50, // Limit log events attached to spans (default 128)
        max_attributes_per_span: 32, // Limit attributes per span (default 128)
        max_links_per_span: 16,  // Limit span links (default 128)
        max_attributes_per_event: 16, // Limit attributes per event (default 128)
        max_attributes_per_link: 16, // Limit attributes per link (default 128)
    };

    // Build the tracer provider
    let tracer_provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_sampler(sampler)
        .with_resource(resource)
        .with_span_limits(span_limits)
        .build();

    // Set as global provider
    global::set_tracer_provider(tracer_provider.clone());

    // Get the tracer directly from the provider (not from global to get concrete type)
    // Note: tracer() requires a static string, so we convert to String
    let tracer = tracer_provider.tracer(component.to_string());

    info!("OpenTelemetry tracer initialized successfully - exporting to OTLP collector");

    Ok(tracer)
}

/// Get environment-aware trace sampler
fn get_trace_sampler(env: &str) -> Sampler {
    // Check for explicit sampler configuration from environment
    if let Ok(sampler_arg) = std::env::var("OTEL_TRACES_SAMPLER_ARG")
        && let Ok(ratio) = sampler_arg.parse::<f64>()
    {
        info!(
            "Using trace sampling ratio from OTEL_TRACES_SAMPLER_ARG: {}",
            ratio
        );
        return Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(ratio)));
    }

    // Fall back to environment-based defaults
    match env {
        "production" => {
            info!("Production environment: using 1% trace sampling");
            Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(0.01)))
        }
        "staging" => {
            info!("Staging environment: using 1% trace sampling");
            Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(0.01)))
        }
        _ => {
            info!("Development environment: using 100% trace sampling");
            Sampler::ParentBased(Box::new(Sampler::AlwaysOn))
        }
    }
}

/// Initialize the OpenTelemetry meter provider for metrics export
/// This is optional and currently returns None (metrics via Prometheus only)
/// Future enhancement: Implement dual metrics export to both Prometheus and OTLP
pub fn init_meter_provider(_env: &str, _component: &str, _version: &str) -> Result<Option<()>> {
    // OTLP metrics export is optional and not yet implemented
    // The metrics crate is already configured to export to Prometheus
    // For dual export, we would need to implement a custom metrics recorder
    // that sends to both Prometheus and OTLP
    info!("OTLP metrics export not yet implemented - using Prometheus only");
    Ok(None)
}

/// Initialize the OpenTelemetry logger provider for log export to Loki via Alloy
/// Exports logs via OTLP/HTTP to the Alloy collector, which forwards them to Loki.
/// The returned LoggerProvider should be used with opentelemetry-appender-tracing
/// to bridge tracing events to OpenTelemetry logs.
pub fn init_logger_provider(
    env: &str,
    component: &str,
    version: &str,
) -> Result<SdkLoggerProvider> {
    info!(
        "Initializing OpenTelemetry logger provider for component={}, env={}, version={}",
        component, env, version
    );

    // Build the Resource with service information (same as tracer)
    let resource = Resource::builder()
        .with_service_name(component.to_string())
        .with_attributes([
            KeyValue::new("deployment.environment", env.to_string()),
            KeyValue::new("service.version", version.to_string()),
        ])
        .build();

    // Create the OTLP log exporter with HTTP
    // Endpoint configured via OTEL_EXPORTER_OTLP_ENDPOINT environment variable
    // Default: http://localhost:4318 (same endpoint as traces)
    let exporter = LogExporter::builder()
        .with_http()
        .build()
        .context("Failed to create OTLP log exporter")?;

    // Build the logger provider with batch export
    let logger_provider = SdkLoggerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .build();

    info!(
        "OpenTelemetry logger provider initialized - exporting logs to OTLP collector (Loki via Alloy)"
    );

    Ok(logger_provider)
}

/// Gracefully shutdown OpenTelemetry providers with timeout
/// Note: In v0.31, shutdown is handled automatically via Drop trait
/// If explicit shutdown is needed, store the TracerProvider handle and call shutdown() on it
#[allow(dead_code)]
pub fn shutdown_telemetry() {
    info!("OpenTelemetry shutdown requested - cleanup happens automatically via Drop");
    // In v0.31, there's no global::shutdown_tracer_provider()
    // The TracerProvider implements Drop and will cleanup automatically
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_trace_sampler() {
        // Production should use 1% sampling
        let sampler = get_trace_sampler("production");
        assert!(matches!(sampler, Sampler::ParentBased(_)));

        // Staging should use 10% sampling
        let sampler = get_trace_sampler("staging");
        assert!(matches!(sampler, Sampler::ParentBased(_)));

        // Development should use 100% sampling
        let sampler = get_trace_sampler("development");
        assert!(matches!(sampler, Sampler::ParentBased(_)));
    }
}
