# Distributed Tracing with OpenTelemetry and Tempo

SOAR uses OpenTelemetry for distributed tracing to help identify performance bottlenecks and understand the flow of data through the system.

## Architecture

- **Tracing Library**: `tracing` crate with `tracing-opentelemetry` integration
- **Protocol**: OTLP (OpenTelemetry Protocol) over HTTP
- **Backend**: Grafana Tempo for trace storage and visualization
- **Integration**: Traces are linked with logs (Loki) and metrics (Prometheus) in Grafana

## Configuration

### Environment Variables

```bash
# Enable OpenTelemetry tracing (default: enabled)
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318

# Control sampling rate (optional)
OTEL_TRACES_SAMPLER_ARG=1.0  # 1.0 = 100% sampling

# Environment-based sampling (automatic)
SOAR_ENV=production  # 1% sampling
SOAR_ENV=staging     # 10% sampling
SOAR_ENV=development # 100% sampling
```

### Starting Tempo

Use the observability stack via Docker Compose:

```bash
docker-compose -f docker-compose.observability.yml up -d
```

This starts:
- Tempo on port 3200 (queries) and 4318 (OTLP HTTP ingestion)
- Grafana on port 3000 with Tempo datasource pre-configured
- Loki and Prometheus (for logs and metrics correlation)

## Instrumented Components

### Message Processing Pipeline

The following key functions are instrumented:

#### APRS Message Processing
- `process_aprs_message()` - Parse and route APRS messages
- `PacketRouter::process_packet_internal()` - Route packets to specialized processors
- `AircraftPositionProcessor::process_aircraft_position()` - Process aircraft position fixes

#### ADS-B Beast Processing
- `process_beast_message()` - Parse and route ADS-B messages
- `adsb_message_to_fix()` - Convert ADS-B to position fix

#### Fix Processing
- `FixProcessor::process_aprs_packet()` - Main APRS packet processing entry point
- `FixProcessor::process_fix()` - Process pre-created fixes (e.g., from ADS-B)
- `FixProcessor::process_fix_internal()` - Internal fix processing pipeline

#### Database Operations
- `FixesRepository::insert()` - Insert fix into database
- `AircraftRepository::get_or_insert_aircraft_by_address()` - Lookup/create aircraft
- `AircraftRepository::aircraft_for_fix()` - Atomic aircraft update for fix

#### Flight Tracking
- `FlightTracker::process_and_insert_fix()` - Flight state transitions and database insertion

#### External Services
- `ElevationService::elevation()` - Terrain elevation lookups

#### API Handlers
- `get_flight_by_id()` - Retrieve flight details
- `get_aircraft_registrations_by_club()` - List club aircraft
- And many more in `src/actions/`

## Viewing Traces in Grafana

1. Open Grafana: http://localhost:3000
2. Navigate to Explore → Select "Tempo" datasource
3. Choose "Search" query type
4. Filter by:
   - **Service Name**: `run`, `web`, `ingest-ogn`, `ingest-adsb`
   - **Span Name**: Search for specific functions (e.g., `process_aprs_packet`)
   - **Duration**: Find slow traces (e.g., `> 100ms`)
   - **Status**: Find errors

### Trace Attributes

Spans include the following attributes for filtering:

- `aircraft_id` - UUID of the aircraft being processed
- `flight_id` - UUID of the flight (if assigned)
- `packet_from` - APRS packet source callsign
- `message_len` - Message size in bytes
- `callsign` - Receiver callsign

### Example Queries

**Find slow fix processing:**
```
Service: run
Span Name: process_fix_internal
Duration: > 50ms
```

**Find database operations for a specific aircraft:**
```
Service: run
Tags: aircraft_id = <uuid>
```

**Find API requests that took > 1 second:**
```
Service: web
Duration: > 1s
```

## Understanding Trace Spans

### Typical APRS Message Flow

```
process_aprs_message
├── PacketRouter::process_packet_internal
│   └── AircraftPositionProcessor::process_aircraft_position
│       └── FixProcessor::process_aprs_packet
│           ├── AircraftRepository::aircraft_for_fix
│           └── FixProcessor::process_fix_internal
│               ├── ElevationService::elevation (if enabled)
│               └── FlightTracker::process_and_insert_fix
│                   └── FixesRepository::insert
└── NATS publish (async)
```

