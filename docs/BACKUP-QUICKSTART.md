# Database Backup Quick Start Guide

This guide helps you set up the SOAR database backup system in 15 minutes.

For comprehensive documentation, see [DB-BACKUPS.md](./DB-BACKUPS.md).

## Prerequisites

- PostgreSQL 17 with 420GB `soar` database
- rclone installed (`apt install rclone`)
- Cloud storage account (Wasabi recommended)
- ~150GB free disk space in `/storage/soar/backups/base`

## Step 1: Set Up Cloud Storage (5 minutes)

### Option A: Backblaze B2 (Recommended - $4.50/month)

1. Create account: https://www.backblaze.com/b2/sign-up.html
2. Create bucket:
   - Name: `soar-backup-prod`
   - Type: Private
   - Lifecycle: Delete files after 30 days
3. Create application key:
   - Go to "App Keys" → "Add a New Application Key"
   - Access: Read and Write to `soar-backup-prod` only
   - Save the **keyID** and **applicationKey**
4. Find your endpoint URL: https://www.backblaze.com/b2/docs/s3_compatible_api.html
   - Format: `https://s3.us-west-004.backblazeb2.com` (region varies)

### Option B: AWS S3 (Standard - $21/month)

1. Create S3 bucket via AWS Console
2. Create IAM user with S3 access
3. Save access key ID and secret key
4. Configure in rclone.conf (see rclone.conf.example)

## Step 2: Configure Backup System (5 minutes)

```bash
# Copy configuration template
sudo mkdir -p /etc/soar
sudo cp backup-env.example /etc/soar/backup-env

# Edit configuration (use your cloud storage credentials)
sudo nano /etc/soar/backup-env
```

**Configure for Wasabi/rclone**:
```bash
# Rclone configuration
BACKUP_RCLONE_REMOTE=wasabi
BACKUP_RCLONE_BUCKET=soar-backup-prod
BACKUP_RCLONE_PATH=

# Backup retention
BASE_BACKUP_KEEP_COUNT=5
WAL_RETENTION_DAYS=30

# PostgreSQL connection
PGHOST=localhost
PGPORT=5432
PGDATABASE=soar
PGUSER=postgres
```

**Set secure permissions:**
```bash
sudo chown postgres:postgres /etc/soar/backup-env
sudo chmod 600 /etc/soar/backup-env
```

**Test cloud access:**
```bash
source /etc/soar/backup-env
rclone lsd ${BACKUP_RCLONE_REMOTE}:${BACKUP_RCLONE_BUCKET}
# Should show empty bucket or return without error
```

## Step 3: Configure PostgreSQL for WAL Archiving (3 minutes)

Edit PostgreSQL configuration:
```bash
sudo nano /etc/postgresql/18/main/postgresql.conf
```

Add these settings (or update if they exist):
```conf
# WAL Archiving for Continuous Backup
wal_level = replica
archive_mode = on
archive_command = '/usr/local/bin/soar-wal-archive %p %f'
archive_timeout = 300

# WAL Sizing
max_wal_size = 4GB
min_wal_size = 1GB
checkpoint_timeout = 15min
wal_keep_size = 160MB

# Monitoring
log_checkpoints = on
log_archive_command = on
```

**Archive script will be installed by deployment:**
The `soar-wal-archive` script will be automatically installed to `/usr/local/bin/soar-wal-archive` by the deployment script.

**Configure authentication for backups:**

Edit `pg_hba.conf` to allow replication connections without password:
```bash
sudo nano /etc/postgresql/18/main/pg_hba.conf
```

Find the replication lines and ensure they use `trust` authentication for localhost:
```conf
# Allow replication connections from localhost, by a user with the
# replication privilege.
local   replication     all                                     peer
host    replication     all             127.0.0.1/32            trust
host    replication     all             ::1/128                 trust
```

**IMPORTANT**: Change `scram-sha-256` to `trust` for the `host replication` lines if needed.

**Restart PostgreSQL:**
```bash
sudo systemctl restart postgresql
```

