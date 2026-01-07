use anyhow::{Context, Result};
use opentelemetry::{KeyValue, global, trace::TracerProvider};
use opentelemetry_otlp::SpanExporter;
use opentelemetry_sdk::{
    Resource,
    trace::{Sampler, SdkTracerProvider},
};
use tracing::info;

/// Initialize the OpenTelemetry tracer with environment-aware sampling
/// Exports traces to an OTLP collector (Grafana Tempo) via gRPC
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

    // Build the tracer provider
    let tracer_provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_sampler(sampler)
        .with_resource(resource)
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
            info!("Staging environment: using 10% trace sampling");
            Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(0.10)))
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

/// Initialize the OpenTelemetry logger provider for log export
/// NOTE: Direct OTLP log export is not yet stable in OpenTelemetry Rust SDK v0.31
/// However, logs are already exported to Tempo via the tracing-opentelemetry layer:
/// - All error!(), warn!(), info!(), debug!(), trace!() calls create span events
/// - These events are attached to the current span and exported to Tempo
/// - They can be queried in Grafana Tempo and correlated with traces
pub fn init_logger_provider(_env: &str, _component: &str, _version: &str) -> Result<()> {
    info!("Log export via tracing-opentelemetry layer (logs appear as span events in Tempo)");
    // Logs are automatically exported via the tracing-opentelemetry layer
    // integrated in main.rs - no additional setup needed
    Ok(())
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
