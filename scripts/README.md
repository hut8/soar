# Scripts

This directory contains database management, deployment, and utility scripts.

## Deployment Script

The `deploy` script allows you to deploy SOAR either locally (on the same server) or remotely (to a production server), mimicking the GitHub Actions deployment workflow.

### Usage

```bash
# Deploy current branch (will prompt for local or remote)
./scripts/deploy

# Deploy a specific branch
./scripts/deploy my-branch

# Local deployment (on the same server)
LOCAL_DEPLOY=1 ./scripts/deploy
DEPLOY_SERVER=local ./scripts/deploy

# Remote deployment
DEPLOY_SERVER=prod.example.com ./scripts/deploy
```

### Environment Variables

The deployment script supports the following optional environment variables:

```bash
# Sentry integration (optional)
SENTRY_AUTH_TOKEN=your-token      # Sentry authentication token
SENTRY_ORG=your-org               # Sentry organization slug
SENTRY_PROJECT=your-project       # Sentry project slug

# Deployment mode
LOCAL_DEPLOY=1                    # Force local deployment (on same server)
DEPLOY_SERVER=local               # 'local'/'localhost' for local, or hostname for remote

# SSH configuration (for remote deployment only)
SSH_PRIVATE_KEY_PATH=~/.ssh/id_rsa  # Path to SSH private key (default: ~/.ssh/id_rsa)

# Deployment options
SKIP_SENTRY=1                     # Skip Sentry debug symbols and release creation
SKIP_TESTS=1                      # Skip running tests (not recommended)
```

### Prerequisites

**For all deployments:**
1. Node.js and Rust toolchain for building
2. sentry-cli installed (optional, for Sentry integration)
3. The deployment script (`/usr/local/bin/soar-deploy`) installed on the target server

**For local deployment:**
- Must be run on the deployment server itself
- Requires sudo access to run `/usr/local/bin/soar-deploy`

**For remote deployment:**
- SSH access to the deployment server as the `soar` user
- SSH key authentication configured

### Deployment Process

The script performs the following steps:

1. Checks out the specified branch (or uses current branch)
2. Runs tests (cargo fmt, clippy, cargo test, npm lint, npm check)
3. Builds the web frontend (`npm run build`)
4. Builds the Rust release binary (`cargo build --release`)
5. Uploads debug symbols to Sentry (if configured)
6. Creates a Sentry release (if configured)
7. Prepares deployment package including:
   - `soar` binary
   - `infrastructure/soar-deploy` script
   - `*.service` and `*.timer` systemd files
   - Prometheus job configurations
   - Grafana provisioning and dashboards
   - Backup scripts
   - VERSION file with commit SHA
8. **For local deployment**: Copies package to `/tmp/soar/deploy/` and invokes `/usr/local/bin/soar-deploy` directly
   **For remote deployment**: Uploads package to server via SSH and executes deployment script remotely
9. The `soar-deploy` script stops services, runs migrations, installs files, and restarts services

### Examples

```bash
# Local deployment on the server itself
LOCAL_DEPLOY=1 ./scripts/deploy main

# Local deployment with Sentry integration
SENTRY_AUTH_TOKEN=xxx SENTRY_ORG=my-org SENTRY_PROJECT=soar \
LOCAL_DEPLOY=1 ./scripts/deploy

# Remote deployment with all features
SENTRY_AUTH_TOKEN=xxx SENTRY_ORG=my-org SENTRY_PROJECT=soar \
DEPLOY_SERVER=prod.example.com \
./scripts/deploy main

# Remote deployment without Sentry, skipping tests (for testing)
SKIP_SENTRY=1 SKIP_TESTS=1 DEPLOY_SERVER=staging.example.com \
./scripts/deploy feat/my-feature

# Interactive mode (will prompt for local vs remote)
./scripts/deploy
# When prompted, enter "local" for local deployment or hostname for remote
```

### Safety Features

