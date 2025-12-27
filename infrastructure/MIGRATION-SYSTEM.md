# SOAR Database Migration System

## Overview

The SOAR database migration system provides resilient, async database migrations with automatic notifications on failure. This system was designed to handle long-running migrations that can continue even if SSH connections are lost.

## Features

- **Asynchronous execution**: Migrations run as systemd services, independent of SSH sessions
- **Resilient**: Survives SSH disconnects, network issues, and CI timeouts
- **Email notifications**: Automatic email alerts on migration failures
- **Sentry integration**: Automatic Sentry events for migration failures and successes
- **Detailed logging**: Comprehensive logs in both systemd journal and log files
- **Status tracking**: JSON status files for programmatic monitoring
- **Flexible timeout**: No systemd timeout (deployment script has 2-hour polling timeout)
- **Environment-specific**: Separate migration services for staging and production

## Components

### 1. Systemd Service Template

**File**: `/etc/systemd/system/soar-migrate@.service`

Systemd service template that runs migrations for a specific environment.

**Usage**:
```bash
# Start migration for staging
sudo systemctl start soar-migrate@staging

# Start migration for production
sudo systemctl start soar-migrate@production

# Check status
systemctl status soar-migrate@staging

# View logs
journalctl -u soar-migrate@staging -f
```

### 2. Migration Runner Script

**File**: `/usr/local/bin/soar-migration-runner`

Shell script that:
- Runs `soar migrate` (or `soar-staging migrate` for staging)
- Captures all output
- Updates JSON status file
- Logs everything to systemd journal and files

**Note**: Email and Sentry notifications are sent by the `soar migrate` Rust command itself, not by the wrapper script.

**Environment variables** (from environment files):
- Staging: `/etc/soar/env-staging`
- Production: `/etc/soar/env-production`

Variables:
- `DATABASE_URL`: PostgreSQL connection string
- `SMTP_SERVER`, `SMTP_PORT`, `SMTP_USERNAME`, `SMTP_PASSWORD`: Email configuration (used by Rust command)
- `FROM_EMAIL`, `FROM_NAME`: Email sender info (used by Rust command)
- `MIGRATION_ALERT_EMAIL`: Override recipient (defaults to `FROM_EMAIL`, used by Rust command)
- `SENTRY_DSN`: Sentry project DSN for alerts (used by Rust command)

### 3. Status Checker Script

**File**: `/usr/local/bin/soar-migration-status`

Helper script to check migration status.

**Usage**:
```bash
# Check staging migration status
soar-migration-status staging

# Check production migration status
soar-migration-status production
```

**Output**:
- Current migration status (running/completed/failed)
- Timestamp
- Error message (if failed)
- Exit code (if failed)
- Log file location
- Recent systemd journal entries

### 4. Deployment Integration

**File**: `infrastructure/soar-deploy`

The deployment script automatically uses the migration system when deploying:

1. Stops `soar-run` service
2. Starts `soar-migrate@{environment}` service
3. Polls for completion (checks every 5 seconds)
4. Shows progress updates every 30 seconds
5. On success: continues deployment
6. On failure: aborts deployment, sends notifications, leaves services stopped

## Migration Lifecycle

### Normal Flow

```
1. soar-deploy starts migration service
   ↓
2. soar-migration-runner executes
   ↓
3. soar migrate completes
   ↓
4. Status file updated (completed)
   ↓
5. Sentry success event sent
   ↓
6. Service exits successfully
   ↓
7. soar-deploy continues
```

### Failure Flow

```
1. soar-deploy starts migration service
   ↓
2. soar-migration-runner executes
   ↓
3. soar migrate FAILS
   ↓
4. Status file updated (failed)
   ↓
5. Email notification sent
   ↓
6. Sentry error event sent
   ↓
7. Service exits with error
   ↓
8. soar-deploy aborts
```

## Status File Format

**Location**: `/var/soar/migration-status/{environment}.json`

