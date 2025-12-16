# ADS-B Ingester Initial Server Setup

This document describes the one-time manual setup required for deploying the SOAR ADS-B ingester (`soar ingest-adsb`) to a separate server accessible only via Tailscale.

## Prerequisites

- ADS-B server running Ubuntu 22.04 or compatible Linux distribution
- Server connected to Tailscale network
- Access to Tailscale admin console
- Beast protocol data source (e.g., dump1090, readsb) running on port 30005
- SSH access to the ADS-B server
- GitHub repository admin access (to add secrets)

## Architecture Overview

```
┌─────────────────┐         ┌──────────────────┐         ┌─────────────────┐
│  dump1090/      │         │  SOAR ADS-B      │         │  Main Server    │
│  readsb         │────────>│  Ingester        │────────>│  (NATS)         │
│  (Beast:30005)  │         │  soar-adsb-      │         │  glider.flights │
└─────────────────┘         │  ingest.service  │         └─────────────────┘
                            └──────────────────┘
                                    │
                                    │ via Tailscale
                                    │
                            ┌──────────────────┐
                            │  GitHub Actions  │
                            │  (Deployment)    │
                            └──────────────────┘
```

**Data Flow:**
1. dump1090/readsb → Beast protocol (TCP port 30005) → ADS-B Ingester
2. ADS-B Ingester → NATS JetStream (via Tailscale) → Main Server
3. GitHub Actions → Deploy updates (via Tailscale SSH) → ADS-B Server

## Step 1: Provision ADS-B Server

### 1.1 Create soar User and Directories

```bash
# On ADS-B server as root or sudo user
sudo adduser soar --disabled-password --gecos "SOAR ADS-B Service User"

# Create required directories
sudo mkdir -p /var/soar /etc/soar /tmp/soar/deploy /home/soar/backups

# Set ownership
sudo chown -R soar:soar /var/soar /tmp/soar /home/soar/backups

# Create bin directory for temporary binaries
sudo mkdir -p /var/soar/bin
sudo chown soar:soar /var/soar/bin
```

### 1.2 Verify Directory Structure

```bash
ls -la /var/soar      # Should be owned by soar:soar
ls -la /etc/soar      # Should exist (may be owned by root)
ls -la /home/soar     # Should contain backups directory
```

## Step 2: Install Deployment Infrastructure

### 2.1 Copy Files from Development Machine

```bash
# On your development machine, from the soar repository root
# Replace ADSB_SERVER with your Tailscale hostname or IP

# Copy deployment script
scp infrastructure/soar-deploy-adsb soar@ADSB_SERVER:/tmp/

# Copy sudoers file
scp infrastructure/sudoers.d/soar-adsb soar@ADSB_SERVER:/tmp/

# Copy environment template (optional, for reference)
scp infrastructure/examples/etc-soar-env-adsb.template soar@ADSB_SERVER:/tmp/
```

### 2.2 Install Deployment Script

```bash
# On ADS-B server
ssh soar@ADSB_SERVER

# Install deployment script
sudo install -m 755 -o root -g root /tmp/soar-deploy-adsb /usr/local/bin/soar-deploy-adsb

# Verify installation
ls -la /usr/local/bin/soar-deploy-adsb
# Should show: -rwxr-xr-x 1 root root ... /usr/local/bin/soar-deploy-adsb
```

### 2.3 Install Sudoers Configuration

```bash
# Still on ADS-B server

# Install sudoers file
sudo install -m 440 /tmp/soar-adsb /etc/sudoers.d/soar

# CRITICAL: Verify sudoers syntax
sudo visudo -c

# Expected output: "parsed OK"
# If there are errors, fix them before proceeding
```

### 2.4 Test Sudo Permissions

```bash
# Still on ADS-B server, as soar user

# This should NOT prompt for password and should show usage message
sudo /usr/local/bin/soar-deploy-adsb

# Expected output:
# [ERROR] Usage: /usr/local/bin/soar-deploy-adsb /tmp/soar/deploy/YYYYMMDDHHMMSS
```

## Step 3: Configure Environment

### 3.1 Identify NATS Server Tailscale Address