- Confirms uncommitted changes before proceeding
- Shows deployment package contents
- Tests SSH connection before uploading
- Requires manual confirmation before executing deployment
- Returns to original branch after deployment (if branch was switched)

---

## Database Reset Script

**⚠️ WARNING: These scripts will permanently DROP and RECREATE the entire database!**

### Files

- `reset-db.py` - Main Python script that drops and recreates the database
- `reset-db.sh` - Shell wrapper for easier execution

### Prerequisites

- Python 3 with `psycopg` (psycopg3) library installed
- Database connection access
- Environment variables for database connection (optional)

### Environment Variables

The script will use these environment variables if available:

```bash
DB_HOST=localhost      # Database host (default: localhost)
DB_PORT=5432          # Database port (default: 5432)
DB_USER=postgres      # Database user (default: postgres)
DB_PASSWORD=secret    # Database password (optional)
```

### Usage

#### Python Script

```bash
# Dry run (safe - shows what would be done)
./scripts/reset-db.py dev --dry-run
./scripts/reset-db.py production --dry-run

# Actual reset (DANGEROUS!)
./scripts/reset-db.py dev
./scripts/reset-db.py production
```

#### Shell Wrapper

```bash
# Dry run
./scripts/reset-db.sh dev --dry-run
./scripts/reset-db.sh production --dry-run

# Actual reset
./scripts/reset-db.sh dev
./scripts/reset-db.sh production
```

### Safety Features

1. **Multiple confirmations** - requires typing specific phrases
2. **Extra confirmation for production** - additional safety step
3. **Dry run mode** - see what would be done without making changes
4. **Force disconnect clients** - terminates all connections before dropping
5. **Database existence check** - verifies database exists before attempting operations
6. **Admin connection** - uses postgres admin database for operations

### Database Names

- **dev**: `soar_dev`
- **production**: `soar`

### Example Output

```
Database Reset Script
=====================
🔍 Running in DRY RUN mode - no changes will be made
Connecting to postgres admin database...
Checking if database 'soar_dev' exists...

🔍 DRY RUN MODE - No actual changes will be made
Would execute the following operations:
  1. Terminate all connections to 'soar_dev'
  2. DROP DATABASE soar_dev;
  3. CREATE DATABASE soar_dev;
```

### Post-Reset Setup

After running the reset script, you'll need to run migrations to recreate the schema:

```bash
# For dev environment
diesel migration run --database-url postgresql://user:pass@host:port/soar_dev

# For production environment
diesel migration run --database-url postgresql://user:pass@host:port/soar
```

### Installation

Install the required Python dependencies:

```bash
pip install psycopg[binary]
```

Or if using a virtual environment:

```bash
python -m pip install psycopg[binary]
```

## Backup and Restore Scripts

The `backup/` directory contains scripts for database backup and restoration. The system uses `pg_dump` for logical backups, stored in Wasabi S3 via rclone.

### Overview

| Script | Purpose |
|--------|---------|
| `backup/base-backup` | Creates full database backups using pg_dump |
| `backup/restore` | Restores database from pg_dump backup |
| `backup/backup-verify` | Verifies backup integrity |
| `restore-backup` | Interactive TUI for backup selection and restoration |

### TimescaleDB Support

The database uses TimescaleDB hypertables (`fixes`, `raw_messages`) with compression enabled. The backup and restore scripts properly handle:

- Compressed hypertables (chunks older than 7 days are automatically compressed)
- TimescaleDB internal tables (`_timescaledb_internal.*`)
- Pre/post restore functions required for hypertable restoration

**Note:** During backup, you may see NOTICE messages like:
```
pg_dump: NOTICE: hypertable data are in the chunks, no data will be copied
```
These are **expected and harmless** - pg_dump correctly handles compressed hypertables.

---

### Required Configuration Files

Before running backups, you need these files in place:

#### 1. `/etc/soar/backup-env` - Backup Configuration