**Format**:
```json
{
  "status": "completed|running|failed",
  "environment": "staging|production",
  "timestamp": "2025-12-27T03:15:30Z",
  "log_file": "/var/soar/logs/migrations/migration-staging-20251227_031530.log",
  "message": "Migration completed successfully",
  "exit_code": 0
}
```

## Log Files

### Systemd Journal

**View live logs**:
```bash
journalctl -u soar-migrate@staging -f
```

**View last 100 lines**:
```bash
journalctl -u soar-migrate@staging -n 100
```

**View logs since date**:
```bash
journalctl -u soar-migrate@staging --since "2025-12-27 00:00:00"
```

### Log Files

**Location**: `/var/soar/logs/migrations/migration-{environment}-{timestamp}.log`

**Example**: `/var/soar/logs/migrations/migration-staging-20251227_031530.log`

These files contain:
- Full migration output from diesel
- All log messages from the runner script
- Timestamps for all operations

## Email Notifications

Email notifications are sent by the `soar migrate` Rust command using the HTML email templates.

### Configuration

Set in `/etc/soar/env-production` or `/etc/soar/env-staging`:

```bash
# SMTP settings
SMTP_SERVER=smtp.example.com
SMTP_PORT=587
SMTP_USERNAME=alerts@example.com
SMTP_PASSWORD=secret

# Email sender
FROM_EMAIL=alerts@example.com
FROM_NAME="SOAR Migrations"

# Alert recipient (optional, defaults to FROM_EMAIL)
# For SOAR project: configured to liam@supervillains.io
MIGRATION_ALERT_EMAIL=liam@supervillains.io
```

### Email Content

**On Success:**

**Subject**: `✓ SOAR Database Migration COMPLETED - YYYY-MM-DD`
**Subject (staging)**: `[STAGING] ✓ SOAR Database Migration COMPLETED - YYYY-MM-DD`

**Body** includes (HTML formatted):
- Environment (staging/production/development)
- Hostname
- Duration
- Timestamp
- List of applied migrations (if any)

**On Failure:**

**Subject**: `✗ SOAR Database Migration FAILED - YYYY-MM-DD`
**Subject (staging)**: `[STAGING] ✗ SOAR Database Migration FAILED - YYYY-MM-DD`

**Body** includes (HTML formatted):
- Environment (staging/production/development)
- Hostname
- Duration
- Timestamp
- Error message
- Migration output (if available)

### When Emails Are Sent

- **On success**: Confirmation email with migration summary
- **On failure**: Alert email with error details

### Implementation

Email sending is implemented in the Rust code at `src/migration_email_reporter.rs` using the same patterns as the archive and data load commands.

## Sentry Integration

Sentry events are sent by the `soar migrate` Rust command using the Sentry SDK.

### Configuration

Set in `/etc/soar/env-production` or `/etc/soar/env-staging`:

```bash
SENTRY_DSN=https://key@sentry.io/project-id
```

### Events Sent

**On Success** (info level):
```
Message: "Database migration completed successfully for staging"
Level: info
Tags:
  - migration: true
  - environment: staging
  - type: database_migration
```

**On Failure** (error level):
```
Message: "Database migration failed for staging (error: ...)"
Level: error
Tags:
  - migration: true
  - environment: staging
  - type: database_migration
```

### Implementation

Sentry integration is implemented in the Rust code at `src/main.rs` in the Migrate command handler using the `sentry` crate.

## Manual Migration

If you need to run migrations manually (outside of deployment):

### 1. Stop Services

```bash
sudo systemctl stop soar-run-staging
# Or for production:
sudo systemctl stop soar-run
```

### 2. Start Migration

```bash
sudo systemctl start soar-migrate@staging
# Or for production:
sudo systemctl start soar-migrate@production
```

### 3. Monitor Progress

```bash
# Watch live logs
journalctl -u soar-migrate@staging -f

# Or check status
soar-migration-status staging
```

### 4. Wait for Completion

```bash
# Wait for service to complete
while systemctl is-active --quiet soar-migrate@staging; do
    sleep 5
    echo "Still running..."
done

# Check final status
soar-migration-status staging
```

### 5. Restart Services