```bash
# On main server (glider.flights)
tailscale ip -4

# Note this IP address (e.g., 100.x.x.x)
# OR use Tailscale MagicDNS hostname (e.g., glider-flights.your-tailnet.ts.net)
```

### 3.2 Create Environment File

```bash
# On ADS-B server

# Create environment file
sudo tee /etc/soar/env > /dev/null <<'EOF'
# SOAR ADS-B Ingester Environment Configuration

# NATS Configuration (REQUIRED)
# Replace with your main server's Tailscale IP or hostname
NATS_URL=nats://100.x.x.x:4222

# Beast Server Configuration
# Adjust if dump1090 is on a different host/port
BEAST_SERVER=localhost
BEAST_PORT=30005

# Application Configuration
RUST_LOG=info
SOAR_ENV=production
METRICS_PORT=9094

# Optional: Error Tracking
# Add your Sentry DSN if using Sentry
SENTRY_DSN=
EOF

# Set secure permissions
sudo chmod 600 /etc/soar/env
sudo chown root:soar /etc/soar/env

# Verify file contents
sudo cat /etc/soar/env
```

### 3.3 Verify NATS Connectivity

```bash
# On ADS-B server
# Test connectivity to NATS server via Tailscale

# Install telnet if not already installed
sudo apt-get update && sudo apt-get install -y telnet

# Test connection (replace with your NATS IP)
telnet 100.x.x.x 4222

# Expected: Connection should succeed and show NATS banner
# Press Ctrl+] then type "quit" to exit

# If connection fails:
# 1. Check Tailscale connectivity: ping 100.x.x.x
# 2. Verify NATS is listening: on main server, run: ss -tlnp | grep 4222
# 3. Check Tailscale ACLs (see Step 4)
```

## Step 4: Configure Tailscale

### 4.1 Setup OAuth for GitHub Actions

1. Go to **Tailscale Admin Console** → **Settings** → **OAuth Clients**
2. Click **Generate OAuth Client**
3. Configure:
   - **Description**: `GitHub Actions - SOAR ADS-B Deployment`
   - **Tags**: `tag:github-actions`
   - **Expiry**: No expiry (or set as needed)
4. Click **Generate**
5. **IMPORTANT**: Copy the Client ID and Client Secret immediately (shown only once)
6. Save these for Step 5 (GitHub Secrets)

### 4.2 Verify Tailscale ACLs

Ensure your Tailscale ACLs allow:

```json
{
  "acls": [
    // Allow ADS-B server to access NATS on main server
    {
      "action": "accept",
      "src": ["tag:adsb-server"],
      "dst": ["tag:production-server:4222"]
    },
    // Allow GitHub Actions to SSH to ADS-B server
    {
      "action": "accept",
      "src": ["tag:github-actions"],
      "dst": ["tag:adsb-server:22"]
    }
  ]
}
```

**Note**: Adjust tags and hostnames based on your Tailscale setup.

### 4.3 Tag ADS-B Server

```bash
# On ADS-B server
tailscale set --advertise-tags=tag:adsb-server
```

Or tag via Tailscale admin console:
1. Go to **Machines**
2. Find your ADS-B server
3. Click **...** → **Edit settings**
4. Add tag: `adsb-server`

## Step 5: Setup GitHub Deployment

### 5.1 Generate SSH Key for Deployment

```bash
# On your development machine (not the ADS-B server)

# Generate deployment key
ssh-keygen -t ed25519 -f ~/.ssh/adsb_deploy -C "github-actions-adsb-deploy" -N ""

# Copy public key to ADS-B server
ssh-copy-id -i ~/.ssh/adsb_deploy.pub soar@ADSB_SERVER

# Test SSH connection
ssh -i ~/.ssh/adsb_deploy soar@ADSB_SERVER "echo SSH connection successful"

# Display private key (for copying to GitHub Secrets)
cat ~/.ssh/adsb_deploy

# IMPORTANT: Copy this entire private key content for next step
```

### 5.2 Add GitHub Repository Secrets

Go to your GitHub repository → **Settings** → **Secrets and variables** → **Actions** → **New repository secret**

