# Pyroscope Continuous Profiling Integration

This document describes the Pyroscope continuous profiling integration for the SOAR project.

## Overview

SOAR uses Grafana Pyroscope for continuous profiling of Rust applications, with Grafana Alloy collecting profiles from pprof endpoints. Alloy also integrates with Loki (logs) and Tempo (traces) for comprehensive observability.

## Architecture

```
┌─────────────┐   /debug/pprof/profile    ┌─────────────┐   HTTP Push      ┌─────────────┐
│ SOAR        │────────────────────────────>│ Alloy       │─────────────────>│ Pyroscope   │
│ Services    │   (HTTP scrape)            │ (Collector) │   (profiles)     │ (Storage)   │
│             │                            │             │                  └─────────────┘
│             │   Logs (journald)          │             │   Logs           ┌─────────────┐
│             │───────────────────────────>│             │─────────────────>│ Loki        │
│             │                            │             │                  │ (Logs)      │
│             │                            │             │                  └─────────────┘
│             │   OTLP Traces              │             │   Traces         ┌─────────────┐
│             │───────────────────────────>│             │─────────────────>│ Tempo       │
│             │   (port 4317/4318)         │             │   (OTLP)         │ (Traces)    │
└─────────────┘                            └─────────────┘                  └─────────────┘
     │                                             │                                │
     │         Prometheus metrics                  │                                │
     └──────────────────────>┌─────────────────────▼────────────────────────────────▼─────┐
                   port 9090 │ Prometheus → Grafana (Unified Visualization)               │
                             └─────────────────────────────────────────────────────────────┘
```

### Components

- **SOAR Services**: Rust applications exposing pprof endpoints at `/debug/pprof/profile` (CPU) and `/debug/pprof/heap` (memory)
- **Grafana Alloy**: Unified collector for logs (Loki), traces (Tempo), and profiles (Pyroscope)
- **Pyroscope**: Profile storage backend with filesystem storage
- **Loki**: Log aggregation (via Alloy journald collection)
- **Tempo**: Distributed tracing backend (via OTLP)
- **Grafana**: Unified visualization with datasources for all three pillars

## Current Status

**✅ Infrastructure deployed and configured**

- ✅ Pyroscope installed via Grafana APT repository (systemd service provided by package)
- ✅ Alloy installed and configured (systemd service provided by package)
- ✅ Alloy integrates with Loki, Tempo, and Pyroscope
- ✅ Prometheus scrape configs for monitoring Pyroscope/Alloy
- ✅ Grafana datasource auto-provisioning
- ✅ 30-day profile retention configured

**What's working:**
- Existing pprof endpoints in SOAR (`/debug/pprof/profile`, `/debug/pprof/heap`)
- Pyroscope server with filesystem storage
- Alloy configuration to scrape production and staging services
- Alloy log collection from systemd journal
- Alloy OTLP trace receiver for Tempo integration
- Grafana datasource for profile visualization

## Configuration

### Pyroscope Configuration

Location: `/etc/pyroscope/config.yml`

```yaml
server:
  http_listen_address: "127.0.0.1"
  http_listen_port: 4040

storage:
  backend: "filesystem"
  filesystem:
    path: "/var/lib/pyroscope"

retention:
  profiles: 720h  # 30 days
```

### Alloy Configuration

Location: `/etc/alloy/config.alloy`

Alloy provides unified observability collection for:

**1. Profiling (Pyroscope)**
- Scrapes pprof endpoints from SOAR services
- CPU profiling via `/debug/pprof/profile?seconds=5` (5-second samples)
- Forwards to Pyroscope at `http://localhost:4040`

**2. Logs (Loki)**
- Collects logs from systemd journal for SOAR services
- Processes and labels log entries
- Forwards to Loki at `http://localhost:3100/loki/api/v1/push`

**3. Traces (Tempo)**
- Receives OTLP traces on ports 4317 (gRPC) and 4318 (HTTP)
- Batches and processes traces
- Forwards to Tempo at `localhost:4317`

**Monitored Services:**

**Production:**
- `soar-run` (localhost:9090)
- `soar-web` (localhost:9091)
- `soar-ingest-ogn` (localhost:9092)
- `soar-ingest-adsb` (localhost:9093)

**Staging:**
- `soar-run-staging` (localhost:9094)
- `soar-web-staging` (localhost:9095)

### Service Ports

