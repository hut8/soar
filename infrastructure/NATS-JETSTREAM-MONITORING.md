# NATS JetStream Monitoring Setup

This guide explains how to set up monitoring for NATS JetStream queues and streams in Grafana.

## Overview

The NATS JetStream monitoring setup consists of three components:

1. **prometheus-nats-exporter** - Scrapes NATS HTTP endpoint and exposes metrics in Prometheus format
2. **Prometheus scrape job** - Configures Prometheus to collect metrics from the exporter
3. **Grafana dashboard** - Visualizes JetStream queue depth, consumer lag, throughput, and health

## Architecture

```
NATS Server (port 8222) → prometheus-nats-exporter (port 7777) → Prometheus → Grafana
                HTTP monitoring         Prometheus metrics
```

## Installation

### 1. Install prometheus-nats-exporter

Download and install the latest release:

```bash
# Download the binary (check https://github.com/nats-io/prometheus-nats-exporter/releases for latest version)
wget https://github.com/nats-io/prometheus-nats-exporter/releases/download/v0.15.0/prometheus-nats-exporter-v0.15.0-linux-amd64.tar.gz

# Extract
tar -xzf prometheus-nats-exporter-v0.15.0-linux-amd64.tar.gz

# Install to system location
sudo mv prometheus-nats-exporter /usr/local/bin/
sudo chmod +x /usr/local/bin/prometheus-nats-exporter

# Verify installation
prometheus-nats-exporter --version
```

### 2. Install systemd service

```bash
# Copy service file to systemd directory
sudo cp infrastructure/prometheus-nats-exporter.service /etc/systemd/system/

# Reload systemd to recognize new service
sudo systemctl daemon-reload

# Enable service to start on boot
sudo systemctl enable prometheus-nats-exporter

# Start the service
sudo systemctl start prometheus-nats-exporter

# Check status
sudo systemctl status prometheus-nats-exporter
```

### 3. Deploy Prometheus scrape job

The Prometheus configuration already includes `scrape_config_files` directive, so the job file will be loaded automatically:

```bash
# Copy job configuration (if not already in repository)
sudo cp infrastructure/prometheus-jobs/nats-jetstream.yml /etc/prometheus/jobs/

# Reload Prometheus configuration
sudo systemctl reload prometheus

# Verify the target is being scraped
# Open http://localhost:9090/targets and look for 'nats-jetstream' job
```

### 4. Import Grafana dashboard

**Option A: Via Grafana UI**
1. Open Grafana (http://localhost:3000)
2. Go to Dashboards → Import
3. Click "Upload JSON file"
4. Select `infrastructure/grafana-dashboard-nats-jetstream.json`
5. Click "Import"

**Option B: Via API**
```bash
curl -X POST http://admin:admin@localhost:3000/api/dashboards/db \
  -H "Content-Type: application/json" \
  -d @infrastructure/grafana-dashboard-nats-jetstream.json
```

## Verification

### Check prometheus-nats-exporter is running

```bash
# Check service status
sudo systemctl status prometheus-nats-exporter

# Verify metrics endpoint
curl http://localhost:7777/metrics | head -20

# You should see metrics like:
# jetstream_stream_total_messages{stream="APRS_RAW"} 12345
# jetstream_consumer_num_pending{stream="APRS_RAW",consumer="soar-run-production"} 42
```

### Check Prometheus is scraping

1. Open Prometheus UI: http://localhost:9090
2. Go to Status → Targets
3. Look for the `nats-jetstream` job - it should show state "UP"
4. Try a test query in the Graph tab:
   ```promql
   jetstream_stream_total_messages{stream="APRS_RAW"}
   ```

### Check Grafana dashboard

1. Open Grafana: http://localhost:3000
2. Go to Dashboards
3. Open "NATS JetStream - APRS Queue Monitoring"
4. You should see:
   - Queue depth gauge
   - Consumer lag gauge
   - Message rate graphs
   - Error panels

## Key Metrics Explained

### Queue Metrics
- **Queue Depth (Pending)**: Number of messages waiting to be delivered to the consumer
- **Consumer Lag**: How many messages behind the consumer is from the latest message in the stream
- **Ack Pending**: Messages delivered to consumer but not yet acknowledged

### Flow Metrics
- **Publish vs Consume Rate**: Compare ingestion rate to processing rate
  - If publish rate > consume rate: Queue will grow
  - If consume rate > publish rate: Queue will shrink

### Health Metrics
- **Redelivery Rate**: Messages that failed processing and are being retried
- **Consumer Errors**: Decode, process, ack, and receive errors
- **Publisher Errors**: Failures when publishing to JetStream

### Storage Metrics
- **Stream Storage**: Total bytes stored in APRS_RAW stream
- **Total Messages**: Count of messages in the stream

## Troubleshooting

### Exporter not starting

```bash
# Check logs
sudo journalctl -u prometheus-nats-exporter -f

# Common issues:
# - NATS server not running: sudo systemctl start nats-server
# - Port 7777 already in use: Check with sudo netstat -tulpn | grep 7777
```

### No metrics in Prometheus

```bash
# Test the exporter directly
curl http://localhost:7777/metrics

# Check Prometheus targets
# http://localhost:9090/targets
# Look for nats-jetstream job and check for errors

# Reload Prometheus config
sudo systemctl reload prometheus
```

### Dashboard shows "No Data"

1. **Check data source**: Dashboard uses `${DS_PROMETHEUS}` variable
   - Go to Dashboard Settings → Variables
   - Ensure Prometheus data source is selected

2. **Verify metrics exist**: Go to Prometheus and query:
   ```promql
   jetstream_consumer_num_pending{stream="APRS_RAW"}
   ```

3. **Check time range**: Dashboard defaults to last 6 hours
   - Adjust time range in top-right corner if needed

## Alerting

Consider setting up alerts for:

1. **High Queue Depth**
   ```promql
   jetstream_consumer_num_pending{stream="APRS_RAW"} > 10000
   ```

2. **High Consumer Lag**
   ```promql
   (jetstream_stream_last_seq{stream="APRS_RAW"} - ignoring(consumer)
    jetstream_consumer_delivered_stream_seq{stream="APRS_RAW",consumer="soar-run-production"}) > 5000
   ```

3. **Redelivery Rate**
   ```promql
   rate(jetstream_consumer_num_redelivered{stream="APRS_RAW"}[5m]) > 10
   ```

4. **Publish vs Consume Imbalance**
   ```promql
   rate(aprs_jetstream_published[5m]) >
   rate(aprs_jetstream_consumed[5m]) * 1.2  # 20% higher publish rate
   ```

## Files Created

- `infrastructure/prometheus-nats-exporter.service` - Systemd service definition
- `infrastructure/prometheus-jobs/nats-jetstream.yml` - Prometheus scrape configuration
- `infrastructure/grafana-dashboard-nats-jetstream.json` - Grafana dashboard definition
- `infrastructure/NATS-JETSTREAM-MONITORING.md` - This documentation file

## References

- [NATS Server Monitoring](https://docs.nats.io/running-a-nats-service/nats_admin/monitoring)
- [prometheus-nats-exporter GitHub](https://github.com/nats-io/prometheus-nats-exporter)
- [JetStream Concepts](https://docs.nats.io/nats-concepts/jetstream)
