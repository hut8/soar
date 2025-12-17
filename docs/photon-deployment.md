# Photon Geocoding Server Deployment Guide

This guide covers deploying [Photon](https://github.com/komoot/photon), an open-source geocoder for OpenStreetMap data, as a systemd service.

## Overview

Photon is a fast, scalable geocoding server built on OpenSearch/Elasticsearch. It provides:
- Forward geocoding (address → coordinates)
- Reverse geocoding (coordinates → address)
- Search suggestions and autocomplete
- Multi-language support

## System Requirements

### Minimum Requirements
- **RAM**: 8GB minimum, **64GB recommended** for production
- **Disk**: Varies by coverage area (worldwide data ~50-100GB)
- **Java**: OpenJDK 17 or newer
- **OS**: Linux (Ubuntu/Debian recommended)

### Performance Considerations
- More RAM = better performance under load
- Consider increasing Java heap size for large datasets: `-Xmx12G` or higher
- SSD storage recommended for data directory

## Installation Steps

### 1. Install Java

```bash
# Ubuntu/Debian
sudo apt update
sudo apt install openjdk-17-jre-headless

# Verify installation
java -version
```

### 2. Create System User

Create a dedicated user for running Photon:

```bash
sudo useradd --system --home-dir /opt/photon --shell /bin/false photon
```

### 3. Create Directory Structure

```bash
# Application directory (for JAR files)
sudo mkdir -p /opt/photon

# Data directory (for OpenSearch data)
sudo mkdir -p /var/lib/photon

# Set ownership
sudo chown -R photon:photon /opt/photon
sudo chown -R photon:photon /var/lib/photon
```

Note: Logs are automatically managed by journald, so no separate log directory is needed.

### 4. Download Photon

Download the latest release from [GitHub Releases](https://github.com/komoot/photon/releases):

```bash
# Download the latest photon-opensearch JAR
cd /opt/photon
sudo -u photon wget https://github.com/komoot/photon/releases/download/[VERSION]/photon-[VERSION].jar

# Create a symlink for easier management
sudo -u photon ln -s photon-[VERSION].jar photon.jar
```

Replace `[VERSION]` with the latest version number (e.g., `0.5.0`).

### 5. Download and Import Data

#### Option A: Download Pre-built Data

Download pre-built Photon data for your region:

```bash
cd /var/lib/photon

# Example: Download worldwide data (large!)
sudo -u photon wget https://download1.graphhopper.com/public/photon-db-latest.tar.bz2

# Extract data
sudo -u photon tar xjf photon-db-latest.tar.bz2
sudo -u photon rm photon-db-latest.tar.bz2
```

#### Option B: Import from Nominatim

If you have a Nominatim database:

```bash
sudo -u photon java -jar /opt/photon/photon.jar \
  -nominatim-import \
  -host localhost \
  -port 5432 \
  -database nominatim \
  -user nominatim \
  -password your_password \
  -languages en \
  -data-dir /var/lib/photon
```

## Configuration

### Create systemd Service File

Create `/etc/systemd/system/photon.service`:

```ini
[Unit]
Description=Photon Geocoding Server
Documentation=https://github.com/komoot/photon
After=network.target

[Service]
Type=simple
User=photon
Group=photon

# Working directory
WorkingDirectory=/opt/photon

# Java heap size - adjust based on available RAM
# Use 50-75% of available RAM for production
Environment="JAVA_OPTS=-Xmx12G -Xms12G"

# Photon command
ExecStart=/usr/bin/java ${JAVA_OPTS} -jar /opt/photon/photon.jar \
    -data-dir /var/lib/photon \
    -listen-ip 127.0.0.1 \
    -listen-port 2322 \
    -cors-any

# Logging to journald (systemd's logging system)
# View logs with: journalctl -u photon -f
StandardOutput=journal
StandardError=journal
SyslogIdentifier=photon

# Restart policy
Restart=on-failure
RestartSec=10s

# Resource limits
LimitNOFILE=65535

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/photon

[Install]
WantedBy=multi-user.target
```

### Configuration Options

Common command-line options for Photon:

| Option | Description | Default |
|--------|-------------|---------|
| `-data-dir` | Path to OpenSearch data directory | `./photon_data` |
| `-listen-ip` | IP address to bind to | `0.0.0.0` |
| `-listen-port` | Port to listen on | `2322` |
| `-cors-any` | Enable CORS for all origins | disabled |
| `-languages` | Supported languages (comma-separated) | all |
| `-enable-update-api` | Enable update API endpoint | disabled |
| `-synonym-file` | Path to synonym configuration | none |

### Security Considerations

**For production deployments:**

1. **Bind to localhost only** (use reverse proxy):
   ```
   -listen-ip 127.0.0.1
   ```

2. **Set up nginx/Apache reverse proxy**:
   ```nginx
   location /api/geocode/ {
       proxy_pass http://127.0.0.1:2322/;
       proxy_set_header Host $host;
       proxy_set_header X-Real-IP $remote_addr;
   }
   ```

3. **Configure firewall**:
   ```bash
   sudo ufw allow 80/tcp
   sudo ufw allow 443/tcp
   # Do NOT expose port 2322 directly
   ```

4. **Enable HTTPS** with Let's Encrypt/certbot

## Managing the Service

### Enable and Start

```bash
# Reload systemd configuration
sudo systemctl daemon-reload

# Enable service to start on boot
sudo systemctl enable photon

# Start the service
sudo systemctl start photon

# Check status
sudo systemctl status photon
```

### Monitor Logs

Photon logs to journald (systemd's logging system). Use `journalctl` to view logs:

```bash
# View live logs (follow mode)
sudo journalctl -u photon -f

# View recent logs (last 100 lines)
sudo journalctl -u photon -n 100

# View logs from specific time
sudo journalctl -u photon --since "1 hour ago"
sudo journalctl -u photon --since "2025-01-15 10:00:00"

# View logs with priority (errors only)
sudo journalctl -u photon -p err

# Search logs for specific text
sudo journalctl -u photon | grep "error"

# Export logs to file
sudo journalctl -u photon > photon-logs.txt
```

**Journald advantages:**
- Automatic log rotation and retention management
- Integrated with systemd status
- Efficient storage with compression
- Advanced filtering and searching capabilities
- No need for separate log rotation configuration

### Restart/Stop Service

```bash
# Restart service
sudo systemctl restart photon

# Stop service
sudo systemctl stop photon

# Disable service from starting on boot
sudo systemctl disable photon
```

## Testing the Service

Once Photon is running, test it:

```bash
# Forward geocoding
curl "http://localhost:2322/api?q=Berlin"

# Reverse geocoding
curl "http://localhost:2322/reverse?lon=13.388860&lat=52.517037"

# Search with language preference
curl "http://localhost:2322/api?q=London&lang=en"
```

## Updating Photon

### Update JAR File

```bash
# Stop service
sudo systemctl stop photon

# Download new version
cd /opt/photon
sudo -u photon wget https://github.com/komoot/photon/releases/download/[NEW_VERSION]/photon-[NEW_VERSION].jar

# Update symlink
sudo -u photon ln -sf photon-[NEW_VERSION].jar photon.jar

# Start service
sudo systemctl start photon

# Check logs
sudo journalctl -u photon -f
```

### Update Data

If using pre-built data:

```bash
# Stop service
sudo systemctl stop photon

# Backup old data (optional)
sudo mv /var/lib/photon/photon_data /var/lib/photon/photon_data.backup

# Download and extract new data
cd /var/lib/photon
sudo -u photon wget https://download1.graphhopper.com/public/photon-db-latest.tar.bz2
sudo -u photon tar xjf photon-db-latest.tar.bz2
sudo -u photon rm photon-db-latest.tar.bz2

# Start service
sudo systemctl start photon
```

If using Nominatim import, re-run the import command.

## Troubleshooting

### Service won't start

```bash
# Check service status
sudo systemctl status photon

# Check logs for errors
sudo journalctl -u photon -n 50

# Verify Java installation
java -version

# Check file permissions
ls -la /opt/photon/photon.jar
ls -la /var/lib/photon/
```

### Out of Memory Errors

Increase Java heap size in `/etc/systemd/system/photon.service`:

```ini
Environment="JAVA_OPTS=-Xmx32G -Xms32G"
```

Then reload and restart:

```bash
sudo systemctl daemon-reload
sudo systemctl restart photon
```

### Port Already in Use

Check what's using port 2322:

```bash
sudo lsof -i :2322
```

Either stop the conflicting service or change Photon's port in the systemd service file.

### Data Directory Issues

Verify data directory exists and has correct permissions:

```bash
ls -la /var/lib/photon/photon_data/
sudo chown -R photon:photon /var/lib/photon/
```

## Performance Tuning

### Java Heap Size

Set heap size to 50-75% of available RAM:

- 16GB RAM → `-Xmx12G -Xms12G`
- 32GB RAM → `-Xmx24G -Xms24G`
- 64GB RAM → `-Xmx48G -Xms48G`

### OpenSearch Settings

For advanced tuning, you can modify OpenSearch settings in the data directory. This is typically not necessary for most deployments.

### System Limits

Increase file descriptor limits if needed:

```bash
# Edit /etc/security/limits.conf
photon soft nofile 65535
photon hard nofile 65535
```

## Integration with SOAR

To integrate Photon with the SOAR application:

1. Ensure Photon is running on localhost:2322
2. Configure SOAR to use Photon endpoint: `http://localhost:2322/api`
3. Use reverse proxy (nginx/Apache) to expose Photon if needed
4. Consider implementing caching layer for frequently accessed geocoding requests

## Monitoring and Maintenance

### Health Check

Create a simple health check script:

```bash
#!/bin/bash
# /usr/local/bin/photon-health-check.sh

RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" "http://localhost:2322/api?q=test")

if [ "$RESPONSE" -eq 200 ]; then
    echo "Photon is healthy"
    exit 0
else
    echo "Photon is unhealthy (HTTP $RESPONSE)"
    exit 1
fi
```

### Log Retention

Journald automatically manages log rotation and retention. To configure retention settings, edit `/etc/systemd/journald.conf`:

```ini
[Journal]
# Keep logs for 30 days
MaxRetentionSec=30d

# Limit journal size to 1GB
SystemMaxUse=1G

# Limit individual log file size
SystemMaxFileSize=100M
```

After changing journald configuration, restart the service:

```bash
sudo systemctl restart systemd-journald
```

View current journal disk usage:

```bash
journalctl --disk-usage
```

## Resources

- **Official Repository**: https://github.com/komoot/photon
- **Documentation**: https://github.com/komoot/photon/blob/master/README.md
- **Data Downloads**: https://download1.graphhopper.com/public/
- **Issue Tracker**: https://github.com/komoot/photon/issues

## Summary

This deployment guide provides a production-ready setup for Photon using systemd. The service will:

- Run as a dedicated `photon` user
- Store data in `/var/lib/photon`
- Log to journald (systemd's integrated logging)
- Automatically restart on failure
- Start on system boot
- Include security hardening with systemd protections

Adjust memory settings and configuration based on your specific requirements and available resources.