| Service | Port | Endpoint |
|---------|------|----------|
| Pyroscope Web UI | 4040 | http://localhost:4040 |
| Pyroscope Metrics | 4040 | http://localhost:4040/metrics |
| Alloy HTTP API | 12345 | http://localhost:12345 |
| Alloy Metrics | 12345 | http://localhost:12345/metrics |

## Installation

### Provision Script

The `scripts/provision` script automatically:
1. Adds Grafana APT repository (if not already added)
2. Installs `pyroscope` and `alloy` packages (includes systemd services)
3. Creates system users (`pyroscope`, `alloy`) if not created by packages
4. Creates directories (`/var/lib/pyroscope`, `/var/lib/alloy`, `/etc/pyroscope`, `/etc/alloy`)
5. Enables and starts services

**Note:** Systemd service files are provided by the Debian packages. Custom service files are NOT needed.

### Manual Installation (if needed)

```bash
# Add Grafana repository (if not already added)
sudo apt-get install -y gnupg
wget -q -O - https://apt.grafana.com/gpg.key | sudo gpg --dearmor -o /etc/apt/keyrings/grafana.gpg
echo "deb [signed-by=/etc/apt/keyrings/grafana.gpg] https://apt.grafana.com stable main" | sudo tee /etc/apt/sources.list.d/grafana.list

# Install Pyroscope and Alloy (includes systemd services)
sudo apt-get update
sudo apt-get install -y pyroscope alloy

# The packages install:
# - Systemd service files (/lib/systemd/system/{pyroscope,alloy}.service)
# - System users (pyroscope, alloy)
# - Default directories

# Create custom configuration directories if needed
sudo mkdir -p /etc/pyroscope /etc/alloy
sudo chown -R pyroscope:pyroscope /etc/pyroscope
sudo chown -R alloy:alloy /etc/alloy

# Configuration files are deployed via soar-deploy
# These are automatically managed by deployment
```

## Deployment

The `soar-deploy` script automatically:
1. Copies `pyroscope-config.yml` to `/etc/pyroscope/config.yml`
2. Copies `alloy-config.alloy` to `/etc/alloy/config.alloy`
3. Sets correct ownership and permissions
4. Restarts services if already running

Configuration files are included in the deployment package by `scripts/deploy`.

## Usage

### Accessing Pyroscope

**Local access (on server):**
```bash
# Web UI
curl http://localhost:4040

# Query profiles via API
curl http://localhost:4040/api/profiles
```