```bash
# Copy the example and edit
sudo mkdir -p /etc/soar
sudo cp infrastructure/examples/backup-env.example /etc/soar/backup-env
sudo nano /etc/soar/backup-env
sudo chown postgres:postgres /etc/soar/backup-env
sudo chmod 600 /etc/soar/backup-env
```

Key settings:
```bash
# Wasabi S3 configuration
BACKUP_RCLONE_REMOTE=wasabi
BACKUP_RCLONE_BUCKET=soar-backup-prod
BACKUP_RCLONE_PATH=              # Optional prefix within bucket

# Rclone config location
RCLONE_CONFIG=/etc/soar/rclone.conf

# PostgreSQL connection
PGHOST=localhost
PGPORT=5432
PGDATABASE=soar                  # or soar_staging for staging
PGUSER=postgres

# Retention
BASE_BACKUP_KEEP_COUNT=5         # Keep 5 most recent backups

# Temp storage (needs ~150GB free)
BACKUP_TEMP_DIR=/storage/soar/backups/base
```

#### 2. `/etc/soar/rclone.conf` - Wasabi S3 Credentials

```bash
sudo nano /etc/soar/rclone.conf
sudo chown postgres:soar /etc/soar/rclone.conf
sudo chmod 640 /etc/soar/rclone.conf
```

Contents:
```ini
[wasabi]
type = s3
provider = Wasabi
access_key_id = YOUR_ACCESS_KEY
secret_access_key = YOUR_SECRET_KEY
endpoint = s3.wasabisys.com
acl = private
```

#### 3. Verify Configuration

```bash
# Test rclone access
source /etc/soar/backup-env
rclone --config ${RCLONE_CONFIG} lsd ${BACKUP_RCLONE_REMOTE}:${BACKUP_RCLONE_BUCKET}
```

---

### How Backups are Normally Invoked (Production)

Backups run automatically via systemd timers on the production server.

#### Systemd Units

| Unit | Schedule | Purpose |
|------|----------|---------|
| `soar-backup-base.timer` | Weekly (Sunday 00:00 UTC) | Full database backup |
| `soar-backup-verify.timer` | Weekly (Monday 03:00 UTC) | Verify backup integrity |

#### Installation (Production Server)

```bash
# Copy systemd units
sudo cp infrastructure/systemd/soar-backup-base.service /etc/systemd/system/
sudo cp infrastructure/systemd/soar-backup-base.timer /etc/systemd/system/
sudo cp infrastructure/systemd/soar-backup-verify.service /etc/systemd/system/
sudo cp infrastructure/systemd/soar-backup-verify.timer /etc/systemd/system/

# Copy scripts
sudo cp scripts/backup/base-backup /usr/local/bin/soar-base-backup
sudo cp scripts/backup/restore /usr/local/bin/soar-restore
sudo cp scripts/backup/backup-verify /usr/local/bin/soar-backup-verify
sudo chmod +x /usr/local/bin/soar-*

# Enable and start timers
sudo systemctl daemon-reload
sudo systemctl enable soar-backup-base.timer soar-backup-verify.timer
sudo systemctl start soar-backup-base.timer soar-backup-verify.timer
```

#### Check Timer Status

```bash
# View timer schedule
sudo systemctl list-timers soar-backup-*

# View last backup run
sudo journalctl -u soar-backup-base.service --since "1 week ago"

# Manual trigger (for testing)
sudo systemctl start soar-backup-base.service
sudo journalctl -u soar-backup-base.service -f
```

---

### Running Backups Manually

#### Create a Backup (Production)

```bash
# As postgres user (or with sudo)
sudo -u postgres /usr/local/bin/soar-base-backup

# Or run the script directly
sudo -u postgres ./scripts/backup/base-backup

# Skip cleanup of old backups (useful for testing)
sudo -u postgres ./scripts/backup/base-backup --no-cleanup
```

#### Verify Backups

