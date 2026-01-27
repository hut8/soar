# SOAR Grafana Dashboards

This directory contains Grafana dashboard configurations for monitoring SOAR metrics.

## Dashboard Sources

Dashboards are built from modular source files. The built `grafana-dashboard-*.json` files are **not committed to the repository** - they are generated during deployment.

### Source Structure
- `dashboards/definitions/` - Dashboard definitions (panel order, rows, metadata)
- `dashboards/panels/{dashboard}/` - Individual panel JSON files
- `dashboards/common/` - Shared configs (annotations, templating variables)
- `dashboards/build.py` - Build script

### Available Dashboards

| Dashboard | Definition | Description |
|-----------|------------|-------------|
| SOAR Ingest | `ingest.json` | OGN/APRS and ADS-B data ingestion |
| SOAR Run | `run.json` | Main processing service metrics |
| SOAR Run - Geocoding | `run-geocoding.json` | Pelias geocoding service |
| SOAR Run - Elevation | `run-elevation.json` | Elevation processing and AGL |
| SOAR Web | `web.json` | Web server and API metrics |
| SOAR Analytics | `analytics.json` | Analytics API and cache |
| SOAR NATS | `nats.json` | NATS messaging metrics |
| SOAR Coverage | `coverage.json` | Coverage API metrics |

## Building Dashboards Manually

To build dashboards locally (e.g., for testing changes before deployment):

```bash
# Build all dashboards
python3 infrastructure/dashboards/build.py

# Build a specific dashboard
python3 infrastructure/dashboards/build.py ingest

# Verify all dashboards build correctly (without writing files)
python3 infrastructure/dashboards/build.py --verify
```

This generates `grafana-dashboard-*.json` files in the `infrastructure/` directory.

## Automated Deployment (Recommended)

Dashboards are **automatically deployed** as part of the standard SOAR deployment process. When you run `soar-deploy`, the script will:

1. Copy all dashboard JSON files to `/etc/grafana/dashboards/`
2. Install Grafana provisioning configuration to `/etc/grafana/provisioning/dashboards/`
3. Restart Grafana to load the updated dashboards

**No manual action required!** Dashboards will be available in Grafana under the "SOAR" folder after deployment.

### Deployment Details

The deployment process uses Grafana's built-in provisioning system:
- Dashboard files are loaded from `/etc/grafana/dashboards/`
- Provisioning configuration is in `/etc/grafana/provisioning/dashboards/dashboards.yml`
- Dashboards update automatically when files change (10-second check interval)
- You can still edit dashboards in the UI (changes are allowed)

## Metrics Endpoint

The metrics server runs on port 9091 when SOAR is started in production mode (`SOAR_ENV=production`).

Metrics are available at: `http://localhost:9091/metrics`

## Available Metrics

### Receiver Status Updates

- **receiver_status_updates_total** - Counter tracking total number of receiver status updates received and stored

## Dashboard Panels

The SOAR dashboard (`soar-dashboard.json`) includes:

1. **Receiver Status Updates (Total)** - Shows the total count of receiver status updates
2. **Receiver Status Updates per Hour** - Graph showing the rate of updates per hour (calculated using 5-minute rate)
3. **Receiver Status Updates Rate (1h)** - Current rate over 1-hour window
4. **Receiver Status Updates Timeline** - Timeline showing increase in updates over 5-minute windows

## Quick Manual Installation (On Server)

If you're working directly on the server and need to update dashboards without doing a full deployment:

```bash
cd /path/to/soar
sudo ./infrastructure/install-grafana-dashboards
```

This script will:
- Install all dashboard JSON files to `/etc/grafana/dashboards/`
- Update the provisioning configuration
- Restart Grafana to load the dashboards

**Requirements**: Must be run from the SOAR repository root with sudo.

## Manual Import (For Reference/Troubleshooting)

If you need to manually import dashboards (e.g., for testing or troubleshooting), you can use these methods:

### Using Grafana UI

1. Open Grafana in your browser
2. Go to Dashboards → Import
3. Click "Upload JSON file"
4. Select the desired `grafana-dashboard-*.json` file
5. Select your Prometheus data source
6. Click "Import"

### Using Grafana API

```bash
# Replace with your Grafana URL and API key
GRAFANA_URL="http://localhost:3000"
GRAFANA_API_KEY="your-api-key"
DASHBOARD_FILE="grafana-dashboard-web.json"

curl -X POST \
  -H "Authorization: Bearer $GRAFANA_API_KEY" \
  -H "Content-Type: application/json" \
  -d @"$DASHBOARD_FILE" \
  "$GRAFANA_URL/api/dashboards/db"
```

### Manual Provisioning Setup

If you're setting up provisioning manually (not using soar-deploy):

1. Copy dashboard files:
   ```bash
   sudo mkdir -p /etc/grafana/dashboards
   sudo cp grafana-dashboard-*.json /etc/grafana/dashboards/
   sudo chown -R grafana:grafana /etc/grafana/dashboards
   ```

2. Copy provisioning config:
   ```bash
   sudo mkdir -p /etc/grafana/provisioning/dashboards
   sudo cp grafana-provisioning/dashboards/dashboards.yml /etc/grafana/provisioning/dashboards/
   sudo chown -R grafana:grafana /etc/grafana/provisioning/dashboards
   ```

3. Restart Grafana:
   ```bash
   sudo systemctl restart grafana-server
   ```

## Prometheus Configuration

SOAR metrics are automatically configured via the Prometheus job files in `/etc/prometheus/jobs/` (deployed by `soar-deploy`):

- **soar-web.yml** - Web server metrics (localhost:61225/data/metrics)
- **soar-run.yml** - APRS processing service (localhost:9091/metrics)
- **soar-pull-data.yml** - Data loading job (localhost:9092/metrics)
- **soar-ingest-ogn.yml** - OGN/APRS ingest service (localhost:9093/metrics)

Your Prometheus configuration should include:

```yaml
scrape_config_files:
  - /etc/prometheus/jobs/*.yml
```

This allows job configurations to be updated without modifying the main Prometheus config file.

## Queries Used

The dashboard uses these Prometheus queries:

- **Total updates**: `receiver_status_updates_total`
- **Updates per hour (5m rate)**: `rate(receiver_status_updates_total[5m]) * 3600`
- **Updates per hour (1h rate)**: `rate(receiver_status_updates_total[1h]) * 3600`
- **Updates in 5m window**: `increase(receiver_status_updates_total[5m])`

## Adding More Metrics

To add more metrics to the dashboard:

1. Add the metric in the Rust code using the `metrics` crate
2. Export it via the Prometheus exporter (already configured in `src/metrics.rs`)
3. Add a new panel to `soar-dashboard.json` with the appropriate Prometheus query
4. Re-import the dashboard

## See Also

- [Grafana Alerting Configuration](GRAFANA-ALERTING.md)
- [Grafana Datasource Troubleshooting](GRAFANA-DATASOURCE-TROUBLESHOOTING.md)
- [SOAR Deployment Guide](DEPLOYMENT.md)