Add the following secrets:

| Secret Name | Value | Example |
|------------|-------|---------|
| `TAILSCALE_OAUTH_CLIENT_ID` | OAuth Client ID from Step 4.1 | `k123abc...` |
| `TAILSCALE_OAUTH_SECRET` | OAuth Client Secret from Step 4.1 | `tskey-client-k123abc...` |
| `SSH_PRIVATE_KEY` | SSH private key (already exists for main deployments) | `-----BEGIN OPENSSH PRIVATE KEY-----\n...` |
| `ADSB_SERVER_HOSTNAME` | Tailscale hostname or IP | `100.x.x.x` or `adsb.tailnet.ts.net` |

**Security Note**: These secrets are encrypted and only accessible to GitHub Actions workflows. The `SSH_PRIVATE_KEY` secret is reused from existing main/staging deployments.

### 5.3 Verify Secrets

After adding secrets, verify they appear in the secrets list:

- Go to **Settings** → **Secrets and variables** → **Actions**
- You should see `TAILSCALE_OAUTH_CLIENT_ID`, `TAILSCALE_OAUTH_SECRET`, and `ADSB_SERVER_HOSTNAME` (SSH_PRIVATE_KEY should already exist)

## Step 6: Test Deployment

### 6.1 Trigger Manual Deployment

1. Go to GitHub repository → **Actions**
2. Select **Deploy ADS-B Ingester** workflow
3. Click **Run workflow**
4. Select **production** environment
5. Click **Run workflow**

### 6.2 Monitor Deployment

Watch the workflow progress in the Actions tab. The deployment should:

1. ✅ Download binary artifact
2. ✅ Setup Tailscale
3. ✅ Upload deployment package
4. ✅ Execute deployment script
5. ✅ Verify service is running

### 6.3 Verify Deployment on Server

```bash
# SSH to ADS-B server
ssh soar@ADSB_SERVER

# Check service status
sudo systemctl status soar-adsb-ingest

# Expected: "Active: active (running)"

# Check recent logs
sudo journalctl -u soar-adsb-ingest -n 50

# Look for:
# - "Connected to NATS"
# - "Connected to Beast server"
# - "Publishing messages to NATS JetStream"

# Check metrics endpoint
curl http://localhost:9094/metrics | grep beast

# Expected: Various beast.* metrics
```

### 6.4 Verify Data Flow on Main Server

```bash
# SSH to main server (glider.flights)

# Check NATS stream
nats stream info BEAST_RAW

# Expected: Message count should be increasing
# If count is 0, check:
# 1. ADS-B server logs for NATS connection errors
# 2. Beast data source (dump1090) is producing data
# 3. Network connectivity between servers
```

## Step 7: Configure Monitoring (Optional)

### 7.1 Add Prometheus Scrape Target

On the main server, add ADS-B metrics endpoint to Prometheus:

```yaml
# /etc/prometheus/prometheus.yml

scrape_configs:
  - job_name: 'soar-adsb-ingest'
    static_configs:
      - targets: ['100.x.x.x:9094']  # ADS-B server Tailscale IP
        labels:
          instance: 'adsb-ingest'
          environment: 'production'
```

Reload Prometheus:
```bash
sudo systemctl reload prometheus
```

### 7.2 Create Grafana Dashboard

See `infrastructure/grafana-dashboard-aprs-ingest.json` as a template for creating a Beast ingester dashboard.

## Troubleshooting

### Service Won't Start

```bash
# Check service status
sudo systemctl status soar-adsb-ingest

# Check full logs
sudo journalctl -u soar-adsb-ingest -n 100 --no-pager

# Common issues:
# 1. Binary not found: Check /usr/local/bin/soar exists
# 2. Permission denied: Check soar user permissions
# 3. Environment file: Verify /etc/soar/env exists and is readable
```

### NATS Connection Failed

```bash
# Test NATS connectivity
telnet 100.x.x.x 4222

# Check environment file
sudo cat /etc/soar/env | grep NATS_URL

# Verify Tailscale connectivity
tailscale status
ping 100.x.x.x

# Check Tailscale ACLs allow traffic to port 4222
```

