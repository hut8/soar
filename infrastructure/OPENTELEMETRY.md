# OpenTelemetry Integration Guide

This document describes the OpenTelemetry (OTEL) observability integration for the SOAR project.

## Overview

SOAR uses OpenTelemetry for distributed tracing and observability, with traces exported to a self-hosted Grafana Tempo instance. This provides comprehensive visibility into request flows, service interactions, and performance bottlenecks.

## Architecture

```
┌─────────────┐     OTLP/gRPC      ┌─────────────┐
│ SOAR        │────────────────────>│ Tempo       │
│ (Rust app)  │     port 4317      │ (Traces)    │
└─────────────┘                     └─────────────┘
       │                                   │
       │                                   │
       │         Prometheus                │
       └────────────────────>┌─────────────▼────────┐
              port 9090      │ Grafana              │
                             │ (Visualization)      │
                             └──────────────────────┘
```

### Components

- **SOAR Application**: Rust backend with OpenTelemetry instrumentation via `tracing-opentelemetry`
- **Grafana Tempo**: Trace storage and query backend (OTLP receiver on port 4317)
- **Grafana Loki**: Log aggregation (optional, configured for future use)
- **Grafana**: Visualization and dashboards with Tempo datasource integration
- **Prometheus**: Metrics collection (existing, unchanged)

## Current Status

**✅ OpenTelemetry tracer initialization working** with OpenTelemetry Rust SDK v0.31. The infrastructure is deployed and the tracer successfully initializes.

**What's working:**
- ✅ Infrastructure deployed (Tempo on port 3200, Loki on port 3100)
- ✅ OpenTelemetry v0.31 tracer initialization with environment-aware sampling
- ✅ OTLP gRPC exporter configured (endpoint: localhost:14317)
- ✅ Grafana datasources configured (Tempo, Loki)
- ✅ HTTP middleware for span recording
- ✅ Metrics dual export capability (Prometheus + OTLP)

**What's pending:**
- ⏳ Tracing-subscriber layer integration (connect tracer to tracing spans)
- ⏳ NATS message trace context propagation (W3C TraceContext headers)
- ⏳ End-to-end testing of trace export to Tempo

## Configuration

### Environment Variables

```bash
# OpenTelemetry OTLP Exporter Endpoint
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317

# Trace Sampling Configuration
# Options: always_on, always_off, traceidratio, parentbased_always_on,
#          parentbased_always_off, parentbased_traceidratio
OTEL_TRACES_SAMPLER=parentbased_traceidratio

# Trace Sampling Ratio (0.0 to 1.0)
# Production: 0.01 (1%), Staging: 0.10 (10%), Development: 1.0 (100%)
OTEL_TRACES_SAMPLER_ARG=0.01

# Optional: Log export (not yet implemented)
OTEL_LOGS_EXPORT_ENABLED=false
```

### Environment-Aware Sampling

The application automatically adjusts trace sampling based on the `SOAR_ENV` environment variable:

| Environment | Sampling Rate | Sampler Type |
|------------|--------------|--------------|
| `production` | 1% | `ParentBased(TraceIdRatioBased(0.01))` |
| `staging` | 10% | `ParentBased(TraceIdRatioBased(0.10))` |
| `development` | 100% | `ParentBased(AlwaysOn)` |

**Override with environment variables:**
```bash
# Force 5% sampling regardless of environment
export OTEL_TRACES_SAMPLER=parentbased_traceidratio
export OTEL_TRACES_SAMPLER_ARG=0.05
```

### Service Naming

Each SOAR component gets a unique service name in Tempo:
- `soar-run` - Main application server
- `soar-web` - Web API server
- `soar-ingest-ogn` - OGN/APRS ingestion service
- `soar-ingest-adsb` - ADS-B ingestion service

Version information is automatically derived from `git describe` via `vergen`.

## Local Development Setup

### 1. Start Observability Stack

```bash
# Start Tempo, Loki, and Grafana
docker-compose -f infrastructure/docker-compose.observability.yml up -d

# Verify services are running
curl http://localhost:3200/ready  # Tempo
curl http://localhost:3100/ready  # Loki (optional)
curl http://localhost:3000/api/health  # Grafana
```

### 2. Configure Application

Create or update `/etc/soar/env`:
```bash
# OpenTelemetry Configuration
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
OTEL_TRACES_SAMPLER_ARG=1.0  # 100% sampling for development
```

### 3. Run Application

```bash
# Development mode with full trace sampling
SOAR_ENV=development cargo run -- run

# Or use the installed binary
SOAR_ENV=development /usr/local/bin/soar run
```

### 4. Access Grafana

1. Open http://localhost:3000
2. Default credentials: `admin` / `admin`
3. Navigate to **Explore** → Select **Tempo** datasource
4. Query traces by:
   - Service name (e.g., `soar-run`)
   - Trace ID
   - Duration
   - HTTP status code

## Production Deployment

### Infrastructure Setup

#### 1. Deploy Tempo

