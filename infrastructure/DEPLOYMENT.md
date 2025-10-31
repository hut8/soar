# SOAR Deployment Guide

## Overview

The SOAR deployment process has been simplified to use a single deployment script that handles all server-side operations. This approach reduces complexity, improves security by minimizing sudo privileges, and makes deployments more reliable.

## Architecture

### Deployment Flow

1. **GitHub Actions CI/CD** (`.github/workflows/ci.yml`):
   - Builds the binary and web assets
   - Creates a timestamped deployment directory: `/tmp/soar/deploy/YYYYMMDDHHMMSS`
   - Copies all deployment artifacts (binary, service files, timer files) to the server
   - Executes **one** sudo command: `/usr/local/bin/soar-deploy /tmp/soar/deploy/YYYYMMDDHHMMSS`

2. **Deployment Script** (`/usr/local/bin/soar-deploy`):
   - Stops all SOAR services and timers
   - Backs up the current binary
   - Installs the new binary to `/usr/local/bin/soar`
   - Installs service files to `/etc/systemd/system/`
   - Reloads systemd daemon
   - Enables and starts all services
   - Checks service health with journalctl
   - Cleans up old backups and deployment directories

3. **Sudoers Configuration** (`/etc/sudoers.d/soar`):
   - Allows **only** the execution of `/usr/local/bin/soar-deploy`
   - No other sudo commands are permitted
   - Minimal attack surface

## Initial Server Setup

### Prerequisites

- A server with:
  - User named `soar` with SSH access
  - PostgreSQL with PostGIS installed
  - NATS server (optional, for real-time features)

### Installation Steps

1. **Install the sudoers configuration**:
   ```bash
   sudo cp infrastructure/sudoers.d/soar /etc/sudoers.d/soar
   sudo chmod 440 /etc/sudoers.d/soar
   sudo visudo -c  # Verify syntax is correct
   ```

2. **Install the deployment script**:
   ```bash
   sudo cp infrastructure/soar-deploy /usr/local/bin/soar-deploy
   sudo chmod 755 /usr/local/bin/soar-deploy
   sudo chown root:root /usr/local/bin/soar-deploy
   ```

3. **Create the deployment directory**:
   ```bash
   sudo mkdir -p /tmp/soar/deploy
   sudo chown soar:soar /tmp/soar/deploy
   ```

4. **Configure GitHub Secrets**:
   - `SSH_PRIVATE_KEY`: SSH private key for the `soar` user
   - `DEPLOY_SERVER`: Hostname or IP address of the deployment server

## Deployment Process

### Automatic Deployment (CI/CD)

When code is pushed to the `main` branch:

1. All tests run (Rust tests, SvelteKit tests, security audit)
2. A release binary is built
3. If all checks pass, the deployment job runs automatically
4. The deployment script handles all server-side operations

### Manual Deployment

If you need to deploy manually:

```bash
# On your local machine:
# 1. Build the binary
cargo build --release

# 2. Create a deployment package
TIMESTAMP=$(date +%Y%m%d%H%M%S)
mkdir -p deploy
cp target/release/soar deploy/
cp *.service deploy/
cp *.timer deploy/

# 3. Copy to server
DEPLOY_DIR="/tmp/soar/deploy/$TIMESTAMP"
ssh soar@your-server "mkdir -p $DEPLOY_DIR"
scp -r deploy/* soar@your-server:$DEPLOY_DIR/

# 4. Execute deployment
ssh soar@your-server "sudo /usr/local/bin/soar-deploy $DEPLOY_DIR"
```

## Service Files

The deployment manages these systemd services:

### Services
- **soar-run.service**: Main SOAR backend application
- **soar-web.service**: Web server component
- **soar-pull-data.service**: Data collection service
- **soar-sitemap.service**: Sitemap generation service
- **soar-archive.service**: Old fixes data archival service

### Timers
- **soar-pull-data.timer**: Periodic data collection
- **soar-sitemap.timer**: Periodic sitemap generation
- **soar-archive.timer**: Daily archival of old fixes data

## Monitoring

### Check Service Status