### Beast Connection Failed

```bash
# Check Beast server is running
ss -tlnp | grep 30005

# If dump1090 is on the same machine:
telnet localhost 30005

# Check environment file
sudo cat /etc/soar/env | grep BEAST

# Verify Beast server is producing data
nc localhost 30005 | head -c 100
# Should see binary data
```

### Deployment Failed

```bash
# Check GitHub Actions logs for detailed error
# Common issues:

# 1. Tailscale connection failed
#    - Verify OAuth client/secret in GitHub Secrets
#    - Check ACLs allow tag:github-actions

# 2. SSH connection failed
#    - Verify SSH key in GitHub Secrets is correct
#    - Test: ssh -i ~/.ssh/adsb_deploy soar@ADSB_SERVER

# 3. Deployment script failed
#    - SSH to server and check: sudo visudo -c
#    - Verify: ls -la /usr/local/bin/soar-deploy-adsb
```

## Maintenance

### Update Environment Configuration

```bash
# Edit environment file
sudo nano /etc/soar/env

# Restart service to apply changes
sudo systemctl restart soar-adsb-ingest

# Verify service started successfully
sudo systemctl status soar-adsb-ingest
```

### Manual Deployment

```bash
# On development machine
# Build binary locally
cargo build --release

# Create deployment directory on server
TIMESTAMP=$(date +%Y%m%d%H%M%S)
ssh soar@ADSB_SERVER "mkdir -p /tmp/soar/deploy/$TIMESTAMP"

# Copy files
scp target/release/soar soar@ADSB_SERVER:/tmp/soar/deploy/$TIMESTAMP/
scp infrastructure/systemd/soar-adsb-ingest.service soar@ADSB_SERVER:/tmp/soar/deploy/$TIMESTAMP/
scp infrastructure/soar-deploy-adsb soar@ADSB_SERVER:/tmp/soar/deploy/$TIMESTAMP/

# Execute deployment
ssh soar@ADSB_SERVER "sudo /usr/local/bin/soar-deploy-adsb /tmp/soar/deploy/$TIMESTAMP"
```

### View Logs

```bash
# Real-time logs
sudo journalctl -u soar-adsb-ingest -f

# Last 100 lines
sudo journalctl -u soar-adsb-ingest -n 100

# Logs from specific time
sudo journalctl -u soar-adsb-ingest --since "1 hour ago"

# Logs with specific level
sudo journalctl -u soar-adsb-ingest -p err
```

### Rollback Deployment

```bash
# List available backups
ls -lt /home/soar/backups/

# Stop service
sudo systemctl stop soar-adsb-ingest

# Restore backup
sudo cp /home/soar/backups/soar.backup.TIMESTAMP /usr/local/bin/soar

# Start service
sudo systemctl start soar-adsb-ingest

# Verify
sudo systemctl status soar-adsb-ingest
```

## Security Checklist

- [ ] soar user has no shell access (default with --disabled-password)
- [ ] /etc/soar/env has permissions 600 (only readable by root and soar group)
- [ ] sudoers file has permissions 440 and passes visudo -c
- [ ] SSH key for deployment is unique and not reused elsewhere
- [ ] GitHub Secrets are properly configured and not exposed in logs
- [ ] Tailscale ACLs restrict access appropriately
- [ ] NATS connection uses Tailscale (not public internet)
- [ ] Service runs as non-root user (soar)
- [ ] Metrics endpoint only accessible via Tailscale (not public)

## Next Steps

After successful deployment:

1. **Monitor for 24 hours**: Check logs and NATS stream message count
2. **Set up alerting**: Create alerts for service down, NATS disconnected
3. **Document specifics**: Record Tailscale IPs, server hostnames in runbook
4. **Enable automatic deployment**: Uncomment `push` trigger in workflow (optional)
5. **Create Grafana dashboard**: Visualize Beast ingestion metrics

## Support

For issues or questions:
- Check deployment logs: GitHub Actions → Deploy ADS-B Ingester
- Check service logs: `sudo journalctl -u soar-adsb-ingest`
- Review this setup guide for missed steps
- Check Tailscale connectivity: `tailscale status`
