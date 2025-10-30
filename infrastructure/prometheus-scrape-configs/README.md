# Prometheus Scrape Configurations for SOAR

This directory contains file-based service discovery configurations for Prometheus scraping SOAR services.

## Overview

Using file-based service discovery allows you to:
- Manage SOAR service targets separately from other Prometheus targets
- Update target configurations without restarting Prometheus
- Easily merge SOAR configs with your existing Prometheus setup

## Files

- **soar-web.yml** - SOAR web server (port 61225, `/data/metrics` endpoint)
- **soar-aprs-ingest.yml** - SOAR APRS ingest service (port 9093, `/metrics` endpoint)
- **soar-run.yml** - SOAR APRS processing service (port 9091, `/metrics` endpoint)

## Installation

### Option 1: Copy to System Prometheus Directory

```bash
# Create the scrape configs directory
sudo mkdir -p /etc/prometheus/scrape-configs

# Copy SOAR scrape configs
sudo cp infrastructure/prometheus-scrape-configs/*.yml /etc/prometheus/scrape-configs/

# Set permissions
sudo chown -R prometheus:prometheus /etc/prometheus/scrape-configs
sudo chmod 644 /etc/prometheus/scrape-configs/*.yml
```

### Option 2: Merge with Existing Configuration

If you already have a Prometheus configuration, you can either:

1. **Use file-based discovery** (recommended):
   - Copy the `scrape_configs` section from `../prometheus.yml`
   - Add it to your existing Prometheus configuration
   - Copy the scrape config files to `/etc/prometheus/scrape-configs/`

2. **Use static configuration**:
   - Copy the target definitions from each `.yml` file
   - Add them as `static_configs` in your Prometheus configuration

## Updating Targets

Prometheus automatically reloads these files every 30 seconds. To update targets:

```bash
# Edit the appropriate file
sudo nano /etc/prometheus/scrape-configs/soar-web.yml

# Prometheus will detect the change within 30 seconds
# No restart required!
```

## Metrics Endpoints

| Service | Port | Path | Description |
|---------|------|------|-------------|
| soar-web | 61225 | `/data/metrics` | Web server metrics (HTTP requests, DB queries) |
| soar-aprs-ingest | 9093 | `/metrics` | APRS ingestion metrics (connection, queue depth) |
| soar-run | 9091 | `/metrics` | APRS processing metrics (queues, workers) |

## Labels

Each service includes these labels:
- `instance` - Service instance identifier
- `component` - Service component name (web, aprs-ingest, run)
- `environment` - Deployment environment (production)
- `source` - Source identifier for metric relabeling

## Testing

Verify Prometheus can scrape the targets:

```bash
# Check if targets are loaded
curl http://localhost:9090/api/v1/targets | jq '.data.activeTargets[] | select(.labels.job | contains("soar"))'

# Test individual endpoints
curl http://localhost:9093/metrics  # soar-aprs-ingest
curl http://localhost:9091/metrics  # soar-run
curl http://localhost:61225/data/metrics  # soar-web
```

## Troubleshooting

### Targets not appearing
- Check file permissions: `ls -la /etc/prometheus/scrape-configs/`
- Check Prometheus logs: `journalctl -u prometheus -f`
- Verify file paths in main Prometheus config match actual file locations

### Metrics not updating
- Verify services are running: `systemctl status soar-aprs-ingest soar-run soar-web`
- Check service is listening on correct port: `ss -tlnp | grep -E '9091|9093|61225'`
- Test metrics endpoints manually with curl (see above)

### File discovery not working
- Ensure Prometheus has read access to scrape config files
- Check that `file_sd_configs` paths in main config are correct
- Verify YAML syntax: `promtool check config /etc/prometheus/prometheus.yml`