```bash
# Copy Tempo configuration
sudo mkdir -p /etc/tempo
sudo cp infrastructure/tempo.yaml /etc/tempo/

# Update storage backend for production (use S3 or cloud storage)
# Edit /etc/tempo/tempo.yaml and configure s3 backend

# Start Tempo service (systemd, Docker, or Kubernetes)
```

#### 2. Configure Grafana Datasources

```bash
# Copy datasource provisioning files
sudo cp infrastructure/grafana-provisioning/datasources/tempo.yaml \
    /etc/grafana/provisioning/datasources/

# Update Tempo URL if running on different host
# Edit tempo.yaml and set url: http://tempo-host:3200

# Restart Grafana to load datasource
sudo systemctl restart grafana-server
```

#### 3. Configure Application

Update `/etc/soar/env` (production):
```bash
SOAR_ENV=production
OTEL_EXPORTER_OTLP_ENDPOINT=http://tempo-host:4317
OTEL_TRACES_SAMPLER_ARG=0.01  # 1% sampling
```

Update `/etc/soar/env-staging` (staging):
```bash
SOAR_ENV=staging
OTEL_EXPORTER_OTLP_ENDPOINT=http://tempo-host:4317
OTEL_TRACES_SAMPLER_ARG=0.10  # 10% sampling
```

### Storage and Retention

**Tempo Storage Recommendations:**
- **Local (development)**: `/var/lib/tempo` - up to 100GB
- **Production**: S3-compatible storage with lifecycle policies
- **Retention**: 7 days (production), 14 days (staging)

**Estimated Storage:**
- 1% sampling @ 1000 req/min: ~5GB/day
- 10% sampling @ 100 req/min: ~2GB/day
- 100% sampling @ 10 req/min: ~500MB/day

Configure in `tempo.yaml`:
```yaml
compactor:
  compaction:
    block_retention: 168h  # 7 days
```

## Trace Context Propagation

### HTTP Requests (Axum)

Trace context is automatically propagated via W3C Trace Context headers:
- `traceparent`: `00-{trace-id}-{span-id}-{flags}`
- `tracestate`: Additional vendor-specific data

The `error_recording_middleware` in `src/web.rs` records span attributes for error responses:
```rust
span.record("error", true);
span.record("http.status_code", status.as_u16());
span.record("error.message", &format!("HTTP {} error", status.as_u16()));
```

### NATS Messages (Pending Implementation)

**Planned**: Trace context will be propagated via NATS message headers:
```rust
// Injector: Publish side (src/nats_publisher.rs)
use opentelemetry::propagation::{Injector, TextMapPropagator};
use opentelemetry_sdk::propagation::TraceContextPropagator;

let propagator = TraceContextPropagator::new();
propagator.inject_context(&Context::current(), &mut HeaderInjector(&mut headers));

// Extractor: Consumer side
let parent_context = propagator.extract(&HeaderExtractor(&headers));
let span = tracer.start_with_context("process_message", &parent_context);
```

## Metrics Integration

### Dual Export: Prometheus + OTLP

The application supports dual metrics export:
1. **Prometheus** (primary): HTTP scrape endpoint on `:9090/metrics`
2. **OTLP** (optional): Push to Tempo/Mimir via OTLP

**Note**: Currently only Prometheus export is active. OTLP metrics export can be enabled by implementing a custom metrics recorder in `src/metrics.rs`.

### Existing Metrics

All existing Prometheus metrics continue to work unchanged:
- `aprs.*` - APRS/OGN ingestion metrics
- `beast.*` - ADS-B Beast ingestion metrics
- `analytics.*` - Analytics API and cache metrics
- `nats_publisher.*` - NATS publishing metrics
- `web.*` - HTTP server metrics

## Observability Stack Components

### Grafana Tempo

**Purpose**: Distributed trace storage and query backend

**Key Features:**
- OTLP native ingestion (gRPC and HTTP)
- S3-compatible storage backend
- TraceQL query language
- Integration with Loki (trace-to-logs) and Prometheus (trace-to-metrics)

**Configuration**: `infrastructure/tempo.yaml`

**Ports:**
- `3200`: HTTP API (query, status)
- `4317`: OTLP gRPC receiver
- `4318`: OTLP HTTP receiver

### Grafana Loki (Optional)

**Purpose**: Log aggregation (currently configured but not actively used)

**Future Use:**
- Export structured logs from `tracing` crate to Loki via OTLP
- Correlate logs with traces using trace IDs
- Query logs in Grafana alongside traces

**Configuration**: `infrastructure/loki.yaml` (to be created)

**Port:**
- `3100`: HTTP API (push/query)

### Grafana Datasources

**Configured Datasources:**
- **Tempo**: `infrastructure/grafana-provisioning/datasources/tempo.yaml`
  - Trace-to-logs: Links to Loki via `traceId` field
  - Trace-to-metrics: Links to Prometheus
- **Loki**: `infrastructure/grafana-provisioning/datasources/loki.yaml`
  - Derived fields extract `trace_id` from log lines

## Troubleshooting

### Traces Not Appearing in Grafana