**Verify WAL archiving is enabled:**
```bash
psql -U postgres -d soar -c "SHOW wal_level;"        # Should show: replica
psql -U postgres -d soar -c "SHOW archive_mode;"     # Should show: on
psql -U postgres -d soar -c "SHOW archive_command;"  # Should show: /usr/local/bin/soar-wal-archive %p %f
```

**Verify backup authentication works:**
```bash
sudo -u postgres pg_basebackup -h localhost -U postgres -D /tmp/test-auth -Fp --progress
# Should start without prompting for password (Ctrl+C to cancel after it starts)
sudo rm -rf /tmp/test-auth
```

## Step 4: Install Backup Services (2 minutes)

```bash
# Copy service files
sudo cp soar-backup-base.service soar-backup-base.timer /etc/systemd/system/
sudo cp soar-backup-verify.service soar-backup-verify.timer /etc/systemd/system/

# Reload systemd
sudo systemctl daemon-reload

# Enable timers (start automatically)
sudo systemctl enable soar-backup-base.timer
sudo systemctl enable soar-backup-verify.timer

# Start timers
sudo systemctl start soar-backup-base.timer
sudo systemctl start soar-backup-verify.timer

# Verify timers are active
sudo systemctl list-timers | grep backup
```

Expected output:
```
Sun 2025-01-12 02:00:00 UTC  ...  soar-backup-base.timer      soar-backup-base.service
Mon 2025-01-13 03:00:00 UTC  ...  soar-backup-verify.timer    soar-backup-verify.service
```

## Step 5: Test Backup System (5 minutes)

### Test WAL Archiving

Generate some database activity to create WAL files:
```bash
psql -U postgres -d soar <<EOF
CREATE TABLE backup_test (id SERIAL, data TEXT);
INSERT INTO backup_test (data) SELECT repeat('X', 1000) FROM generate_series(1, 100000);
DROP TABLE backup_test;
CHECKPOINT;
EOF
```

Check if WAL files are being archived:
```bash
# Check archiver status (last_archived_time should be recent)
psql -U postgres -d soar -c "SELECT * FROM pg_stat_archiver;"

# Check cloud storage for WAL files
source /etc/soar/backup-env
rclone ls ${BACKUP_RCLONE_REMOTE}:${BACKUP_RCLONE_BUCKET}/wal/ | tail -10
```

✓ You should see recent WAL files in cloud storage.

### Test Base Backup (Optional - takes 2-4 hours)

Run first base backup manually:
```bash
sudo systemctl start soar-backup-base.service

# Watch progress
sudo journalctl -u soar-backup-base.service -f
```

Or wait for the weekly timer (Sunday midnight).

### Test Verification

```bash
sudo systemctl start soar-backup-verify.service

# Check results
sudo journalctl -u soar-backup-verify.service -n 50
```

## Monitoring

### Check Backup Status

```bash
# View backup logs
sudo tail -f /var/log/soar/backup.log

# Check timer schedules
sudo systemctl list-timers | grep backup

# Check last base backup
source /etc/soar/backup-env
rclone lsd ${BACKUP_RCLONE_REMOTE}:${BACKUP_RCLONE_BUCKET}/base/

# Check WAL archiving status
psql -U postgres -d soar -c "SELECT last_archived_wal, last_archived_time FROM pg_stat_archiver;"

# Check pg_wal directory size (should stay under 1GB)
du -sh /var/lib/postgresql/18/main/pg_wal/
```

### Important Metrics to Monitor

| Metric | Good | Bad | Action |
|--------|------|-----|--------|
| Last base backup age | < 8 days | > 10 days | Investigate timer/service |
| Last WAL archive | < 5 min | > 15 min | Check archive_command |
| pg_wal/ size | < 1GB | > 5GB | Archive command failing |
| WAL file count | < 20 | > 100 | Archive falling behind |

## Common Issues

### Backup Prompting for Password

**Symptom**: Backup log shows repeated "Password:" prompts, backups timeout after 12 hours

