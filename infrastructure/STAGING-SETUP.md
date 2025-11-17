# Staging Environment Setup Guide

This guide documents the one-time manual setup required on the production server to enable the staging environment.

## Overview

The staging environment runs alongside production on the same server with:
- **Separate database**: `soar_staging`
- **Separate NATS streams**: `STAGING_APRS_RAW` (already configured)
- **Separate services**: All services have `-staging` suffix
- **Different ports**: To avoid conflicts with production
- **3-day data retention**: Instead of production's 21-day retention
- **No backups**: Staging does not run backup services

## Prerequisites

- Production server access with sudo privileges
- PostgreSQL installed and running
- NATS server installed and running
- Prometheus and Grafana installed (for metrics)

## Setup Steps

### 1. Create Staging Database

```bash
# Switch to postgres user
sudo -u postgres psql

# Create staging database
CREATE DATABASE soar_staging OWNER soar;

# Connect to staging database
\c soar_staging

# Install PostGIS extension
CREATE EXTENSION IF NOT EXISTS postgis;

# Install pg_partman extension (for partitioning)
CREATE EXTENSION IF NOT EXISTS pg_partman;

# Verify extensions
\dx

# Exit psql
\q
```

### 2. Create Staging Environment File

Create `/etc/soar/env-staging` with the following content:

```bash
# SOAR Staging Environment Configuration
SOAR_ENV=staging
DATABASE_URL=postgresql://localhost/soar_staging
NATS_URL=nats://localhost:4222

# APRS Connection (same as production)
OGN_APRS_SERVER=aprs.glidernet.org
OGN_APRS_PORT=14580

# Application settings
BASE_URL=https://staging.glider.flights
SITEMAP_ROOT=/var/soar/sitemap-staging

# JWT and SMTP settings (copy from production /etc/soar/env)
JWT_SECRET=<copy_from_production>
SMTP_SERVER=<copy_from_production>
SMTP_PORT=<copy_from_production>
SMTP_USERNAME=<copy_from_production>
SMTP_PASSWORD=<copy_from_production>
FROM_EMAIL=<copy_from_production>
FROM_NAME="SOAR Staging"
GOOGLE_MAPS_API_KEY=<copy_from_production>

# Optional: Override metrics ports if needed (defaults should work)
# METRICS_PORT=9092  # for soar-run-staging (default auto-assigns based on SOAR_ENV)
# METRICS_PORT=9094  # for soar-aprs-ingest-staging
```

Set proper permissions:

```bash
sudo chown root:soar /etc/soar/env-staging
sudo chmod 640 /etc/soar/env-staging
```

### 3. Create Staging Directories

```bash
# Create archive directory for staging
sudo mkdir -p /var/soar/archive-staging
sudo chown soar:soar /var/soar/archive-staging
sudo chmod 755 /var/soar/archive-staging

# Create sitemap directory for staging
sudo mkdir -p /var/soar/sitemap-staging
sudo chown soar:soar /var/soar/sitemap-staging
sudo chmod 755 /var/soar/sitemap-staging

# Create logs directory (if not exists)
sudo mkdir -p /var/soar/logs
sudo chown soar:soar /var/soar/logs
sudo chmod 755 /var/soar/logs
```

### 4. Update Deployment Script

The updated `soar-deploy` script should already be installed from the first staging deployment. Verify it exists:

```bash
ls -l /usr/local/bin/soar-deploy
```

If needed, manually install it:

```bash
# This will be done automatically on first staging deployment, but can be done manually:
sudo cp infrastructure/soar-deploy /usr/local/bin/soar-deploy
sudo chmod 755 /usr/local/bin/soar-deploy
sudo chown root:root /usr/local/bin/soar-deploy
```

### 4a. Create Backup Directories

```bash
# Create backup directories for both environments
sudo mkdir -p /home/soar/backups/production
sudo mkdir -p /home/soar/backups/staging
sudo chown -R soar:soar /home/soar/backups
sudo chmod 755 /home/soar/backups
```

### 5. Update Sudoers Configuration

Update sudoers to allow staging deployments:

```bash
sudo cp infrastructure/sudoers.d/soar /etc/sudoers.d/soar
sudo chmod 440 /etc/sudoers.d/soar

# Verify syntax
sudo visudo -c
```

### 6. Verify NATS Streams

The staging NATS streams should already exist. Verify:

```bash
# Install NATS CLI if not already installed
# https://github.com/nats-io/natscli

# List streams
nats stream ls

# You should see:
# - APRS_RAW (production)
# - STAGING_APRS_RAW (staging)
```

If staging streams don't exist, create them:

```bash
# Create staging stream (if needed)
nats stream add STAGING_APRS_RAW \
  --subjects "staging.aprs.raw" \
  --storage file \
  --retention limits \
  --max-msgs=-1 \
  --max-bytes=-1 \
  --max-age=24h \
  --max-msg-size=-1 \
  --discard old
```

### 7. Configure Reverse Proxy

The `infrastructure/Caddyfile` already includes the staging configuration. After the first deployment, Caddy will automatically reload and pick up the configuration.

The staging site will be available at: **https://staging.glider.flights**

If you need to manually reload Caddy:

```bash
sudo systemctl reload caddy
```

**Note**: Caddy will automatically obtain and renew SSL certificates for staging.glider.flights.

