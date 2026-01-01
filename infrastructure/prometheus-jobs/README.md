# Prometheus Job Configuration Files

This directory contains complete Prometheus scrape job configurations that are loaded via `scrape_config_files` in the main `prometheus.yml`.

## How It Works

The main `prometheus.yml` file contains only one line for scrape configuration:

```yaml
scrape_config_files:
  - '/etc/prometheus/jobs/*.yml'
```

This tells Prometheus to load all `.yml` files from `/etc/prometheus/jobs/` as complete scrape job configurations.

## Deployment

Copy these files to your Prometheus server:

```bash
sudo mkdir -p /etc/prometheus/jobs
sudo cp infrastructure/prometheus-jobs/*.yml /etc/prometheus/jobs/
sudo chown -R prometheus:prometheus /etc/prometheus/jobs
sudo chmod 644 /etc/prometheus/jobs/*.yml
```

## Available Jobs

- **soar-web.yml** - SOAR web server metrics (port 61225, `/data/metrics`)
- **soar-ingest-ogn.yml** - APRS ingest service (port 9093, `/metrics`)
- **soar-run.yml** - APRS processing service (port 9091, `/metrics`)
- **soar-pull-data.yml** - Data loading job (port 9092, `/metrics`)
- **soar-ingest-adsb.yml** - ADS-B Beast ingest service - production (port 9094, `/metrics`)
- **soar-ingest-adsb-staging.yml** - ADS-B Beast ingest service - staging (port 9096, `/metrics`)

## Adding New Jobs

To add a new scrape job:

1. Create a new `.yml` file in this directory
2. Define the complete job configuration (see examples below)
3. Copy the file to `/etc/prometheus/jobs/` on your Prometheus server
4. Prometheus will automatically load it (may take up to 1 minute)

### Example Job Configuration

**IMPORTANT:** Each file must have `scrape_configs:` as the root key (Prometheus 2.47+)

```yaml
scrape_configs:
  - job_name: 'my-service'
    metrics_path: '/metrics'
    scrape_interval: 15s
    scrape_timeout: 10s
    static_configs:
      - targets:
          - 'localhost:9090'
        labels:
          instance: 'my-service-prod'
          component: 'backend'
          environment: 'production'
```

## Advantages

- **Simpler** - Only one line needed in main prometheus.yml
- **Complete** - Each job file contains all configuration (paths, intervals, labels, relabeling)
- **Easier to manage** - Each service has its own independent configuration file
- **Better for GitOps** - Easy to version control and deploy individual jobs

## Reloading Configuration

Prometheus automatically reloads configuration files, but you can force a reload:

```bash
# Send SIGHUP to Prometheus process
sudo systemctl reload prometheus

# Or use the HTTP API
curl -X POST http://localhost:9090/-/reload
```

## Validation

Test your job configuration before deploying:

```bash
promtool check config /etc/prometheus/prometheus.yml
```