**Cause**: PostgreSQL `pg_hba.conf` requires password authentication for replication connections

**Fix**:
1. Edit `pg_hba.conf`: `sudo nano /etc/postgresql/18/main/pg_hba.conf`
2. Change replication authentication from `scram-sha-256` to `trust` for localhost:
   ```
   host    replication     all             127.0.0.1/32            trust
   host    replication     all             ::1/128                 trust
   ```
3. Reload PostgreSQL: `sudo systemctl reload postgresql`
4. Test: `sudo -u postgres pg_basebackup -h localhost -U postgres -D /tmp/test -Fp --progress`
   - Should start without password prompt (Ctrl+C to cancel)
5. Cleanup: `sudo rm -rf /tmp/test`

### WAL Archiving Not Working

**Symptom**: `pg_wal/` directory growing, no files in cloud storage

**Fix**:
1. Check credentials: `source /etc/soar/backup-env && rclone lsd ${BACKUP_RCLONE_REMOTE}:${BACKUP_RCLONE_BUCKET}`
2. Check logs: `grep "archive command failed" /var/log/postgresql/postgresql-18-main.log`
3. Test manually: `sudo -u postgres /usr/local/bin/soar-wal-archive /var/lib/postgresql/18/main/pg_wal/000000010000000000000001 000000010000000000000001`

### Disk Space Running Low

**Symptom**: `/storage/soar/backups/base` filling up

**Fix**:
1. Check for stuck backups: `ls -lh /storage/soar/backups/base/`
2. Clean old temp files: `sudo rm -rf /storage/soar/backups/base/*`
3. Ensure base backup completed: `sudo journalctl -u soar-backup-base.service -n 100`

### Base Backup Taking Too Long

**Symptom**: Base backup runs for > 6 hours

**Fix**:
1. Check network bandwidth to cloud storage
2. Increase parallel jobs: Set `BACKUP_PARALLEL_JOBS=6` in `/etc/soar/backup-env`
3. Check PostgreSQL load during backup

## Recovery Procedures

### Test Restore (Non-Destructive)

Practice recovery by restoring to a test database:
```bash
sudo -u postgres createdb soar_test_restore
sudo soar-restore --target-database soar_test_restore --no-destructive --latest

# Verify
psql -U postgres -d soar_test_restore -c "SELECT COUNT(*) FROM devices;"

# Cleanup
sudo -u postgres dropdb soar_test_restore
```

### Full Disaster Recovery

**Only run this if you need to restore production!**

See [DB-BACKUPS.md](./DB-BACKUPS.md#restore-procedures) for detailed procedures.

Quick reference:
```bash
# Stop application
sudo systemctl stop soar-*

# Restore to latest point
sudo soar-restore --latest

# Or restore to specific time
sudo soar-restore --target-time "2025-01-10 14:30:00"

# Restart application
sudo systemctl start soar-*
```

## Next Steps

1. **Schedule quarterly restore test**: Add to calendar for first Sunday of each quarter
2. **Set up monitoring**: Add backup metrics to Grafana dashboard
3. **Configure alerts**: Alert if backup age > 8 days or WAL archiving stops
4. **Document your setup**: Note your cloud storage provider and retention settings
5. **Read full documentation**: [DB-BACKUPS.md](./DB-BACKUPS.md)

## Cost Estimate

For 420GB database with 30-day retention:

| Provider | Monthly Cost | Annual Cost |
|----------|--------------|-------------|
| Backblaze B2 | $4.50 | $55 |
| AWS S3 Standard | $21 | $309 |
| Wasabi | $5.40 | $65 |

**Recommendation**: Backblaze B2 for best value.

## Support

- **Full Documentation**: [DB-BACKUPS.md](./DB-BACKUPS.md)
- **Troubleshooting**: See DB-BACKUPS.md#troubleshooting
- **Scripts**: `/usr/local/bin/soar-{wal-archive,base-backup,backup-verify,restore}`
- **Logs**: `/var/log/soar/backup.log`
- **Service Status**: `sudo systemctl status soar-backup-*`
