# SOAR Database Backup Strategy

This document describes the comprehensive backup strategy for the SOAR PostgreSQL database (420GB production database).

## Table of Contents

- [Overview](#overview)
- [Backup Strategy](#backup-strategy)
- [PostgreSQL Configuration](#postgresql-configuration)
- [Cloud Storage Setup](#cloud-storage-setup)
- [Backup Scripts](#backup-scripts)
- [Restore Procedures](#restore-procedures)
- [Monitoring](#monitoring)
- [Testing](#testing)
- [Cost Estimates](#cost-estimates)
- [Troubleshooting](#troubleshooting)

## Overview

### Current State
- **Database Size**: 420GB
- **Largest Tables**:
  - `aprs_messages`: 216GB
  - `fixes`: 190GB
- **Growth Rate**: ~5-10GB/day from continuous APRS data ingestion
- **Existing System**: Data archival system (moves old data to CSV) - NOT a backup solution

### Backup Requirements
- **Data Loss Tolerance**: Zero - point-in-time recovery (PITR) capability
- **Storage Location**: Cloud storage (S3-compatible)
- **Retention Period**: 30 days
- **Recovery Time Objective (RTO)**: 2-4 hours for full restore
- **Recovery Point Objective (RPO)**: Seconds (continuous WAL archiving)

### Solution Architecture

We implement a **physical backup strategy** using PostgreSQL's continuous archiving:

1. **Base Backups**: Weekly full database backups using `pg_basebackup`
2. **WAL Archiving**: Continuous archiving of Write-Ahead Log (WAL) segments to cloud storage
3. **Point-in-Time Recovery**: Ability to restore to any second within the 30-day retention window

```
┌─────────────────┐
│   PostgreSQL    │
│   Production    │
│    (420GB)      │
└────────┬────────┘
         │
         ├─── Weekly Base Backup ───┐
         │                          │
         └─── Continuous WAL ───────┤
                                    │
                            ┌───────▼────────┐
                            │  Cloud Storage │
                            │   (S3/B2/etc)  │
                            │                │
                            │ Base Backups:  │
                            │  ~600GB        │
                            │ WAL Archives:  │
                            │  ~300GB        │
                            └────────────────┘
```

## Backup Strategy

### Base Backups

**Frequency**: Weekly (every Sunday at 2:00 AM)
**Method**: `pg_basebackup` - creates a physical copy of the database cluster
**Compression**: gzip compression (reduces size ~60-70%)
**Retention**: Keep 4 weekly backups (28 days)
**Location**: Cloud storage bucket under `base/`
**Duration**: ~2-4 hours for 420GB database

### WAL (Write-Ahead Log) Archiving

**Frequency**: Continuous (as WAL segments are filled, typically every ~16MB)
**Method**: PostgreSQL `archive_command` uploads WAL segments to cloud storage
**Retention**: 30 days (automatically pruned by lifecycle policy)
**Location**: Cloud storage bucket under `wal/`
**Size**: ~5-10GB per day (depends on write activity)

### How It Works

1. PostgreSQL writes all database changes to WAL files before applying them to data files
2. When a WAL segment (16MB) is full, PostgreSQL calls our `archive_command`
3. Our script uploads the WAL segment to cloud storage
4. Once a week, we take a full base backup as a starting point
5. To restore: Start with base backup + replay WAL segments to desired point in time

### Why This Approach?

- **Point-in-Time Recovery**: Can restore to any second, not just backup times
- **Zero Data Loss**: Continuous WAL archiving means no gap between backups
- **Efficient**: WAL files are small (16MB), easy to upload continuously
- **Battle-Tested**: Standard PostgreSQL feature used in production worldwide
- **Faster Restore**: Physical backups are faster than logical dumps for large databases

## PostgreSQL Configuration

### Required Configuration Changes

Edit `/etc/postgresql/15/main/postgresql.conf` (adjust version number as needed):

```conf
#------------------------------------------------------------------------------
# WRITE-AHEAD LOG (WAL) CONFIGURATION FOR BACKUPS
#------------------------------------------------------------------------------

# Enable WAL archiving for continuous backup
# This is the minimum level needed for archiving and streaming replication
wal_level = replica

# Enable archive mode - PostgreSQL will call archive_command for each WAL file
archive_mode = on

# Command to archive a WAL file segment
# %p = path of file to archive
# %f = file name only
# This will be called for each 16MB WAL segment
archive_command = '/usr/local/bin/soar-wal-archive %p %f'

# If archive_command fails, PostgreSQL will retry. Set a timeout to prevent
# WAL files from accumulating indefinitely if archiving is broken
archive_timeout = 300

#------------------------------------------------------------------------------
# WAL SIZING AND CHECKPOINTS
#------------------------------------------------------------------------------

# Maximum size to let the WAL grow between automatic checkpoints
# Larger values reduce checkpoint frequency but increase recovery time
# For a 420GB database with high write volume, 4GB is reasonable
max_wal_size = 4GB

# Minimum size to shrink the WAL to
min_wal_size = 1GB

# Time between automatic checkpoints (in seconds)
# 15 minutes is a good balance for most workloads
checkpoint_timeout = 15min

# Fraction of checkpoint completion where writes can start slowing down
# 0.9 means start slowing writes at 90% of checkpoint_timeout
checkpoint_completion_target = 0.9

# Keep extra WAL segments in pg_wal/ to allow fetching by standby servers
# or recovery. For cloud archiving, 10 segments (~160MB) is usually enough
# as a buffer in case archiving gets temporarily delayed
wal_keep_size = 160MB

#------------------------------------------------------------------------------
# LOGGING FOR MONITORING
#------------------------------------------------------------------------------

# Log checkpoints for monitoring and tuning
log_checkpoints = on

# Log completed archive commands (useful for debugging)
# Note: This can be verbose, disable after confirming archiving works
log_archive_command = on
```

### Applying Configuration Changes

1. **Edit the configuration file**:
   ```bash
   sudo nano /etc/postgresql/15/main/postgresql.conf
   ```

2. **Verify configuration syntax**:
   ```bash
   sudo -u postgres /usr/lib/postgresql/15/bin/postgres -C archive_mode -D /var/lib/postgresql/15/main
   ```

3. **Restart PostgreSQL** (requires downtime):
   ```bash
   sudo systemctl restart postgresql
   ```

4. **Verify settings are active**:
   ```bash
   psql -U postgres -d soar -c "SHOW wal_level;"
   psql -U postgres -d soar -c "SHOW archive_mode;"
   psql -U postgres -d soar -c "SHOW archive_command;"
   ```

### Understanding the Configuration

**`wal_level = replica`**
- Writes enough information to WAL to support archiving and replication
- Required for `pg_basebackup` and WAL archiving
- Minimal performance overhead compared to `minimal`

**`archive_mode = on`**
- Enables the archiving system
- PostgreSQL will call `archive_command` for each completed WAL segment

**`archive_command`**
- Shell command executed for each WAL segment
- Must return exit code 0 on success, non-zero on failure
- If command fails, PostgreSQL keeps the WAL file and retries
- **Critical**: If archiving fails consistently, WAL files accumulate in `pg_wal/` and can fill disk

**`max_wal_size = 4GB`**
- PostgreSQL keeps this much WAL before forcing a checkpoint
- Larger values = less frequent checkpoints = better performance
- Trade-off: More WAL to replay during recovery
- For high-write databases like SOAR, 4GB is a good balance

**`checkpoint_timeout = 15min`**
- Maximum time between checkpoints
- Checkpoints flush dirty data to disk
- More frequent = faster recovery but more I/O overhead

**`wal_keep_size = 160MB`**
- Extra WAL kept in `pg_wal/` directory
- Buffer in case archiving gets delayed (network issues, cloud API limits)
- 10 segments × 16MB = 160MB buffer

### Disk Space Monitoring

With WAL archiving enabled, monitor `pg_wal/` directory:

```bash
# Check WAL directory size (should stay under a few GB)
du -sh /var/lib/postgresql/15/main/pg_wal/

# Count WAL files (normal: 10-20, concerning: >100)
ls /var/lib/postgresql/15/main/pg_wal/ | wc -l

# Check if archiving is keeping up
psql -U postgres -d soar -c "SELECT last_archived_wal, last_archived_time FROM pg_stat_archiver;"
```

**Warning Signs**:
- `pg_wal/` directory > 10GB: Archiving is falling behind
- >100 WAL files: Archive command is likely failing
- `last_archived_time` > 5 minutes ago: Investigate archive command

## Cloud Storage Setup

### Choosing a Provider

Our backup scripts support any S3-compatible storage:

| Provider | Cost (per TB/month) | Notes |
|----------|---------------------|-------|
| AWS S3 Standard | $23 | Best integration, highest cost |
| AWS S3 Glacier Instant | $4 | Lower cost, millisecond retrieval |
| Backblaze B2 | $5 | Good balance, S3-compatible API |
| Wasabi | $5.99 | No egress fees, S3-compatible |
| DigitalOcean Spaces | $5 | Simple, includes 250GB egress |

**Recommendation**: **Backblaze B2** for best cost/performance balance.

### Storage Structure

```
s3://your-backup-bucket/
├── base/
│   ├── 2025-01-05/           # Weekly base backup
│   │   ├── backup.tar.gz     # ~150GB compressed
│   │   ├── backup_label      # Backup metadata
│   │   └── tablespace_map    # Tablespace info
│   ├── 2025-01-12/
│   ├── 2025-01-19/
│   └── 2025-01-26/
├── wal/
│   ├── 000000010000000000000001
│   ├── 000000010000000000000002
│   ├── ... (thousands of 16MB files)
│   └── 00000001000000000000FFFF
└── config/
    └── backup-config.json    # Backup metadata
```

### AWS S3 Setup

1. **Create IAM user for backups**:
   ```bash
   aws iam create-user --user-name soar-backup
   ```

2. **Create S3 bucket**:
   ```bash
   aws s3 mb s3://soar-backup-prod --region us-east-1
   ```

3. **Enable versioning** (protects against accidental deletion):
   ```bash
   aws s3api put-bucket-versioning \
     --bucket soar-backup-prod \
     --versioning-configuration Status=Enabled
   ```

4. **Set lifecycle policy** (auto-delete old backups):
   ```bash
   cat > lifecycle-policy.json <<'EOF'
   {
     "Rules": [
       {
         "Id": "delete-old-wal",
         "Status": "Enabled",
         "Prefix": "wal/",
         "Expiration": {
           "Days": 30
         }
       },
       {
         "Id": "delete-old-base",
         "Status": "Enabled",
         "Prefix": "base/",
         "Expiration": {
           "Days": 30
         }
       }
     ]
   }
   EOF

   aws s3api put-bucket-lifecycle-configuration \
     --bucket soar-backup-prod \
     --lifecycle-configuration file://lifecycle-policy.json
   ```

5. **Create IAM policy**:
   ```bash
   cat > backup-policy.json <<'EOF'
   {
     "Version": "2012-10-17",
     "Statement": [
       {
         "Effect": "Allow",
         "Action": [
           "s3:PutObject",
           "s3:GetObject",
           "s3:ListBucket",
           "s3:DeleteObject"
         ],
         "Resource": [
           "arn:aws:s3:::soar-backup-prod",
           "arn:aws:s3:::soar-backup-prod/*"
         ]
       }
     ]
   }
   EOF

   aws iam put-user-policy \
     --user-name soar-backup \
     --policy-name soar-backup-policy \
     --policy-document file://backup-policy.json
   ```

6. **Create access key**:
   ```bash
   aws iam create-access-key --user-name soar-backup
   ```

   Save the `AccessKeyId` and `SecretAccessKey` - you'll need these for configuration.

### Backblaze B2 Setup

1. **Create account** at https://www.backblaze.com/b2/sign-up.html

2. **Create bucket**:
   - Log into B2 console
   - Create bucket: `soar-backup-prod`
   - Set to "Private"
   - Enable "Lifecycle Settings" → Delete old versions after 30 days

3. **Create application key**:
   - Go to "App Keys" in B2 console
   - Create new key with access to `soar-backup-prod` bucket
   - Save the `keyID` and `applicationKey`

4. **Install B2 CLI** (optional, for testing):
   ```bash
   pip install b2
   b2 authorize-account <keyID> <applicationKey>
   b2 ls soar-backup-prod
   ```

### Local Configuration

Create `/etc/soar/backup-env` (restrict permissions!):

```bash
# Cloud storage credentials
# Use either AWS S3 or S3-compatible endpoint (Backblaze B2, Wasabi, etc.)

# For AWS S3
export AWS_ACCESS_KEY_ID="AKIA..."
export AWS_SECRET_ACCESS_KEY="..."
export AWS_DEFAULT_REGION="us-east-1"
export BACKUP_BUCKET="s3://soar-backup-prod"

# For Backblaze B2 (S3-compatible)
# export AWS_ACCESS_KEY_ID="<keyID>"
# export AWS_SECRET_ACCESS_KEY="<applicationKey>"
# export AWS_ENDPOINT_URL="https://s3.us-west-004.backblazeb2.com"
# export BACKUP_BUCKET="s3://soar-backup-prod"

# For Wasabi
# export AWS_ACCESS_KEY_ID="..."
# export AWS_SECRET_ACCESS_KEY="..."
# export AWS_ENDPOINT_URL="https://s3.wasabisys.com"
# export BACKUP_BUCKET="s3://soar-backup-prod"

# Backup retention
export BACKUP_RETENTION_DAYS=30

# PostgreSQL connection (usually local)
export PGHOST=localhost
export PGPORT=5432
export PGDATABASE=soar
export PGUSER=postgres
# Note: Use .pgpass for password, not environment variable

# Backup paths
export BACKUP_TEMP_DIR="/var/lib/soar/backup-temp"
export BACKUP_LOG_DIR="/var/log/soar"

# Notification (optional)
# export BACKUP_NOTIFY_EMAIL="ops@example.com"
# export BACKUP_NOTIFY_SLACK_WEBHOOK="https://hooks.slack.com/..."
```

**Set restrictive permissions**:
```bash
sudo chown postgres:postgres /etc/soar/backup-env
sudo chmod 600 /etc/soar/backup-env
```

### Testing Cloud Access

```bash
# Source credentials
source /etc/soar/backup-env

# Test upload
echo "test" | aws s3 cp - ${BACKUP_BUCKET}/test.txt

# Test download
aws s3 cp ${BACKUP_BUCKET}/test.txt -

# Test listing
aws s3 ls ${BACKUP_BUCKET}/

# Cleanup
aws s3 rm ${BACKUP_BUCKET}/test.txt
```

## Backup Scripts

All scripts are located in `/scripts/backup/` and must be executable (`chmod +x`).

### 1. WAL Archive Script: `wal-archive`

**Purpose**: Called by PostgreSQL to archive WAL segments continuously.

**Location**: `/usr/local/bin/soar-wal-archive` (symlink to `/scripts/backup/wal-archive`)

**Usage**: Called automatically by PostgreSQL via `archive_command`.

**What it does**:
1. Receives WAL file path and name from PostgreSQL
2. Uploads to cloud storage under `wal/` prefix
3. Verifies upload succeeded
4. Returns exit code 0 on success, non-zero on failure

**Manual testing**:
```bash
# Test archiving a WAL file (must be run as postgres user)
sudo -u postgres /usr/local/bin/soar-wal-archive \
  /var/lib/postgresql/15/main/pg_wal/000000010000000000000001 \
  000000010000000000000001
```

### 2. Base Backup Script: `base-backup`

**Purpose**: Creates weekly full database backup.

**Location**: `/scripts/backup/base-backup`

**Scheduled**: Weekly via systemd timer (Sunday 2:00 AM)

**What it does**:
1. Creates base backup using `pg_basebackup`
2. Compresses backup with gzip
3. Uploads to cloud storage under `base/<date>/`
4. Verifies backup integrity
5. Cleans up old backups (keeps 4 most recent)
6. Logs results and sends notifications

**Manual usage**:
```bash
# Run as postgres user
sudo -u postgres /scripts/backup/base-backup

# Or via systemd
sudo systemctl start soar-backup-base.service
```

**Options** (set in `/etc/soar/backup-env`):
- `BACKUP_COMPRESSION=gzip` - Compression method (gzip or none)
- `BACKUP_PARALLEL_JOBS=4` - Number of parallel compression threads
- `BACKUP_TEMP_DIR` - Local staging directory

### 3. Backup Verification Script: `backup-verify`

**Purpose**: Verifies backup integrity weekly.

**Location**: `/scripts/backup/backup-verify`

**Scheduled**: Weekly via systemd timer (Monday 3:00 AM)

**What it does**:
1. Lists all base backups in cloud storage
2. Downloads backup manifest files
3. Verifies file checksums
4. Checks WAL continuity
5. Reports any issues

**Manual usage**:
```bash
sudo -u postgres /scripts/backup/backup-verify

# Verify specific backup
sudo -u postgres /scripts/backup/backup-verify 2025-01-05
```

### 4. Restore Script: `restore`

**Purpose**: Restore database from backup (full disaster recovery or PITR).

**Location**: `/scripts/backup/restore`

**Usage**: **ONLY run during disaster recovery - this is destructive!**

**What it does**:
1. Stops PostgreSQL
2. Downloads base backup from cloud
3. Extracts to PostgreSQL data directory
4. Configures recovery settings
5. Starts PostgreSQL in recovery mode
6. Replays WAL segments to target time
7. Promotes to normal operation

**Manual usage**:
```bash
# Full restore to latest available point
sudo /scripts/backup/restore --latest

# Point-in-time restore to specific timestamp
sudo /scripts/backup/restore --target-time "2025-01-10 14:30:00"

# Restore from specific base backup
sudo /scripts/backup/restore --base-backup 2025-01-05 --target-time "2025-01-08 10:00:00"
```

**Safety features**:
- Requires confirmation before proceeding
- Creates snapshot of current data directory
- Validates backup before destroying current data
- Logs all operations

## Restore Procedures

### Pre-Requisites

Before attempting any restore:

1. **Verify backups exist**:
   ```bash
   source /etc/soar/backup-env
   aws s3 ls ${BACKUP_BUCKET}/base/
   aws s3 ls ${BACKUP_BUCKET}/wal/ | head -20
   ```

2. **Check backup age**:
   ```bash
   /scripts/backup/backup-verify
   ```

3. **Estimate restore time**:
   - Base backup download: ~30-60 minutes (150GB @ 50-100 MB/s)
   - Extraction: ~20-30 minutes
   - WAL replay: ~10-60 minutes (depends on amount of WAL)
   - **Total**: 2-4 hours typical

4. **Ensure sufficient disk space**:
   - Need 2× database size during restore (~850GB free)
   - Temporary space for downloaded backup (~150GB)

### Scenario 1: Full Disaster Recovery (Latest Point)

**When**: Complete database loss (hardware failure, corruption, accidental deletion).

**Goal**: Restore to the most recent possible state.

**Steps**:

1. **Stop application services**:
   ```bash
   sudo systemctl stop soar-aprs soar-api soar-archive
   ```

2. **Run restore script**:
   ```bash
   sudo /scripts/backup/restore --latest
   ```

3. **Verify restoration**:
   ```bash
   sudo systemctl start postgresql

   # Check database size
   psql -U postgres -d soar -c "SELECT pg_size_pretty(pg_database_size('soar'));"

   # Check latest data
   psql -U postgres -d soar -c "SELECT MAX(created_at) FROM fixes;"
   psql -U postgres -d soar -c "SELECT COUNT(*) FROM devices;"
   ```

4. **Restart application services**:
   ```bash
   sudo systemctl start soar-aprs soar-api soar-archive
   ```

5. **Monitor logs**:
   ```bash
   sudo journalctl -u soar-aprs -f
   sudo tail -f /var/log/soar/backup.log
   ```

### Scenario 2: Point-in-Time Recovery (Undo Bad Change)

**When**: Accidental data deletion, bad migration, or need to recover to specific time.

**Goal**: Restore database to exact state at specific timestamp.

**Example**: Someone accidentally deleted all devices on Jan 10 at 2:30 PM. Restore to 2:25 PM.

**Steps**:

1. **Determine target time**:
   ```bash
   # Find out when the problem occurred
   # Check application logs, user reports, etc.
   TARGET_TIME="2025-01-10 14:25:00"
   ```

2. **Stop application services**:
   ```bash
   sudo systemctl stop soar-aprs soar-api soar-archive
   ```

3. **Run restore with target time**:
   ```bash
   sudo /scripts/backup/restore --target-time "2025-01-10 14:25:00"
   ```

4. **Verify restored state**:
   ```bash
   sudo systemctl start postgresql

   # Check data is correct (before the deletion)
   psql -U postgres -d soar -c "SELECT COUNT(*) FROM devices;"

   # Verify timestamp
   psql -U postgres -d soar -c "SELECT now(), MAX(created_at) FROM fixes;"
   ```

5. **Restart services**:
   ```bash
   sudo systemctl start soar-aprs soar-api soar-archive
   ```

### Scenario 3: Restore to Development Database (Testing)

**When**: Testing restore process, creating dev environment, investigating historical data.

**Goal**: Restore backup to non-production database without affecting production.

**Steps**:

1. **Create target database**:
   ```bash
   sudo -u postgres createdb soar_restore_test
   ```

2. **Run restore to alternate location**:
   ```bash
   sudo /scripts/backup/restore \
     --target-time "2025-01-10 14:25:00" \
     --target-database soar_restore_test \
     --no-destructive
   ```

3. **Query restored data**:
   ```bash
   psql -U postgres -d soar_restore_test -c "SELECT COUNT(*) FROM devices;"
   ```

4. **Cleanup when done**:
   ```bash
   sudo -u postgres dropdb soar_restore_test
   ```

### Scenario 4: Partial Restore (Single Table)

**When**: Need to recover specific table without full database restore.

**Goal**: Extract single table from backup and restore it.

**Steps**:

1. **Download and extract base backup to temporary location**:
   ```bash
   source /etc/soar/backup-env
   mkdir -p /tmp/restore
   cd /tmp/restore

   # Find latest backup
   LATEST_BACKUP=$(aws s3 ls ${BACKUP_BUCKET}/base/ | tail -1 | awk '{print $2}' | tr -d '/')

   # Download
   aws s3 cp ${BACKUP_BUCKET}/base/${LATEST_BACKUP}/backup.tar.gz .
   tar xzf backup.tar.gz
   ```

2. **Start temporary PostgreSQL instance**:
   ```bash
   # Use different port to avoid conflict
   /usr/lib/postgresql/15/bin/pg_ctl \
     -D /tmp/restore/data \
     -o "-p 5433" \
     start
   ```

3. **Export specific table**:
   ```bash
   pg_dump -h localhost -p 5433 -U postgres -d soar \
     --table=devices \
     --data-only \
     > devices_restore.sql
   ```

4. **Import to production** (be careful!):
   ```bash
   # Review the SQL first!
   less devices_restore.sql

   # Import
   psql -U postgres -d soar -f devices_restore.sql
   ```

5. **Cleanup**:
   ```bash
   /usr/lib/postgresql/15/bin/pg_ctl -D /tmp/restore/data stop
   rm -rf /tmp/restore
   ```

### Recovery Testing

**Test restore quarterly** to ensure backups work:

1. **Schedule test restore** (2 hours downtime):
   ```bash
   # Create calendar reminder for first Sunday of quarter
   ```

2. **Document test**:
   - Date of test
   - Backup tested (which base backup + WAL range)
   - Restore duration
   - Any issues encountered
   - Verification queries passed

3. **Update RTO/RPO** based on actual test results.

## Monitoring

### Backup Health Metrics

Add these metrics to existing Prometheus/Grafana setup:

```rust
// In src/metrics.rs or appropriate metrics module

// Time since last successful base backup (should be < 7 days)
gauge!("backup.base.age_seconds", last_base_backup_age());

// Time since last WAL archive (should be < 5 minutes)
gauge!("backup.wal.age_seconds", last_wal_archive_age());

// WAL archive queue depth (should be < 10)
gauge!("backup.wal.queue_depth", wal_queue_depth());

// Base backup duration (for capacity planning)
histogram!("backup.base.duration_seconds", base_backup_duration);

// WAL archive errors (should be 0)
counter!("backup.wal.errors_total", wal_archive_errors);

// Base backup size (for cost tracking)
gauge!("backup.base.size_bytes", base_backup_size);
```

### PostgreSQL Monitoring Queries

```sql
-- Check archiver status
SELECT
  last_archived_wal,
  last_archived_time,
  last_failed_wal,
  last_failed_time,
  stats_reset
FROM pg_stat_archiver;

-- Check WAL generation rate (bytes per hour)
SELECT
  (pg_current_wal_lsn() - '0/0') /
  EXTRACT(EPOCH FROM (now() - pg_postmaster_start_time())) * 3600
  AS wal_bytes_per_hour;

-- Check replication slots (if using)
SELECT slot_name, active, restart_lsn FROM pg_replication_slots;

-- Check WAL disk usage
SELECT
  setting AS data_directory,
  pg_size_pretty(sum(size)) AS wal_size
FROM pg_settings,
  LATERAL pg_ls_waldir() AS wal(name, size, modification)
WHERE name = 'data_directory'
GROUP BY setting;
```

### Alert Rules

Configure alerts for:

| Metric | Warning Threshold | Critical Threshold | Action |
|--------|------------------|-------------------|---------|
| Last base backup age | > 8 days | > 10 days | Check systemd timer, investigate failures |
| Last WAL archive age | > 5 minutes | > 15 minutes | Check archive_command, cloud connectivity |
| WAL queue depth | > 20 files | > 50 files | Archive command failing, check logs |
| Archive failure count | > 5/hour | > 20/hour | Cloud API issues, check credentials |
| pg_wal directory size | > 5GB | > 20GB | Archive falling behind, disk fill risk |
| Backup verification failed | N/A | Any failure | Backup corruption, re-run base backup |

### Log Monitoring

**Backup logs**: `/var/log/soar/backup.log`

**PostgreSQL logs**: `/var/log/postgresql/postgresql-15-main.log`

**Important patterns to watch for**:

```bash
# Archive command failures
grep "archive command failed" /var/log/postgresql/postgresql-15-main.log

# Successful archives (should see continuous activity)
grep "archived write-ahead log file" /var/log/postgresql/postgresql-15-main.log | tail

# Base backup completion
grep "backup completed" /var/log/soar/backup.log

# Verification failures
grep "verification failed" /var/log/soar/backup.log
```

### Grafana Dashboard

Create dashboard with panels for:

1. **Backup Age**: Time series showing age of last base backup and last WAL archive
2. **WAL Archive Rate**: Archives per minute
3. **Backup Size Trend**: Base backup size over time
4. **Archive Queue Depth**: Number of WAL files waiting to be archived
5. **Error Rate**: Archive errors per hour
6. **Disk Usage**: `pg_wal` directory size
7. **Cloud Upload Bandwidth**: Bytes uploaded per minute

## Testing

### Initial Setup Testing

After installing backup system, test each component:

#### 1. Test WAL Archiving

```bash
# Generate some WAL activity
psql -U postgres -d soar <<EOF
CREATE TABLE backup_test (id SERIAL, data TEXT);
INSERT INTO backup_test (data)
  SELECT repeat('X', 1000) FROM generate_series(1, 100000);
DROP TABLE backup_test;
CHECKPOINT;
EOF

# Check if WAL files were archived
source /etc/soar/backup-env
aws s3 ls ${BACKUP_BUCKET}/wal/ | tail -20

# Check PostgreSQL archiver status
psql -U postgres -d soar -c "SELECT * FROM pg_stat_archiver;"
```

**Expected**: Should see recent WAL files in cloud storage and `last_archived_time` within last few minutes.

#### 2. Test Base Backup

```bash
# Run base backup manually
sudo systemctl start soar-backup-base.service

# Watch progress
sudo journalctl -u soar-backup-base.service -f

# Check results in cloud
source /etc/soar/backup-env
aws s3 ls ${BACKUP_BUCKET}/base/ --recursive --human-readable
```

**Expected**: Should complete in 2-4 hours and show ~150GB compressed backup in cloud.

#### 3. Test Restore (to dev database)

```bash
# Create test database
sudo -u postgres createdb soar_restore_test

# Restore
sudo /scripts/backup/restore \
  --target-database soar_restore_test \
  --no-destructive \
  --latest

# Verify
psql -U postgres -d soar_restore_test -c "\dt"
psql -U postgres -d soar_restore_test -c "SELECT COUNT(*) FROM devices;"

# Cleanup
sudo -u postgres dropdb soar_restore_test
```

**Expected**: Should restore successfully and show same data as production.

### Quarterly Restore Testing

**Schedule**: First Sunday of each quarter (March, June, September, December)

**Procedure**:

1. **Pre-test checks**:
   ```bash
   # Verify backup health
   /scripts/backup/backup-verify

   # Document current database state
   psql -U postgres -d soar -c "
     SELECT
       pg_size_pretty(pg_database_size('soar')) AS db_size,
       (SELECT COUNT(*) FROM devices) AS device_count,
       (SELECT COUNT(*) FROM flights) AS flight_count,
       (SELECT MAX(created_at) FROM fixes) AS latest_fix;
   " > /tmp/pre-restore-state.txt
   ```

2. **Announce maintenance window** (2-4 hours)

3. **Perform restore test to staging database**:
   ```bash
   sudo -u postgres createdb soar_restore_test

   # Time the restore
   time sudo /scripts/backup/restore \
     --target-database soar_restore_test \
     --no-destructive \
     --latest
   ```

4. **Verify restored data**:
   ```bash
   # Compare to production state
   psql -U postgres -d soar_restore_test -c "
     SELECT
       pg_size_pretty(pg_database_size('soar_restore_test')) AS db_size,
       (SELECT COUNT(*) FROM devices) AS device_count,
       (SELECT COUNT(*) FROM flights) AS flight_count,
       (SELECT MAX(created_at) FROM fixes) AS latest_fix;
   " > /tmp/post-restore-state.txt

   diff /tmp/pre-restore-state.txt /tmp/post-restore-state.txt
   ```

5. **Test application functionality**:
   ```bash
   # Update connection string to test database
   PGDATABASE=soar_restore_test cargo test --test integration_tests

   # Manual smoke tests
   # - Can you log in?
   # - Can you view devices?
   # - Can you see flights?
   # - Does map work?
   ```

6. **Document results**:
   - Restore duration: _____ hours
   - Backup age: _____ days old
   - Data integrity: ✓ Pass / ✗ Fail
   - Issues encountered: _____
   - RTO estimate: _____ hours
   - RPO verified: _____ seconds

7. **Cleanup**:
   ```bash
   sudo -u postgres dropdb soar_restore_test
   ```

8. **Update documentation** if any issues found.

## Cost Estimates

### Storage Costs (Monthly)

Based on 420GB database, 30-day retention:

| Component | Size | AWS S3 | Backblaze B2 | Wasabi |
|-----------|------|---------|--------------|---------|
| Base backups (4 weekly) | ~600GB | $14 | $3 | $3.60 |
| WAL archives (30 days) | ~300GB | $7 | $1.50 | $1.80 |
| **Total Storage** | ~900GB | **$21/mo** | **$4.50/mo** | **$5.40/mo** |

### Transfer Costs

| Operation | Monthly Volume | AWS S3 | Backblaze B2 | Wasabi |
|-----------|---------------|---------|--------------|---------|
| Upload (WAL + base) | ~400GB | Free | Free | Free |
| Download (restore test) | ~150GB/quarter | $13.50 | Free* | Free |
| API Requests | ~500K/month | $0.25 | $0.04 | $0.01 |

*Backblaze B2: First 3× data stored is free download (3 × 900GB = 2.7TB/month free)

### Annual Cost Estimate

| Provider | Storage | Transfer | API | **Total/year** |
|----------|---------|----------|-----|----------------|
| AWS S3 | $252 | $54 | $3 | **~$309/year** |
| Backblaze B2 | $54 | $0 | $0.48 | **~$55/year** |
| Wasabi | $65 | $0 | $0.12 | **~$65/year** |

**Recommendation**: Backblaze B2 for ~$55/year is excellent value.

### Cost Optimization

1. **Compress older backups**: After 7 days, re-compress with higher compression (gzip -9 → zstd)
2. **Thin WAL archives**: Keep hourly WAL checkpoints instead of all segments after 7 days
3. **Tiered storage**: Move base backups > 14 days old to cheaper storage tier
4. **Monitor growth**: If APRS data grows faster, adjust retention

## Troubleshooting

### Issue: Archive Command Failing

**Symptoms**:
- `pg_wal/` directory growing large (>5GB)
- Many WAL files accumulating
- Logs show "archive command failed"

**Diagnosis**:
```bash
# Check archiver status
psql -U postgres -d soar -c "SELECT * FROM pg_stat_archiver;"

# Check last failure
grep "archive command failed" /var/log/postgresql/postgresql-15-main.log | tail -5

# Test archive command manually
sudo -u postgres /usr/local/bin/soar-wal-archive \
  /var/lib/postgresql/15/main/pg_wal/000000010000000000000001 \
  000000010000000000000001
```

**Common causes**:

1. **Cloud credentials expired/invalid**:
   ```bash
   # Test AWS credentials
   source /etc/soar/backup-env
   aws sts get-caller-identity
   ```

   **Fix**: Update credentials in `/etc/soar/backup-env`

2. **Network connectivity issues**:
   ```bash
   # Test connectivity to S3
   curl -I https://s3.amazonaws.com
   ```

   **Fix**: Check firewall, proxy settings

3. **Insufficient permissions**:
   ```bash
   # Test bucket access
   aws s3 ls ${BACKUP_BUCKET}/wal/
   ```

   **Fix**: Update IAM policy to allow PutObject

4. **Disk full on destination**:
   ```bash
   aws s3api list-buckets
   ```

   **Fix**: Verify lifecycle policy is working, manually clean old files

**Emergency mitigation** (if disk filling up):
```bash
# Temporarily disable archiving to prevent disk fill
sudo -u postgres psql -d soar -c "ALTER SYSTEM SET archive_command = '/bin/true';"
sudo systemctl reload postgresql

# Clean up old WAL files (DANGEROUS - only if archiving is broken)
# This will lose PITR capability!
sudo -u postgres pg_archivecleanup /var/lib/postgresql/15/main/pg_wal 000000010000000000000064

# Fix archiving issue, then re-enable
sudo -u postgres psql -d soar -c "ALTER SYSTEM SET archive_command = '/usr/local/bin/soar-wal-archive %p %f';"
sudo systemctl reload postgresql
```

### Issue: Base Backup Not Running

**Symptoms**:
- Last base backup > 7 days old
- Alerts firing for backup age

**Diagnosis**:
```bash
# Check systemd timer status
sudo systemctl status soar-backup-base.timer
sudo systemctl list-timers | grep backup

# Check last run
sudo journalctl -u soar-backup-base.service -n 100

# Try manual run
sudo systemctl start soar-backup-base.service
sudo journalctl -u soar-backup-base.service -f
```

**Common causes**:

1. **Timer not enabled**:
   ```bash
   sudo systemctl enable soar-backup-base.timer
   sudo systemctl start soar-backup-base.timer
   ```

2. **Script has errors**:
   ```bash
   sudo -u postgres /scripts/backup/base-backup
   ```

3. **Insufficient disk space**:
   ```bash
   df -h /var/lib/soar/backup-temp
   ```

   **Fix**: Clean temp directory, increase disk space

4. **PostgreSQL not accepting connections**:
   ```bash
   sudo -u postgres psql -d soar -c "SELECT 1;"
   ```

### Issue: Restore Failing

**Symptoms**:
- Restore script exits with error
- Database won't start after restore

**Diagnosis**:
```bash
# Check restore logs
sudo tail -100 /var/log/soar/backup.log

# Check PostgreSQL logs
sudo tail -100 /var/log/postgresql/postgresql-15-main.log

# Check disk space
df -h /var/lib/postgresql
```

**Common causes**:

1. **Insufficient disk space**:
   - Need 2× database size during restore

   **Fix**: Free up space, use external disk

2. **Corrupted backup**:
   ```bash
   # Verify backup integrity
   /scripts/backup/backup-verify
   ```

   **Fix**: Use older backup, re-run base backup

3. **Missing WAL files**:
   - Gap in WAL sequence

   **Fix**: Restore to earlier point before gap

4. **PostgreSQL version mismatch**:
   - Backup from different PostgreSQL version

   **Fix**: Use matching PostgreSQL version

5. **Incorrect recovery settings**:
   - Check `postgresql.auto.conf`

   **Fix**: Review restore script recovery settings

### Issue: High Cloud Storage Costs

**Symptoms**:
- Cloud bill higher than expected
- Storage growing faster than database

**Diagnosis**:
```bash
# Check actual storage usage
aws s3 ls ${BACKUP_BUCKET}/ --recursive --summarize --human-readable

# Check lifecycle policies
aws s3api get-bucket-lifecycle-configuration --bucket soar-backup-prod

# Find large files
aws s3 ls ${BACKUP_BUCKET}/ --recursive --human-readable | sort -k3 -h | tail -20
```

**Common causes**:

1. **Lifecycle policy not working**:
   - Old backups not being deleted

   **Fix**: Re-apply lifecycle policy, manually delete old files

2. **Versioning enabled + many updates**:
   - Multiple versions of same file

   **Fix**: Configure version expiration in lifecycle policy

3. **WAL files not expiring**:
   ```bash
   aws s3 ls ${BACKUP_BUCKET}/wal/ | head -20
   # Check oldest WAL file date
   ```

   **Fix**: Set shorter retention in lifecycle policy

4. **Multiple failed backups**:
   - Partial backups not cleaned up

   **Fix**: Review backup script cleanup logic

### Issue: Cannot Connect During Restore

**Symptoms**:
- Applications can't connect to database
- Getting "database does not exist" errors

**Cause**: Database is still in recovery mode, replaying WAL files.

**Fix**: Wait for recovery to complete. Monitor:
```bash
# Check if recovery is in progress
psql -U postgres -d postgres -c "SELECT pg_is_in_recovery();"

# Check recovery progress (approximate)
tail -f /var/log/postgresql/postgresql-15-main.log | grep recovery
```

Recovery can take 15-60 minutes depending on amount of WAL to replay.

## Additional Resources

- **PostgreSQL Backup Documentation**: https://www.postgresql.org/docs/current/backup.html
- **WAL Archiving**: https://www.postgresql.org/docs/current/continuous-archiving.html
- **pg_basebackup**: https://www.postgresql.org/docs/current/app-pgbasebackup.html
- **Point-in-Time Recovery**: https://www.postgresql.org/docs/current/continuous-archiving.html#BACKUP-PITR-RECOVERY
- **AWS S3 Lifecycle Policies**: https://docs.aws.amazon.com/AmazonS3/latest/userguide/object-lifecycle-mgmt.html
- **Backblaze B2 Documentation**: https://www.backblaze.com/b2/docs/

## Maintenance Schedule

| Task | Frequency | Time | Systemd Timer |
|------|-----------|------|---------------|
| WAL archiving | Continuous | - | (automatic) |
| Base backup | Weekly | Sunday 2:00 AM | `soar-backup-base.timer` |
| Backup verification | Weekly | Monday 3:00 AM | `soar-backup-verify.timer` |
| Restore testing | Quarterly | First Sunday | (manual) |
| Cost review | Monthly | - | (manual) |
| Documentation update | Quarterly | - | (manual) |

## Security Considerations

1. **Credential Protection**:
   - Store credentials in `/etc/soar/backup-env` with 600 permissions
   - Use IAM roles instead of access keys when possible
   - Rotate access keys quarterly

2. **Backup Encryption**:
   - Enable S3 server-side encryption (SSE-S3 or SSE-KMS)
   - Consider client-side encryption for highly sensitive data

3. **Access Control**:
   - Limit backup bucket access to backup user only
   - Enable MFA delete on S3 bucket
   - Monitor access logs for unauthorized access

4. **Network Security**:
   - Use HTTPS for all cloud transfers (enforced by default)
   - Consider VPN or AWS PrivateLink for extra security

## Change Log

| Date | Version | Changes |
|------|---------|---------|
| 2025-01-10 | 1.0 | Initial documentation |

---

**Questions or issues?** Check the troubleshooting section or contact the ops team.
