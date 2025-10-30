# Pull-Data Metrics Documentation

This document describes the metrics collection and monitoring setup for the SOAR pull-data process.

## Overview

The pull-data command now exports detailed Prometheus metrics for each stage of the data loading process. These metrics are available during pull-data execution and can be scraped by Prometheus for historical analysis and visualization in Grafana.

## Metrics Exported

### Overall Metrics

- **`data_load.total_duration_seconds`** (histogram) - Total duration of the entire pull-data run
- **`data_load.overall_success`** (gauge) - 1.0 if successful, 0.0 if failed
- **`data_load.last_run_timestamp`** (gauge) - Unix timestamp of the last run

### Per-Stage Metrics

All stage metrics include a `stage` label identifying the specific stage:

- **`data_load.stage.duration_seconds{stage="..."}`** (histogram) - Duration of each stage
- **`data_load.stage.records_loaded{stage="..."}`** (counter) - Number of records loaded in each stage
- **`data_load.stage.success{stage="..."}`** (gauge) - 1.0 if stage succeeded, 0.0 if failed
- **`data_load.stage.records_in_db{stage="..."}`** (gauge) - Total records in database after stage

### Stages Tracked

1. `aircraft_models` - Load aircraft models from FAA data
2. `aircraft_registrations` - Load aircraft registrations from FAA data
3. `airports` - Load airport data
4. `runways` - Load runway data
5. `receivers` - Load OGN receiver data
6. `devices` - Load FlarmNet device data
7. `link_aircraft_to_devices` - Link aircraft to devices by registration
8. `link_devices_to_clubs` - Link devices to clubs
9. `geocoding` - Geocode soaring club locations (if enabled)
10. `link_home_bases` - Link home bases (if enabled)

## Metrics Server

During execution, pull-data starts a metrics server on **port 9092** that exposes:

- **`/metrics`** - Prometheus metrics endpoint
- **`/debug/pprof/profile`** - CPU profiling (30s flamegraph)
- **`/debug/pprof/heap`** - Heap profiling (10s pprof)

The metrics server runs for the duration of the pull-data execution.

## Prometheus Configuration

### Main Configuration

The main `prometheus.yml` file has been updated to support modular scrape configurations:

```yaml
scrape_config_files:
  - 'prometheus-scrape-*.yml'
```

This allows you to add new scrape targets by simply creating `prometheus-scrape-*.yml` files without modifying the main configuration.

### Pull-Data Scrape Configuration

A new file `infrastructure/prometheus-scrape-pull-data.yml` has been created:

```yaml
- job_name: 'soar-pull-data'
  metrics_path: '/metrics'
  scrape_interval: 5s
  scrape_timeout: 3s
  static_configs:
    - targets:
        - 'localhost:9092'
      labels:
        instance: 'soar-pull-data'
        component: 'pull-data'
```

**Note**: The pull-data metrics server is ephemeral and only runs during pull-data execution. Prometheus will only successfully scrape metrics while pull-data is running.

## Grafana Dashboard

A new Grafana dashboard has been created: `infrastructure/grafana-dashboard-pull-data.json`

### Dashboard Sections

1. **Pull-Data Overview**
   - Last run status (SUCCESS/FAILED)
   - Total duration of last run
   - Last run timestamp

2. **Stage Durations**
   - Time series showing duration of each stage over time
   - Pie chart showing duration breakdown
   - Table view with success/failure status

3. **Records Loaded**
   - Records loaded per stage
   - Total records in database

### Importing the Dashboard

1. Open Grafana
2. Go to Dashboards → Import
3. Upload `infrastructure/grafana-dashboard-pull-data.json`
4. Select your Prometheus datasource
5. Click Import

## Email Reporting

The email reports sent after each pull-data run already include:

- Overall success/failure status
- Total duration
- Per-stage metrics table with:
  - Stage name
  - Success/failure status
  - Duration
  - Records loaded
  - Total records in database

No changes were needed to the email reporting as it already tracked all necessary metrics.

## Usage

### Running Pull-Data

```bash
./target/release/soar pull-data
```

The metrics server will automatically start on port 9092 and metrics will be recorded throughout execution.

### Viewing Metrics Manually

While pull-data is running, you can view metrics directly:

```bash
curl http://localhost:9092/metrics
```

### Prometheus Integration

Ensure your Prometheus instance includes the scrape configuration:

```yaml
# In prometheus.yml
scrape_config_files:
  - 'prometheus-scrape-*.yml'
```

Then place `prometheus-scrape-pull-data.yml` in the same directory as `prometheus.yml` and reload Prometheus:

```bash
# Send SIGHUP to reload config
kill -HUP $(pgrep prometheus)
# OR restart prometheus service
sudo systemctl reload prometheus
```

## Monitoring Recommendations

1. **Set up alerts** for pull-data failures (`data_load_overall_success == 0`)
2. **Monitor duration trends** to detect performance degradation
3. **Track records loaded** to detect data source issues
4. **Review stage failures** to identify specific problem areas

## Implementation Details

### Code Changes

1. **`src/loader/mod.rs`**
   - Added `record_stage_metrics()` helper function
   - Added Prometheus metrics recording for each stage
   - Added overall metrics at completion

2. **`src/pull.rs`**
   - Already starts metrics server on port 9092 (no changes needed)

3. **`src/email_reporter.rs`**
   - No changes needed (already tracks all metrics)

### Metric Types

- **Histograms**: Used for durations to enable percentile calculations
- **Counters**: Used for cumulative counts (records loaded)
- **Gauges**: Used for current state (success/failure, timestamp, database totals)

## Future Enhancements

Potential improvements:

1. Add download stage metrics (FAA data, FlarmNet, etc.)
2. Add rate metrics (records/second)
3. Add error rate tracking per stage
4. Add alerting rules for common failure scenarios
5. Create composite metrics (e.g., efficiency score)
