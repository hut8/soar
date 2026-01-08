# Unified Ingest Service Migration Guide

## Overview

The SOAR ingest services have been consolidated into a single unified `soar ingest` command that can handle multiple ingestion sources simultaneously:
- **OGN (APRS)**: Glider and light aircraft tracking via the OGN network
- **Beast**: ADS-B data in Beast binary format
- **SBS**: ADS-B data in SBS (BaseStation/port 30003) text format

This consolidation simplifies deployment and provides better monitoring with aggregate metrics across all sources.

## Migration Path

The separate `soar ingest-ogn` and `soar ingest-adsb` services have been replaced with a unified `soar ingest` service:

- **Old**: `soar ingest-ogn` (port 9093 production, 9194 staging) and `soar ingest-adsb` (port 9094 production, 9196 staging)
- **New**: `soar ingest` - Unified ingestion service (port 9095 production, 9197 staging)

## Command Examples

### OGN Only
```bash
soar ingest --ogn-server aprs.glidernet.org
```

### ADS-B Beast Only
```bash
soar ingest --beast radar:41365
```

### Both OGN and ADS-B
```bash
soar ingest --ogn-server aprs.glidernet.org --beast radar:41365
```

### Multiple Beast Sources
```bash
soar ingest \
  --ogn-server aprs.glidernet.org \
  --beast radar1:30005 \
  --beast radar2:30005 \
  --sbs data.adsbhub.org:5002
```

### With Filters
```bash
soar ingest \
  --ogn-server aprs.glidernet.org \
  --ogn-filter "r/38/-122/500" \
  --ogn-callsign "N0CALL"
```

## Systemd Service Files

### Production
- `/etc/systemd/system/soar-ingest.service`

### Staging
- `/etc/systemd/system/soar-ingest-staging.service`

Note: The old separate service files (`soar-ingest-ogn` and `soar-ingest-adsb`) have been removed.

## Deployment

The unified ingest service must be deployed manually using the regular deployment process. The service is not automatically restarted during regular deploys, but deploys will ensure it is running.

Refer to the main deployment documentation for details on deploying services.

## Metrics

The unified service exports metrics on:
- **Production**: `http://localhost:9095/metrics`
- **Staging**: `http://localhost:9197/metrics`

### Aggregate Metrics
These metrics combine data from all active sources (OGN, Beast, SBS):

- `ingest_messages_per_second` - Total message rate across all sources
- `ingest_messages_received_total` - Total messages received
- `ingest_messages_sent_total` - Total messages sent to soar-run

### Per-Source Metrics
These metrics are labeled with `source="ogn"`, `source="beast"`, or `source="sbs"`:

- `ingest_messages_per_second{source="ogn"}` - Message rate per source
- `ingest_messages_received_total{source="ogn"}` - Messages received per source
- `ingest_socket_send_error_total{source="ogn"}` - Send errors per source
- `ingest_socket_send_duration_ms{source="ogn"}` - Socket write latency histogram
- `ingest_queue_depth{source="ogn",type="memory"}` - Queue depth (memory)
- `ingest_queue_depth{source="ogn",type="disk"}` - Queue depth (disk)

### Health Metrics
- `ingest_health_ogn_connected` - OGN connection status (0 or 1)
- `ingest_health_beast_connected` - Beast connection status (0 or 1)
- `ingest_health_sbs_connected` - SBS connection status (0 or 1)
- `ingest_health_socket_connected` - Socket to soar-run status (0 or 1)

## Prometheus Configuration

### Configuration Files
- `infrastructure/prometheus-jobs/soar-ingest.yml` - Production
- `infrastructure/prometheus-jobs/soar-ingest-staging.yml` - Staging
- `infrastructure/prometheus-scrape-configs/soar-ingest.yml` - Production scrape config
- `infrastructure/prometheus-scrape-configs/soar-ingest-staging.yml` - Staging scrape config

Note: The old Prometheus configuration files for the separate OGN and ADS-B services have been removed as part of this consolidation.

## Benefits

1. **Simplified Deployment**: One service instead of two
2. **Better Monitoring**: Aggregate metrics show total ingestion rate
3. **Easier Configuration**: All sources configured in one place
4. **Resource Efficiency**: Shared metrics server and health monitoring
5. **Flexible Source Selection**: Enable/disable sources as needed

## Troubleshooting

### Check Service Status
```bash
# Production
sudo systemctl status soar-ingest.service

# Staging
sudo systemctl status soar-ingest-staging.service
```

### View Logs
```bash
# Production
sudo journalctl -u soar-ingest.service -f

# Staging
sudo journalctl -u soar-ingest-staging.service -f
```

### Test Command Locally
```bash
# Test with OGN only
./soar ingest --ogn-server aprs.glidernet.org

# Test with ADS-B only
./soar ingest --beast localhost:30005

# Test with both
./soar ingest --ogn-server aprs.glidernet.org --beast localhost:30005
```

## Questions?

See the issue tracker or contact the SOAR development team for assistance with migration.