```bash
# Verify all backups
sudo -u postgres /usr/local/bin/soar-backup-verify

# Verify specific backup
sudo -u postgres /usr/local/bin/soar-backup-verify 2025-01-13
```

---

### Running Backups on Staging

The backup scripts normally run on production. To backup the staging database:

#### Option 1: One-Time Manual Backup

```bash
# SSH to staging server (supervillain)
# Set environment to point to staging database
export PGDATABASE=soar_staging

# Create a separate backup-env for staging (or modify temporarily)
cat > /tmp/backup-env-staging << 'EOF'
BACKUP_RCLONE_REMOTE=wasabi
BACKUP_RCLONE_BUCKET=soar-backup-prod
BACKUP_RCLONE_PATH=staging
RCLONE_CONFIG=/etc/soar/rclone.conf
PGHOST=localhost
PGPORT=5432
PGDATABASE=soar_staging
PGUSER=postgres
BACKUP_TEMP_DIR=/storage/soar/backups/base
BASE_BACKUP_KEEP_COUNT=3
EOF

# Run backup with staging config
sudo -u postgres env $(cat /tmp/backup-env-staging | xargs) ./scripts/backup/base-backup
```

#### Option 2: Permanent Staging Backup Setup

```bash
# Create staging-specific config
sudo cp /etc/soar/backup-env /etc/soar/backup-env-staging
sudo nano /etc/soar/backup-env-staging
# Change: PGDATABASE=soar_staging
# Change: BACKUP_RCLONE_PATH=staging
sudo chown postgres:postgres /etc/soar/backup-env-staging
sudo chmod 600 /etc/soar/backup-env-staging

# Run staging backup
sudo -u postgres bash -c 'source /etc/soar/backup-env-staging && ./scripts/backup/base-backup'
```

---

### Copying Staging Database to Production

To backup staging and restore it to production:

#### Step 1: Create Staging Backup

```bash
# On staging server (supervillain)
export PGDATABASE=soar_staging
export BACKUP_RCLONE_PATH=staging-to-prod

# Create backup of staging
sudo -u postgres env PGDATABASE=soar_staging BACKUP_RCLONE_PATH=staging-to-prod \
    bash -c 'source /etc/soar/backup-env && ./scripts/backup/base-backup'

# Note the backup date (e.g., 2025-01-13)
```

#### Step 2: Restore to Production

```bash
# On production server (glider)
# First, verify the backup exists
source /etc/soar/backup-env
rclone --config ${RCLONE_CONFIG} lsd ${BACKUP_RCLONE_REMOTE}:${BACKUP_RCLONE_BUCKET}/staging-to-prod/base

# DANGER: This will DESTROY the production database!
# Restore from staging backup
export BACKUP_RCLONE_PATH=staging-to-prod
./scripts/restore-backup --date 2025-01-13

# Or use the low-level restore script
sudo -u postgres env BACKUP_RCLONE_PATH=staging-to-prod ./scripts/backup/restore --backup 2025-01-13
```

#### Alternative: Direct pg_dump/pg_restore

For a one-off copy without using S3:

```bash
# On staging server, dump directly
pg_dump -h localhost -U postgres -d soar_staging -Fd -j 4 -f /tmp/staging-dump

# Transfer to production (rsync or scp)
rsync -avz /tmp/staging-dump/ glider.flights:/tmp/staging-dump/

# On production server, restore
# WARNING: This will DESTROY production data!
psql -U postgres -d postgres -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = 'soar';"
dropdb -U postgres soar
createdb -U postgres soar
psql -U postgres -d soar -c "CREATE EXTENSION IF NOT EXISTS timescaledb; SELECT timescaledb_pre_restore();"
pg_restore -U postgres -d soar -j 4 /tmp/staging-dump
psql -U postgres -d soar -c "SELECT timescaledb_post_restore();"
```

---

### restore-backup - Interactive Backup Restoration

The `restore-backup` script provides an interactive Terminal User Interface (TUI) for enumerating and restoring database backups from cloud storage (Wasabi S3).