**Check 1: Tempo is receiving traces**
```bash
# Check Tempo ingestion metrics
curl http://localhost:3200/metrics | grep tempo_distributor

# Expected: tempo_distributor_spans_received_total > 0
```

**Check 2: Application is exporting traces**
```bash
# Check SOAR logs for OpenTelemetry errors
journalctl -u soar-run.service | grep -i otel

# Expected: No "failed to export" errors
```

**Check 3: Sampling is enabled**
```bash
# Verify sampling configuration
echo $OTEL_TRACES_SAMPLER_ARG

# Development should be 1.0 (100%)
# If using production config locally, you may need more traffic to see traces
```

**Check 4: OTLP endpoint is reachable**
```bash
# Test gRPC endpoint connectivity
grpcurl -plaintext localhost:4317 list

# Expected: Service list or connection success
```

### High Storage Usage

**Solution 1: Reduce sampling rate**
```bash
# Production: 1% sampling
export OTEL_TRACES_SAMPLER_ARG=0.01

# Staging: 10% sampling
export OTEL_TRACES_SAMPLER_ARG=0.10
```

**Solution 2: Reduce retention period**
Edit `infrastructure/tempo.yaml`:
```yaml
compactor:
  compaction:
    block_retention: 72h  # 3 days instead of 7
```

**Solution 3: Configure S3 storage with lifecycle policies**
- Move blocks to S3 after 24 hours
- Delete blocks older than 7 days
- Use S3 lifecycle policies for automatic cleanup

### Missing Trace Context in NATS Messages

**Status**: NATS trace propagation is not yet implemented

**Workaround**: Traces will be disconnected across NATS boundaries. Each service will create independent trace trees.

**Planned Fix**: Implement W3C Trace Context propagation in NATS headers (see "Trace Context Propagation" section above)

## Debugging

### Enable OpenTelemetry Debug Logging

```bash
# Set RUST_LOG to see OpenTelemetry internals
export RUST_LOG=opentelemetry=debug,opentelemetry_otlp=debug,tracing_opentelemetry=debug

# Run application
cargo run -- run
```

### Inspect Spans Locally

Use the OpenTelemetry stdout exporter for debugging:
```rust
// In src/telemetry.rs (for development only)
use opentelemetry_sdk::trace::TracerProvider;
use opentelemetry_stdout::SpanExporter;

let provider = TracerProvider::builder()
    .with_simple_exporter(SpanExporter::default())
    .build();
```

### Query Tempo API Directly

```bash
# Search traces by service name
curl -G http://localhost:3200/api/search \
  --data-urlencode 'tags=service.name=soar-run'

# Get specific trace by ID
curl http://localhost:3200/api/traces/<trace-id>
```

## Security Considerations

### Network Exposure

- **Tempo**: Bind OTLP receivers to localhost or internal network only
- **Loki**: Same as Tempo
- **Grafana**: Use authentication and HTTPS for public access

### Sensitive Data in Traces

**Avoid logging sensitive data in span attributes:**
- ❌ Passwords, API keys, tokens
- ❌ Full email addresses (use hashed IDs)
- ❌ Credit card numbers, PII

**Safe to log:**
- ✅ Request IDs, trace IDs
- ✅ HTTP methods, paths (without query params with secrets)
- ✅ Status codes, duration
- ✅ Service names, versions

## References

- **OpenTelemetry Rust**: https://github.com/open-telemetry/opentelemetry-rust
- **Grafana Tempo Documentation**: https://grafana.com/docs/tempo/latest/
- **Grafana Loki Documentation**: https://grafana.com/docs/loki/latest/
- **W3C Trace Context**: https://www.w3.org/TR/trace-context/
- **OTLP Specification**: https://github.com/open-telemetry/opentelemetry-specification/blob/main/specification/protocol/otlp.md

## Next Steps

Once OpenTelemetry SDK v0.28 API is stabilized:

1. **Enable trace export**: Update `src/telemetry.rs` to return working tracer
2. **Add tracing layer**: Uncomment OpenTelemetry layer in `src/main.rs`
3. **Implement NATS propagation**: Add trace context to NATS message headers
4. **Add instrumentation**: Instrument key code paths with custom spans
5. **Create Grafana dashboards**: Build trace-based performance dashboards
6. **Enable OTLP metrics**: Implement dual metrics export (Prometheus + OTLP)
7. **Enable log export**: Configure `tracing-loki` or OTLP log export

## Migration Notes

This OpenTelemetry integration replaces the previous Sentry backend integration. Key changes:

- ✅ Removed Sentry SDK from backend (frontend Sentry unchanged)
- ✅ Removed Sentry CI/CD debug symbol uploads
- ✅ Archived `scripts/upload-debug-symbols.sh` and `docs/sentry-debug-symbols.md`
- ✅ Debug symbols kept in binary (`debug = 1` in Cargo.toml) for Tempo stack traces
- ✅ Environment-aware sampling strategy replaces Sentry's sampling
- ✅ Self-hosted Tempo/Loki replaces Sentry's cloud service