**Via Grafana:**
1. Open Grafana (http://localhost:3000 or https://grafana.glider.flights)
2. Navigate to **Explore** → Select **Pyroscope** datasource
3. Choose a service name (e.g., `soar-run`)
4. Select profile type (CPU, heap)
5. Set time range
6. View flamegraph and analyze

### Service Management

```bash
# Check service status
sudo systemctl status pyroscope
sudo systemctl status alloy

# View logs
sudo journalctl -u pyroscope -f
sudo journalctl -u alloy -f

# Restart services
sudo systemctl restart pyroscope
sudo systemctl restart alloy

# Reload Alloy configuration without restart
sudo systemctl reload alloy
```

### Monitoring

Both Pyroscope and Alloy expose Prometheus metrics:

```bash
# Pyroscope metrics
curl http://localhost:4040/metrics

# Alloy metrics
curl http://localhost:12345/metrics
```

These metrics are automatically scraped by Prometheus via job configs:
- `infrastructure/prometheus-jobs/pyroscope.yml`
- `infrastructure/prometheus-jobs/alloy.yml`

## Existing pprof Endpoints

SOAR already exposes pprof endpoints for manual profiling:

### CPU Profiling
**Endpoint:** `GET /debug/pprof/profile`
- Profiles CPU for 30 seconds
- Returns flamegraph SVG

**Example:**
```bash
curl http://localhost:9090/debug/pprof/profile > profile.svg
```

### Heap Profiling
**Endpoint:** `GET /debug/pprof/heap`
- Samples heap allocations for 10 seconds
- Returns pprof protobuf format

**Example:**
```bash
curl http://localhost:9090/debug/pprof/heap > heap.pb
```

**These endpoints are now continuously scraped by Alloy for long-term profiling.**

## Profile Retention and Storage

### Storage Configuration

- **Backend:** Filesystem (local storage)
- **Location:** `/var/lib/pyroscope`
- **Retention:** 30 days (720 hours)
- **Format:** Pyroscope native format

### Estimated Storage Requirements

Profile storage depends on sampling frequency and number of services:

| Services | Sampling | Storage/Day | 30 Days |
|----------|----------|-------------|---------|
| 4 production | 30s samples | ~500MB | ~15GB |
| 6 (prod + staging) | 30s samples | ~750MB | ~22GB |

**Note:** Actual storage may vary based on application complexity and profiling duration.

### Cleanup and Maintenance

Pyroscope automatically handles:
- Profile compaction
- Retention enforcement (30-day cleanup)
- Storage optimization

Manual cleanup (if needed):
```bash
# Check storage usage
du -sh /var/lib/pyroscope

# Stop Pyroscope, clear all profiles, restart
sudo systemctl stop pyroscope
sudo rm -rf /var/lib/pyroscope/*
sudo systemctl start pyroscope
```

## Troubleshooting

### Pyroscope Not Receiving Profiles

**Check 1: Alloy is scraping successfully**
```bash
# Check Alloy logs for scrape errors
sudo journalctl -u alloy -f | grep -i error

# Check Alloy metrics for scrape failures
curl http://localhost:12345/metrics | grep scrape
```

**Check 2: SOAR services are exposing pprof endpoints**
```bash
# Test endpoint directly (soar-run on port 9090)
curl -I http://localhost:9090/debug/pprof/profile
# Expected: HTTP 200 OK (takes 30 seconds)
```

**Check 3: Pyroscope is accepting profiles**
```bash
# Check Pyroscope logs
sudo journalctl -u pyroscope -f

# Check Pyroscope ingestion metrics
curl http://localhost:4040/metrics | grep ingester
```

### Profiles Not Appearing in Grafana

**Check 1: Pyroscope datasource is configured**
```bash
# Check datasource in Grafana
curl -s -u admin:admin http://localhost:3000/api/datasources | jq '.[] | select(.type=="grafana-pyroscope-datasource")'
```

**Check 2: Query correct time range**
- Profiles are collected continuously but may take a few minutes to appear
- Ensure time range in Grafana includes recent data

**Check 3: Service labels are correct**
- Profiles are tagged with `service`, `environment`, `job` labels
- Use these labels to filter in Grafana

### High Storage Usage

**Solution 1: Reduce retention**
Edit `/etc/pyroscope/config.yml`:
```yaml
retention:
  profiles: 168h  # 7 days instead of 30
```

**Solution 2: Reduce scraping frequency**
Edit `/etc/alloy/config.alloy` to scrape less frequently (not recommended for continuous profiling).

**Solution 3: Profile fewer services**
Remove staging services from Alloy configuration if not needed.

## Security Considerations

### Network Exposure

- **Pyroscope:** Bound to localhost (127.0.0.1:4040) - not exposed publicly
- **Alloy:** Bound to localhost (127.0.0.1:12345) - not exposed publicly
- **pprof endpoints:** Exposed on SOAR metrics ports (localhost only)

To expose Pyroscope via Caddy (optional):
1. Create `/etc/soar/caddy/pyroscope.glider.flights.conf`
2. Configure reverse proxy to `http://localhost:4040`
3. Reload Caddy: `sudo systemctl reload caddy`

### Profiling Overhead

CPU profiling has minimal overhead:
- **Sampling rate:** 99 Hz (99 samples/second)
- **CPU overhead:** < 1% typically
- **Network overhead:** ~50-100 KB/s per service

Heap profiling is more expensive and should be used sparingly (on-demand only).

## References

- **Grafana Pyroscope Documentation:** https://grafana.com/docs/pyroscope/latest/
- **Grafana Alloy Documentation:** https://grafana.com/docs/alloy/latest/
- **Pyroscope Rust SDK:** https://github.com/grafana/pyroscope-rs
- **pprof-rs Documentation:** https://github.com/tikv/pprof-rs

## Next Steps

1. **Monitor profile collection:** Check that profiles are being collected regularly
2. **Create Grafana dashboards:** Build dashboards for key performance indicators
3. **Set up alerts:** Alert on high CPU usage or anomalies
4. **Analyze performance:** Use flamegraphs to identify bottlenecks
5. **Optional: Add SDK integration:** For more detailed profiling, consider integrating pyroscope-rs SDK directly

## Migration Notes

This integration complements the existing pprof endpoints in SOAR:
- ✅ Existing endpoints remain functional for manual profiling
- ✅ Alloy continuously scrapes these endpoints for long-term analysis
- ✅ No code changes required in SOAR applications
- ✅ Profiles are now stored and queryable over time