### 8. First Deployment

The first staging deployment will happen automatically when you merge to `main` after these changes are deployed.

To manually trigger the first deployment (optional):

```bash
# From your local machine with the built binary and service files:
# 1. Upload deployment package
TIMESTAMP=$(date +%Y%m%d%H%M%S)
ssh soar@server "mkdir -p /tmp/soar/deploy/$TIMESTAMP"
scp soar *-staging.service *-staging.timer infrastructure/prometheus-jobs/* soar@server:/tmp/soar/deploy/$TIMESTAMP/

# 2. Execute deployment
ssh soar@server "sudo /usr/local/bin/soar-deploy staging /tmp/soar/deploy/$TIMESTAMP"
```

## Verification

After setup and first deployment, verify all services are running:

```bash
# Check service status
sudo systemctl status soar-web-staging
sudo systemctl status soar-aprs-ingest-staging
sudo systemctl status soar-run-staging

# Check timer status
sudo systemctl list-timers | grep staging

# Check logs
sudo journalctl -u soar-run-staging -n 50
sudo journalctl -u soar-web-staging -n 50

# Verify database connection
sudo -u soar bash -c 'source /etc/soar/env-staging && psql $DATABASE_URL -c "SELECT version FROM __diesel_schema_migrations ORDER BY version DESC LIMIT 5;"'

# Check metrics endpoints
curl http://localhost:9092/metrics  # soar-run-staging
curl http://localhost:9094/metrics  # soar-aprs-ingest-staging
curl http://localhost:61226/data/metrics  # soar-web-staging
```

## Binary Locations

| Environment | Binary Path |
|-------------|-------------|
| Production | `/usr/local/bin/soar` |
| Staging | `/usr/local/bin/soar-staging` |

**Important**: Staging and production use **separate binaries** so they can run different code versions simultaneously. This allows testing code in staging before deploying to production.

## Port Assignments

| Service | Production Port | Staging Port |
|---------|----------------|--------------|
| Web Server | 61225 | 61226 |
| APRS Run Metrics | 9091 | 9092 |
| APRS Ingest Metrics | 9093 | 9094 |

## NATS Streams/Subjects

| Stream | Subject | Environment |
|--------|---------|-------------|
| `APRS_RAW` | `aprs.raw` | Production |
| `STAGING_APRS_RAW` | `staging.aprs.raw` | Staging |

## Deployment Workflow

### Staging Deployment (Automatic)
- **Trigger**: Push to `main` branch
- **When**: After all tests pass
- **Command**: `soar-deploy staging /tmp/soar/deploy/<timestamp>`

### Production Deployment (Manual via GitHub Releases)
- **Trigger**: GitHub release published (e.g., `v1.0.0`)
- **When**: After all tests pass
- **Command**: `soar-deploy production /tmp/soar/deploy/<timestamp>`

## Monitoring

Staging metrics are available in Prometheus/Grafana with `environment="staging"` label:

- **Prometheus jobs**:
  - `soar-run-staging` (localhost:9092)
  - `soar-aprs-ingest-staging` (localhost:9094)
  - `soar-web-staging` (localhost:61226)

- **Grafana**: Filter dashboards by `environment="staging"` to view staging metrics

## Troubleshooting

### Database connection errors

```bash
# Verify staging database exists
sudo -u postgres psql -l | grep soar_staging

# Check environment file
cat /etc/soar/env-staging | grep DATABASE_URL

# Test connection
sudo -u soar bash -c 'source /etc/soar/env-staging && psql $DATABASE_URL -c "SELECT 1;"'
```

### Service won't start

```bash
# Check service file
systemctl cat soar-run-staging

# Check logs
sudo journalctl -u soar-run-staging -n 100 --no-pager

# Verify environment file exists
ls -l /etc/soar/env-staging
```

### NATS connection issues

```bash
# Verify NATS is running
systemctl status nats-server

# Check if staging streams exist
nats stream ls

# View stream info
nats stream info STAGING_APRS_RAW
```

### Archive not running

```bash
# Check timer status
systemctl status soar-archive-staging.timer

# Check last run
systemctl list-timers soar-archive-staging.timer

# Manually trigger archive
sudo systemctl start soar-archive-staging.service

# Check logs
sudo journalctl -u soar-archive-staging.service -n 100
```

## Cleanup (if needed)

To remove staging environment:

```bash
# Stop and disable all staging services
for svc in soar-web-staging soar-aprs-ingest-staging soar-run-staging; do
    sudo systemctl stop $svc
    sudo systemctl disable $svc
done

# Stop and disable all staging timers
for timer in soar-pull-data-staging soar-archive-staging soar-sitemap-staging partman-maintenance-staging; do
    sudo systemctl stop $timer.timer
    sudo systemctl disable $timer.timer
done

# Remove service files
sudo rm /etc/systemd/system/*-staging.service
sudo rm /etc/systemd/system/*-staging.timer
sudo systemctl daemon-reload

# Drop staging database (CAUTION: This deletes all data!)
sudo -u postgres dropdb soar_staging

# Remove staging directories
sudo rm -rf /var/soar/archive-staging
sudo rm -rf /var/soar/sitemap-staging

# Remove environment file
sudo rm /etc/soar/env-staging

# Remove Prometheus job files
sudo rm /etc/prometheus/jobs/*-staging.yml
```