### Typical ADS-B Message Flow

```
process_beast_message
├── decode_beast_frame
├── AircraftRepository::get_or_insert_aircraft_by_address
├── RawMessagesRepository::insert_beast
├── adsb_message_to_fix
└── FixProcessor::process_fix
    └── FixProcessor::process_fix_internal
        └── FlightTracker::process_and_insert_fix
            └── FixesRepository::insert
```

## Performance Analysis

### Common Bottlenecks

1. **Database Operations** - Look for slow `insert`, `update`, or `select` operations
2. **Elevation Lookups** - Check `ElevationService::elevation` spans
3. **Flight State Transitions** - Monitor `FlightTracker::process_and_insert_fix`
4. **External API Calls** - Geocoding, aircraft registry lookups

### Optimization Tips

- Use the "Service Graph" in Grafana to visualize service dependencies
- Use "Trace to Logs" feature to correlate traces with log messages
- Use "Trace to Metrics" to see how latency correlates with throughput
- Look for "parallel work" opportunities (multiple database calls that could be batched)

## Adding New Instrumentation

To add tracing to a new function:

```rust
use tracing::instrument;

#[instrument(skip(self, complex_param), fields(important_id = %id))]
pub async fn my_function(&self, id: Uuid, complex_param: &ComplexType) -> Result<()> {
    // Function body
}
```

### Guidelines

1. **Skip large parameters**: Use `skip()` for large structs, connections, repositories
2. **Add important fields**: Use `fields()` to add searchable attributes
3. **Use appropriate levels**: Most spans default to `tracing::Level::INFO`
4. **Keep spans focused**: Instrument at function boundaries, not inside loops

### Examples

```rust
// ✅ GOOD: Skip complex types, add searchable fields
#[instrument(skip(self, packet), fields(aircraft_id = %packet.aircraft_id))]
pub async fn process_packet(&self, packet: &Packet) -> Result<()>

// ❌ BAD: Don't skip simple IDs, don't add redundant fields
#[instrument(skip(id), fields(id = %id))]  // id already captured by default
pub async fn get_by_id(&self, id: Uuid) -> Result<()>

// ✅ GOOD: Add contextual information for database operations
#[instrument(skip(conn), fields(table = "fixes", operation = "insert"))]
async fn insert_fix(conn: &mut PgConnection, fix: &Fix) -> Result<()>
```

## Troubleshooting

### No traces appearing in Tempo

1. Check OTLP endpoint is accessible:
   ```bash
   curl http://localhost:4318/v1/traces
   ```

2. Check application logs for OpenTelemetry errors:
   ```bash
   journalctl -u soar-run | grep -i "opentelemetry\|tempo\|otlp"
   ```

3. Verify sampling rate:
   ```bash
   echo $OTEL_TRACES_SAMPLER_ARG  # Should be 1.0 for 100% sampling in dev
   ```

### High memory usage

If trace collection is using too much memory:

1. Reduce sampling rate in production: `OTEL_TRACES_SAMPLER_ARG=0.01` (1%)
2. Adjust span limits in `src/telemetry.rs` if needed
3. Check Tempo retention policy in `infrastructure/tempo.yaml`

### Traces not correlated with logs

Make sure:
1. Both tracing and OpenTelemetry layers are registered
2. Log messages use the same `tracing` macros (`info!`, `error!`, etc.)
3. Tempo datasource has `tracesToLogsV2` configured in Grafana

## Best Practices

1. **Start broad, drill down**: Begin with high-level spans, add detail as needed
2. **Profile in production**: Use 1-10% sampling to gather real-world data
3. **Correlate with metrics**: Compare trace latency with Prometheus metrics
4. **Use trace context**: Spans automatically propagate context to child spans
5. **Document new spans**: Update this file when adding significant instrumentation

## Further Reading

- [OpenTelemetry Rust SDK](https://github.com/open-telemetry/opentelemetry-rust)
- [Grafana Tempo Documentation](https://grafana.com/docs/tempo/latest/)
- [Tracing Crate](https://docs.rs/tracing/latest/tracing/)
- [Best Practices for Distributed Tracing](https://opentelemetry.io/docs/concepts/instrumentation/)