```bash
sudo systemctl start soar-run-staging
# Or for production:
sudo systemctl start soar-run
```

## Troubleshooting

### Migration Stuck/Hanging

If migration appears stuck:

1. **Check if actually running**:
   ```bash
   systemctl is-active soar-migrate@staging
   journalctl -u soar-migrate@staging -n 50
   ```

2. **Check PostgreSQL**:
   ```bash
   # Look for long-running queries
   psql -U soar -d soar_staging -c "SELECT pid, now() - query_start AS duration, query FROM pg_stat_activity WHERE state = 'active' ORDER BY duration DESC;"
   ```

3. **Check locks**:
   ```bash
   psql -U soar -d soar_staging -c "SELECT * FROM pg_locks WHERE NOT granted;"
   ```

### Migration Failed

1. **Check status file**:
   ```bash
   cat /var/soar/migration-status/staging.json
   ```

2. **View full logs**:
   ```bash
   journalctl -u soar-migrate@staging -n 200
   # Or view log file directly
   cat /var/soar/logs/migrations/migration-staging-*.log | tail -100
   ```

3. **Check database state**:
   ```bash
   psql -U soar -d soar_staging -c "SELECT * FROM __diesel_schema_migrations ORDER BY version DESC LIMIT 5;"
   ```

4. **Fix and retry**:
   - Identify and fix the issue
   - Reset failed state: `sudo systemctl reset-failed soar-migrate@staging`
   - Retry: `sudo systemctl start soar-migrate@staging`

### Email Not Received

1. **Check SMTP configuration**:
   ```bash
   cat /etc/soar/env-staging | grep SMTP
   ```

2. **Test SMTP manually**:
   ```bash
   python3 -c "import smtplib; s = smtplib.SMTP('smtp.example.com', 587); s.starttls(); s.login('user', 'pass'); print('OK')"
   ```

3. **Check migration logs for email errors**:
   ```bash
   journalctl -u soar-migrate@staging | grep -i email
   ```

### Sentry Event Not Received

1. **Check Sentry DSN**:
   ```bash
   cat /etc/soar/env-staging | grep SENTRY_DSN
   ```

2. **Test Sentry manually**:
   ```bash
   curl -X POST "https://sentry.io/api/PROJECT_ID/store/" \
     -H "X-Sentry-Auth: Sentry sentry_version=7, sentry_key=KEY" \
     -H "Content-Type: application/json" \
     -d '{"message":"test"}'
   ```

3. **Check migration logs**:
   ```bash
   journalctl -u soar-migrate@staging | grep -i sentry
   ```

## Best Practices

### For Large Migrations

1. **Test on staging first**:
   - Always test large migrations on staging
   - Measure execution time
   - Verify data integrity

2. **Schedule maintenance window**:
   - Plan for 2x the staging execution time
   - Communicate downtime to users
   - Have rollback plan ready

3. **Monitor during execution**:
   - Watch logs in real-time
   - Monitor database connections
   - Check system resources (CPU, memory, disk I/O)

4. **Verify after completion**:
   - Check row counts
   - Verify data integrity
   - Test application functionality

### For CI/CD

The migration system is designed to work with CI/CD:

- GitHub Actions can SSH and run `soar-deploy`
- If SSH disconnects, migration continues
- CI can poll status by SSHing back in
- Failures trigger email and Sentry alerts
- No manual intervention needed

## Installation

The migration system is installed automatically by the provision script:

```bash
sudo ./scripts/provision staging
# Or
sudo ./scripts/provision production
```

This creates:
- `/etc/systemd/system/soar-migrate@.service`
- `/usr/local/bin/soar-migration-runner`
- `/usr/local/bin/soar-migration-status`
- `/var/soar/logs/migrations/`
- `/var/soar/migration-status/`

## See Also

- [TIMESCALEDB_MIGRATION_PLAN.md](../TIMESCALEDB_MIGRATION_PLAN.md) - Specific plan for TimescaleDB migration
- [DEPLOYMENT.md](DEPLOYMENT.md) - General deployment documentation
- [soar-deploy](soar-deploy) - Deployment script source code
