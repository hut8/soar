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

### Elevation Data Setup

SOAR uses SRTM elevation data in HGT format to calculate Above Ground Level (AGL) altitudes for aircraft. The elevation data must be downloaded to `/var/soar/elevation` before SOAR can process position reports with AGL calculations.

#### Quick Setup with rclone

The easiest way to populate the elevation data is using rclone to download from our S3 bucket:

```bash
# Install rclone if not already installed
sudo apt-get update && sudo apt-get install -y rclone

# Download elevation tiles from S3 (this may take some time, ~50GB+ of data)
# Using --size-only to efficiently skip files that already match by size
sudo rclone copy --progress --size-only \
  :s3,provider=AWS,anonymous=true,region=us-east-1:elevation-tiles-prod/skadi \
  /var/soar/elevation

# Set proper ownership for the soar user
sudo chown -R soar:soar /var/soar/elevation
```

The download includes worldwide SRTM elevation tiles in gzipped HGT format. The tiles are organized by latitude into subdirectories (e.g., `N45/`, `S12/`), with each tile named by its southwest corner coordinates (e.g., `N45E009.hgt.gz`).

The `--size-only` flag makes rclone skip files that already have matching sizes, allowing you to safely resume interrupted downloads without re-downloading or checksumming existing files.

**Note:** The `scripts/provision` script automatically handles this setup during server provisioning.

#### Manual Setup (Alternative)

If you prefer to download elevation data manually or need specific regions:

1. **Create the directory structure**:
   ```bash
   sudo mkdir -p /var/soar/elevation
   sudo chown soar:soar /var/soar/elevation
   ```

2. **Download SRTM tiles**:
   - **Viewfinder Panoramas** (recommended): http://viewfinderpanoramas.org/Coverage%20map%20viewfinderpanoramas_org3.htm
   - **NASA Earthdata**: https://search.earthdata.nasa.gov/ (search for "SRTM")

3. **Organize tiles**:
   ```
   /var/soar/elevation/
   ├── N00/
   │   ├── N00E000.hgt.gz
   │   └── N00E001.hgt.gz
   ├── N45/
   │   ├── N45E009.hgt.gz
   │   └── ...
   └── S45/
       └── S45W009.hgt.gz
   ```

4. **Set ownership**:
   ```bash
   sudo chown -R soar:soar /var/soar/elevation
   ```

#### Verification

After populating the elevation data, verify the setup:

```bash
# Check directory structure
ls -lh /var/soar/elevation/

# Verify a specific tile (example)
ls -lh /var/soar/elevation/N45/N45E009.hgt.gz

# Test decompression
gzip -t /var/soar/elevation/N45/N45E009.hgt.gz
```

#### Coverage

The elevation service gracefully handles missing tiles (e.g., ocean areas) by returning `None` for elevation lookups. You can start with just the regions you need and add more tiles later as coverage requirements expand.

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

## ADS-B Ingester Deployment

The ADS-B ingester (`soar ingest-beast`) runs on a separate server accessible only via Tailscale, using a simplified deployment workflow.

### Overview

The ADS-B deployment differs from the main deployment in several ways:

- **Separate server**: Runs on a different machine (not the main SOAR server)
- **Tailscale-only access**: Server is not publicly accessible, only via Tailscale network
- **Single service**: Only deploys `soar-beast-ingest.service`
- **No database**: The ingester only publishes to NATS JetStream, no database access required
- **Simplified script**: Uses `soar-deploy-adsb` instead of the full `soar-deploy` script

### Architecture

```
┌─────────────────┐         ┌──────────────────┐         ┌─────────────────┐
│  dump1090/      │         │  SOAR ADS-B      │         │  Main Server    │
│  readsb         │────────>│  Ingester        │────────>│  (NATS)         │
│  (Beast:30005)  │         │  ingest-beast    │         │  glider.flights │
└─────────────────┘         └──────────────────┘         └─────────────────┘
                                    │
                                    │ via Tailscale
                                    │
                            ┌──────────────────┐
                            │  GitHub Actions  │
                            │  (Deployment)    │
                            └──────────────────┘
```

### Initial Setup

For first-time setup of the ADS-B server, see **[ADSB-SETUP.md](ADSB-SETUP.md)** which covers:

1. Server provisioning (user, directories, permissions)
2. Installing deployment script and sudoers configuration
3. Environment configuration (`/etc/soar/env`)
4. Tailscale setup and OAuth configuration
5. GitHub Secrets configuration
6. First deployment test
7. Verification and monitoring setup

### Deployment Workflow

**File**: `.github/workflows/deploy-adsb.yml`

The ADS-B deployment workflow:

1. **Reuses build artifacts** from the main CI/CD build job
2. **Connects via Tailscale** using OAuth credentials (ephemeral connection)
3. **Deploys via SSH** over Tailscale network (no public SSH exposure)
4. **Executes deployment script**: `sudo /usr/local/bin/soar-deploy-adsb`

### GitHub Secrets Required

| Secret Name | Description |
|-------------|-------------|
| `TAILSCALE_OAUTH_CLIENT_ID` | Tailscale OAuth client ID for GitHub Actions |
| `TAILSCALE_OAUTH_SECRET` | Tailscale OAuth secret |
| `SSH_PRIVATE_KEY` | SSH private key for soar user (reused from existing deployments) |
| `ADSB_SERVER_HOSTNAME` | Tailscale hostname or IP (e.g., `100.x.x.x`) |

### Deployment Triggers

**Manual Deployment** (default):
```
GitHub Actions → Deploy ADS-B Ingester → Run workflow → Select environment
```

**Automatic Deployment** (optional, uncomment in workflow file):
- Triggers on push to `main` branch
- Only when ADS-B-related files change

### Files Deployed

The ADS-B deployment package includes:
- `soar` binary
- `soar-beast-ingest.service` service file
- `soar-deploy-adsb` deployment script
- `VERSION` file (git commit hash)

### Monitoring ADS-B Deployment

**Check service status**:
```bash
# Via SSH (over Tailscale)
ssh soar@ADSB_SERVER

# Check service
sudo systemctl status soar-beast-ingest

# View logs
sudo journalctl -u soar-beast-ingest -f

# Check metrics
curl http://localhost:9094/metrics | grep beast
```

**Verify data flow on main server**:
```bash
# Check NATS stream is receiving data
nats stream info BEAST_RAW

# Should show increasing message count
```

### Troubleshooting ADS-B Deployment

**Deployment fails - Tailscale connection**:
```bash
# Verify OAuth credentials in GitHub Secrets
# Check Tailscale ACLs allow tag:github-actions → adsb-server
```

**Service won't start**:
```bash
# Check NATS connectivity
telnet <NATS_TAILSCALE_IP> 4222

# Verify environment file
sudo cat /etc/soar/env | grep NATS_URL

# Check Beast server connectivity
telnet localhost 30005
```

**No data in NATS stream**:
```bash
# Verify Beast server (dump1090) is running and producing data
ss -tlnp | grep 30005
nc localhost 30005 | head -c 100  # Should see binary data

# Check ADS-B ingester logs for errors
sudo journalctl -u soar-beast-ingest -n 100
```

### Differences from Main Deployment

| Aspect | Main Deployment | ADS-B Deployment |
|--------|----------------|------------------|
| Deployment script | `soar-deploy` | `soar-deploy-adsb` |
| Services deployed | All SOAR services | Only `soar-beast-ingest` |
| Database access | Required | Not required |
| Access method | Public SSH | Tailscale SSH only |
| Workflow file | `ci.yml` | `deploy-adsb.yml` |
| Binary location | `/usr/local/bin/soar` | `/usr/local/bin/soar` |
| Environment file | `/etc/soar/env` | `/etc/soar/env` |

### Security Notes

- **No public access**: ADS-B server only accessible via Tailscale
- **Minimal privileges**: Sudoers only allows deployment script execution
- **Ephemeral connections**: GitHub Actions nodes automatically removed from Tailscale
- **Non-root service**: Service runs as `soar` user
- **ACL restrictions**: Tailscale ACLs limit access to required ports only

For detailed setup instructions, see **[ADSB-SETUP.md](ADSB-SETUP.md)**.

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
   # Copy the SOAR Prometheus base configuration (one-time setup)
   sudo cp infrastructure/prometheus.yml /etc/prometheus/prometheus.yml
   ```

   **Note:** The Prometheus job files are automatically deployed by the `soar-deploy` script when you deploy SOAR. The deployment process creates `/etc/prometheus/jobs/` and copies all job configuration files from `infrastructure/prometheus-jobs/`. You only need to manually copy `prometheus.yml` once during initial setup.

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