```bash
# Check if services are running
sudo systemctl is-active soar-run soar-web soar-pull-data soar-sitemap

# Get detailed status
sudo systemctl status soar-run
sudo systemctl status soar-web

# View logs
sudo journalctl -u soar-run -f
sudo journalctl -u soar-web -f
```

### Check Timer Status

```bash
# Check when timers will run next
sudo systemctl list-timers soar-*

# View timer logs
sudo journalctl -u soar-pull-data.timer -f
```

### Prometheus Metrics

SOAR exposes Prometheus-compatible metrics at `/data/metrics` on the web server. These metrics provide visibility into application performance and behavior.

#### Available Metrics

**Web Server Metrics** (`localhost:61225/data/metrics`):
- **http_request_duration_seconds** (histogram): Duration of HTTP requests to API endpoints
- **http_requests_total** (counter): Total number of HTTP requests
- **elevation_lookup_duration_seconds** (histogram): Time taken to perform elevation lookups, including tile download, GDAL operations, and bilinear interpolation

**APRS Client Metrics** (`localhost:9090/metrics`):
- **elevation_lookup_duration_seconds** (histogram): Time taken to perform elevation lookups during fix processing
- **flight_tracker_save_duration_seconds** (histogram): Time taken to persist flight tracker state to disk

All metrics are aggregated by endpoint pattern (e.g., `/data/devices/{id}` rather than specific IDs), making them suitable for dashboards and alerting.

#### Setting Up Prometheus

1. **Install Prometheus** on your monitoring server:
   ```bash
   # On Ubuntu/Debian
   sudo apt-get update
   sudo apt-get install prometheus

   # Or download directly from https://prometheus.io/download/
   ```

2. **Configure Prometheus** to scrape SOAR metrics:
   ```bash
   # Copy the SOAR Prometheus base configuration
   sudo cp infrastructure/prometheus.yml /etc/prometheus/prometheus.yml

   # Create the jobs directory and copy all job configurations
   sudo mkdir -p /etc/prometheus/jobs
   sudo cp infrastructure/prometheus-jobs/*.yml /etc/prometheus/jobs/
   sudo chown -R prometheus:prometheus /etc/prometheus/jobs
   sudo chmod 644 /etc/prometheus/jobs/*.yml
   ```

   **Note:** The new configuration uses `scrape_config_files` with a glob pattern. You only need to edit `prometheus.yml` to change global settings. Individual scrape jobs are managed in separate files in `/etc/prometheus/jobs/`.

3. **Restart Prometheus**:
   ```bash
   sudo systemctl restart prometheus
   sudo systemctl status prometheus
   ```

4. **Verify metrics are being collected**:
   - Open Prometheus UI: `http://your-prometheus-server:9090`
   - Navigate to Status → Targets
   - Verify all SOAR targets are "UP": `soar-web`, `soar-aprs-ingest`, `soar-run`, `soar-pull-data`
   - Query metrics: `elevation_lookup_duration_seconds`

5. **Adding new scrape jobs** (optional):
   To add a new SOAR service or other service to monitor:
   ```bash
   # Create a new job file
   sudo nano /etc/prometheus/jobs/my-new-service.yml

   # Prometheus will automatically load it within ~1 minute
   # Or force reload:
   sudo systemctl reload prometheus
   ```

   See `infrastructure/prometheus-jobs/README.md` for job configuration examples.

#### Accessing Metrics Directly

You can view raw metrics output by accessing the endpoints directly:

```bash
# Web server metrics (production)
curl http://localhost:61225/data/metrics

# Web server metrics (development)
curl http://localhost:1337/data/metrics

# APRS client metrics (run subcommand)
curl http://localhost:9090/metrics
```

#### Example Prometheus Queries

Once metrics are being scraped, you can use these example queries:

**Elevation Lookup Performance:**
```promql
# Average elevation lookup time over 5 minutes
rate(elevation_lookup_duration_seconds_sum[5m]) / rate(elevation_lookup_duration_seconds_count[5m])

# 95th percentile elevation lookup time
histogram_quantile(0.95, rate(elevation_lookup_duration_seconds_bucket[5m]))

# Total number of elevation lookups per second
sum(rate(elevation_lookup_duration_seconds_count[5m]))
```