#### Features

- Lists all available backups with timestamps and metadata
- Interactive curses-based menu for backup selection
- Displays backup size, age, and database information
- Integrates with existing `restore` script for automatic restoration
- Supports command-line mode for automation
- Comprehensive error handling and user confirmations

#### Usage

```bash
# Interactive TUI mode (default)
./scripts/restore-backup

# List available backups
./scripts/restore-backup --list

# Restore latest backup (with confirmation)
./scripts/restore-backup --latest

# Restore specific backup by date
./scripts/restore-backup --date 2025-01-10

# Show help
./scripts/restore-backup --help
```

#### Interactive Mode

In interactive mode, the script displays a navigable list of available backups:

- **Up/Down** or **k/j**: Navigate through backups
- **Enter**: Select a backup
- **q** or **Esc**: Cancel and exit

The latest backup is marked with `*` and displayed in green.

#### Security Warnings

**This is a DESTRUCTIVE operation**. The restore process will:

1. DROP the target database
2. Recreate the database
3. Run TimescaleDB pre-restore preparation
4. Restore from pg_dump backup
5. Run TimescaleDB post-restore finalization

Expected downtime: 30-60 minutes (depends on database size)

The script requires explicit confirmation before proceeding with restoration.

#### Examples

```bash
# Interactive selection (safest)
./scripts/restore-backup

# Quick restore of latest backup
./scripts/restore-backup --latest

# Restore specific backup
./scripts/restore-backup --date 2025-01-05

# List backups with metadata
./scripts/restore-backup --list

# Use custom config file
./scripts/restore-backup --config /path/to/backup-env

# Restore from a different path (e.g., staging backup)
BACKUP_RCLONE_PATH=staging ./scripts/restore-backup --list
```

#### Exit Codes

- `0`: Success
- `1`: Failure
- `2`: User cancelled

---

### Backup Storage Structure

Backups are stored in Wasabi S3 with this structure:

```
s3://soar-backup-prod/
├── base/                          # Production backups
│   ├── 2025-01-13/
│   │   ├── database/              # pg_dump directory format
│   │   │   ├── toc.dat
│   │   │   ├── *.dat.zst          # Compressed table data
│   │   │   └── ...
│   │   └── backup-metadata.json
│   ├── 2025-01-06/
│   └── ...
├── staging/                       # Staging backups (if configured)
│   └── base/
│       └── 2025-01-13/
└── wal/                           # WAL archives (for PITR)
    └── ...
```

---

### Troubleshooting

#### Backup Fails with Permission Error

```bash
# Ensure postgres user owns the temp directory
sudo chown -R postgres:postgres /storage/soar/backups
sudo chmod 750 /storage/soar/backups
```

#### rclone Cannot Connect to Wasabi

```bash
# Test connectivity
rclone --config /etc/soar/rclone.conf lsd wasabi:soar-backup-prod

# Check credentials
cat /etc/soar/rclone.conf
```

#### Restore Fails with TimescaleDB Errors

The restore script automatically runs `timescaledb_pre_restore()` and `timescaledb_post_restore()`. If you see errors:

```bash
# Manually run post-restore
psql -U postgres -d soar -c "SELECT timescaledb_post_restore();"

# Check hypertable status
psql -U postgres -d soar -c "SELECT * FROM timescaledb_information.hypertables;"
psql -U postgres -d soar -c "SELECT count(*) FROM timescaledb_information.chunks;"
```

#### Verify Backup Contents

```bash
# List backup files
source /etc/soar/backup-env
rclone --config ${RCLONE_CONFIG} ls ${BACKUP_RCLONE_REMOTE}:${BACKUP_RCLONE_BUCKET}/base/2025-01-13/

# Check backup metadata
rclone --config ${RCLONE_CONFIG} cat ${BACKUP_RCLONE_REMOTE}:${BACKUP_RCLONE_BUCKET}/base/2025-01-13/backup-metadata.json | jq .
```
