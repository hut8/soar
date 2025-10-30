# SOAR Grafana Dashboard

This directory contains Grafana dashboard configurations for monitoring SOAR metrics.

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

## Importing the Dashboard

### Using Grafana UI

1. Open Grafana in your browser
2. Go to Dashboards → Import
3. Click "Upload JSON file"
4. Select `soar-dashboard.json`
5. Select your Prometheus data source
6. Click "Import"

### Using Grafana API

```bash
# Replace with your Grafana URL and API key
GRAFANA_URL="http://localhost:3000"
GRAFANA_API_KEY="your-api-key"

curl -X POST \
  -H "Authorization: Bearer $GRAFANA_API_KEY" \
  -H "Content-Type: application/json" \
  -d @soar-dashboard.json \
  "$GRAFANA_URL/api/dashboards/db"
```

### Using Grafana Provisioning

1. Copy `soar-dashboard.json` to your Grafana provisioning directory:
   ```bash
   sudo cp soar-dashboard.json /etc/grafana/provisioning/dashboards/
   ```

2. Create a provisioning config file at `/etc/grafana/provisioning/dashboards/soar.yaml`:
   ```yaml
   apiVersion: 1

   providers:
     - name: 'SOAR'
       orgId: 1
       folder: ''
       type: file
       disableDeletion: false
       updateIntervalSeconds: 10
       allowUiUpdates: true
       options:
         path: /etc/grafana/provisioning/dashboards
   ```

3. Restart Grafana:
   ```bash
   sudo systemctl restart grafana-server
   ```

## Prometheus Configuration

Make sure your Prometheus is configured to scrape the SOAR metrics endpoint:

```yaml
scrape_configs:
  - job_name: 'soar'
    static_configs:
      - targets: ['localhost:9091']
    scrape_interval: 15s
```

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