**HTTP API Performance:**
```promql
# Average HTTP request duration
rate(http_request_duration_seconds_sum[5m]) / rate(http_request_duration_seconds_count[5m])

# 95th percentile HTTP request duration
histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))

# Requests per second
sum(rate(http_requests_total[5m]))
```

**Flight Tracker Performance:**
```promql
# Average flight tracker save time
rate(flight_tracker_save_duration_seconds_sum[5m]) / rate(flight_tracker_save_duration_seconds_count[5m])

# 95th percentile save time
histogram_quantile(0.95, rate(flight_tracker_save_duration_seconds_bucket[5m]))

# Save operations per second
sum(rate(flight_tracker_save_duration_seconds_count[5m]))
```

#### Grafana Dashboard (Optional)

For visualization, you can connect Grafana to Prometheus:

1. Install Grafana: `sudo apt-get install grafana`
2. Add Prometheus as a data source
3. Create dashboards using the queries above
4. Set up alerts for performance degradation

## Rollback

If a deployment fails, you can rollback to a previous version:

```bash
# List available backups
ls -lh /home/soar/soar.backup.*

# Restore a specific backup (as root)
sudo systemctl stop soar-run soar-web
sudo cp /home/soar/soar.backup.TIMESTAMP /usr/local/bin/soar
sudo chmod +x /usr/local/bin/soar
sudo chown root:root /usr/local/bin/soar
sudo systemctl start soar-run soar-web
```

The deployment script automatically keeps the last 5 backups.

## Troubleshooting

### Deployment Fails

1. Check the GitHub Actions log for detailed error messages
2. SSH into the server and check service logs:
   ```bash
   sudo journalctl -u soar-run --no-pager --lines=50
   sudo journalctl -u soar-web --no-pager --lines=50
   ```

### Services Won't Start

1. Check the service status:
   ```bash
   sudo systemctl status soar-run
   ```

2. Check for configuration issues:
   ```bash
   /usr/local/bin/soar --help
   ```

3. Verify database connectivity:
   ```bash
   psql -h localhost -U soar -d soar
   ```

### Permission Issues

If the deployment script fails due to permissions:

1. Verify sudoers configuration:
   ```bash
   sudo visudo -c
   sudo cat /etc/sudoers.d/soar
   ```

2. Ensure the deployment script is executable:
   ```bash
   ls -l /usr/local/bin/soar-deploy
   ```

## Security Considerations

- The `soar` user can only execute `/usr/local/bin/soar-deploy` with sudo
- The deployment script validates input and provides detailed logging
- All deployment artifacts are stored in `/tmp/soar/deploy/` with limited retention
- Binary backups are kept for rollback purposes
- Old deployment directories and backups are automatically cleaned up

## Maintenance

### Cleanup

The deployment script automatically cleans up:
- Keeps the last 5 binary backups
- Keeps the last 3 deployment directories

To manually clean up:

```bash
# Remove old backups
ls -t /home/soar/soar.backup.* | tail -n +6 | xargs rm -f

# Remove old deployment directories
ls -td /tmp/soar/deploy/* | tail -n +4 | xargs rm -rf
```

### Updating the Deployment Script

If you need to modify the deployment script:

1. Update `infrastructure/soar-deploy`
2. Deploy it manually to the server:
   ```bash
   scp infrastructure/soar-deploy soar@your-server:/tmp/soar-deploy-new
   ssh soar@your-server "sudo mv /tmp/soar-deploy-new /usr/local/bin/soar-deploy && sudo chmod 755 /usr/local/bin/soar-deploy"
   ```

### Updating Service Files

Service file changes are automatically deployed with each deployment. The deployment script:
1. Copies new service files to `/etc/systemd/system/`
2. Runs `systemctl daemon-reload`
3. Enables and starts the services

## Further Reading

- [Systemd Service Management](https://www.freedesktop.org/software/systemd/man/systemctl.html)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Sudoers Manual](https://www.sudo.ws/docs/man/sudoers.man/)
