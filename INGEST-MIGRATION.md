# Unified Ingest Service Migration Guide

## Overview

The SOAR ingest services have been consolidated into a single unified `soar ingest` command that can handle multiple ingestion sources simultaneously:
- **OGN (APRS)**: Glider and light aircraft tracking via the OGN network
- **Beast**: ADS-B data in Beast binary format
- **SBS**: ADS-B data in SBS (BaseStation/port 30003) text format

This consolidation simplifies deployment and provides better monitoring with aggregate metrics across all sources.

## Migration Path

### Current State (Deprecated)
- `soar ingest-ogn` - Separate OGN ingestion service (port 9093 production, 9194 staging)
- `soar ingest-adsb` - Separate ADS-B ingestion service (port 9094 production, 9196 staging)

### New State (Recommended)
- `soar ingest` - Unified ingestion service (port 9095 production, 9197 staging)

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
- **New**: `/etc/systemd/system/soar-ingest.service`
- **Old** (deprecated): 
  - `/etc/systemd/system/soar-ingest-ogn.service`
  - `/etc/systemd/system/soar-ingest-adsb.service`

### Staging
- **New**: `/etc/systemd/system/soar-ingest-staging.service`
- **Old** (deprecated):
  - `/etc/systemd/system/soar-ingest-ogn-staging.service`
  - `/etc/systemd/system/soar-ingest-adsb-staging.service`

## Deployment

The unified ingest service is deployed via the manual GitHub Actions workflow:

```bash
# Deploy to production
gh workflow run deploy-ingest.yml \
  -f environment=production \
  -f services=ingest

# Deploy to staging
gh workflow run deploy-ingest.yml \
  -f environment=staging \
  -f services=ingest
```

The old services can still be deployed for backward compatibility:
```bash
gh workflow run deploy-ingest.yml \
  -f environment=production \
  -f services=ingest-ogn,ingest-adsb
```

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

### New Configuration Files
- `infrastructure/prometheus-jobs/soar-ingest.yml` - Production
- `infrastructure/prometheus-jobs/soar-ingest-staging.yml` - Staging
- `infrastructure/prometheus-scrape-configs/soar-ingest.yml` - Production scrape config
- `infrastructure/prometheus-scrape-configs/soar-ingest-staging.yml` - Staging scrape config

### Old Configuration Files (Deprecated)
- `infrastructure/prometheus-jobs/soar-ingest-ogn.yml` 
- `infrastructure/prometheus-jobs/soar-ingest-adsb.yml`
- And corresponding staging files

## Benefits

1. **Simplified Deployment**: One service instead of two
2. **Better Monitoring**: Aggregate metrics show total ingestion rate
3. **Easier Configuration**: All sources configured in one place
4. **Resource Efficiency**: Shared metrics server and health monitoring
5. **Flexible Source Selection**: Enable/disable sources as needed

## Backward Compatibility

The old `ingest-ogn` and `ingest-adsb` commands are still available for backward compatibility during the transition period. They will be removed in a future release once all deployments have migrated to the unified service.

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

## Migration Timeline

1. **Phase 1** (Current): New unified service deployed alongside old services
2. **Phase 2**: Production deployments switch to unified service
3. **Phase 3**: Old services disabled but kept for rollback capability
4. **Phase 4**: Old services removed from repository

## Questions?

See the issue tracker or contact the SOAR development team for assistance with migration.
