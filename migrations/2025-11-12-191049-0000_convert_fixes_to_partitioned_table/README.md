# Fixes and APRS Messages Table Partitioning Migration

This migration converts the `fixes` and `aprs_messages` tables from monolithic tables to partitioned tables managed by pg_partman.

## Performance Impact

**Before partitioning:**
- Bounding box queries with time filters: **90+ seconds**
- Full table scans required even for recent data
- 225M+ rows in fixes table

**After partitioning:**
- Bounding box queries with time filters: **<1 second** (expected)
- Only recent partitions scanned (partition pruning)
- Data split into daily partitions

## Migration Timeline

**Estimated time on production:** 40-70 minutes

**Steps:**
1. Install pg_partman extension (~1s)
2. Partition fixes table:
   - Rename fixes → fixes_old (~1s)
   - Create partitioned table structure (~1s)
   - Create partitions for existing data (~30s)
   - Migrate 225M rows to partitions in batches (**SLOW**: 30-60 mins)
     - The migration loops automatically, processing 10 days of data per iteration
     - Progress is logged with NOTICE messages showing rows moved per iteration
   - Recreate indexes on partitions (~2-3 mins)
   - Add constraints (~10s)
3. Partition aprs_messages table:
   - Rename aprs_messages → aprs_messages_old (~1s)
   - Create partitioned table structure (~1s)
   - Create partitions (~30s)
   - Migrate data in batches (~5-10 mins)
     - The migration loops automatically until all data is migrated
   - Recreate indexes (~1 min)
   - Add constraints (~10s)
4. Configure pg_partman with 30-day retention (~1s)

## Prerequisites

### Install pg_partman Extension

On Debian/Ubuntu:
```bash
# For PostgreSQL 17 (production version)
sudo apt-get install postgresql-17-partman

# For PostgreSQL 16
sudo apt-get install postgresql-16-partman
```

On macOS (Homebrew):
```bash
brew install pg_partman
```

Verify installation:
```sql
SELECT * FROM pg_available_extensions WHERE name = 'pg_partman';
```

## Running the Migration

### Development/Staging
```bash
diesel migration run
```

### Production
**IMPORTANT:** Run during maintenance window!

```bash
# 1. Notify users of downtime
# 2. Stop the application
sudo systemctl stop soar

# 3. Run migration
diesel migration run --database-url="$DATABASE_URL"

# 4. Verify migration succeeded
psql $DATABASE_URL -c "SELECT COUNT(*) FROM fixes;"
psql $DATABASE_URL -c "SELECT COUNT(*) FROM fixes_old;"
psql $DATABASE_URL -c "SELECT COUNT(*) FROM aprs_messages;"
psql $DATABASE_URL -c "SELECT COUNT(*) FROM aprs_messages_old;"

# 5. Once verified (after a few days), drop old tables
psql $DATABASE_URL -c "DROP TABLE fixes_old;"
psql $DATABASE_URL -c "DROP TABLE aprs_messages_old;"

# 6. Restart application
sudo systemctl start soar
```

## Ongoing Maintenance

### Systemd Timer (Installed)

A systemd timer has been installed and configured to run partition maintenance daily at 3 AM.

**Service file**: `/etc/systemd/system/partman-maintenance.service`
```ini
[Unit]
Description=pg_partman partition maintenance
After=postgresql.service

[Service]
Type=oneshot
User=soar
ExecStart=/usr/bin/psql postgres://localhost/soar -c "CALL partman.run_maintenance_proc()"
StandardOutput=append:/var/soar/logs/partman.log
StandardError=append:/var/soar/logs/partman.log
```

**Timer file**: `/etc/systemd/system/partman-maintenance.timer`
```ini
[Unit]
Description=Run pg_partman maintenance daily at 3 AM
Requires=partman-maintenance.service

[Timer]
OnCalendar=*-*-* 03:00:00
Persistent=true

[Install]
WantedBy=timers.target
```

**Check timer status**:
```bash
sudo systemctl status partman-maintenance.timer
sudo systemctl list-timers partman-maintenance.timer
```

**View logs**:
```bash
tail -f /var/soar/logs/partman.log
journalctl -u partman-maintenance.service
```

**Manually trigger maintenance** (for testing):
```bash
sudo systemctl start partman-maintenance.service
```

### Alternative: pg_cron

If you prefer database-level scheduling, you can use pg_cron instead:

```bash
# For PostgreSQL 17 (production version)
sudo apt-get install postgresql-17-cron

# For PostgreSQL 16
sudo apt-get install postgresql-16-cron
```

```sql
-- Run partition maintenance daily at 3 AM for both tables
SELECT cron.schedule('partman-maintenance', '0 3 * * *',
    $$CALL partman.run_maintenance_proc()$$
);
```

## What pg_partman Maintenance Does

When `partman.run_maintenance_proc()` runs, it:

1. **Creates future partitions** (3 days ahead by default)
2. **Detaches old partitions** (older than 30 days based on retention config)
3. **Does NOT drop detached partitions** (allows manual verification)

### Safe Partition Dropping

With `retention_keep_table = true`, pg_partman will:
- **DETACH** partitions older than 30 days (they become standalone tables)
- **NOT drop** the detached partitions automatically
- Allow **manual inspection** of detached partitions before dropping

This ensures the soar-archive process has time to archive data before partitions are removed.

### Manually Dropping Detached Partitions

After the archive process completes, you can safely drop empty partitions:

```sql
-- List all detached partition tables
SELECT
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname || '.' || tablename)) AS size,
    (SELECT COUNT(*) FROM pg_class c
     WHERE c.relname = t.tablename) as exists
FROM pg_tables t
WHERE schemaname = 'public'
  AND (tablename LIKE 'fixes_p%' OR tablename LIKE 'aprs_messages_p%')
ORDER BY tablename;
```

To check if a partition is empty:
```sql
-- Check row count for a specific partition
SELECT COUNT(*) FROM fixes_p20251001;
SELECT COUNT(*) FROM aprs_messages_p20251001;
```

To drop empty partitions:
```sql
-- Only drop if verified empty and archived!
DROP TABLE IF EXISTS fixes_p20251001;
DROP TABLE IF EXISTS aprs_messages_p20251001;
```

## Monitoring

### Check Current Partitions
```sql
-- Fixes partitions
SELECT tablename, pg_size_pretty(pg_total_relation_size('public.' || tablename)) as size
FROM pg_tables
WHERE schemaname = 'public'
  AND tablename LIKE 'fixes_p%'
ORDER BY tablename DESC
LIMIT 10;

-- APRS messages partitions
SELECT tablename, pg_size_pretty(pg_total_relation_size('public.' || tablename)) as size
FROM pg_tables
WHERE schemaname = 'public'
  AND tablename LIKE 'aprs_messages_p%'
ORDER BY tablename DESC
LIMIT 10;
```

### Check Partition Sizes and Row Counts
```sql
SELECT
    schemaname || '.' || tablename AS partition,
    pg_size_pretty(pg_total_relation_size(schemaname || '.' || tablename)) AS size,
    n_live_tup AS row_count
FROM pg_stat_user_tables
WHERE tablename LIKE 'fixes_p%' OR tablename LIKE 'aprs_messages_p%'
ORDER BY tablename DESC;
```

### Check pg_partman Configuration
```sql
SELECT parent_table, retention, retention_keep_table, premake, infinite_time_partitions
FROM partman.part_config
WHERE parent_table IN ('public.fixes', 'public.aprs_messages');
```

### Verify Partition Pruning is Working
```sql
-- Fixes partition pruning
EXPLAIN ANALYZE
SELECT COUNT(*)
FROM fixes
WHERE received_at >= NOW() - INTERVAL '1 day';

-- APRS messages partition pruning
EXPLAIN ANALYZE
SELECT COUNT(*)
FROM aprs_messages
WHERE received_at >= NOW() - INTERVAL '1 day';
```

You should see "Partitions pruned: N" in the output.

## Rollback

If something goes wrong:

```bash
diesel migration revert
```

This will:
1. Drop the partitioned `fixes` and `aprs_messages` tables
2. Rename `fixes_old` and `aprs_messages_old` back to original names

**Note:** Only works if old tables haven't been dropped yet.

## Troubleshooting

### Migration Fails During Data Copy

If the migration fails partway through:

```sql
-- Check how much data was migrated
SELECT COUNT(*) FROM fixes;
SELECT COUNT(*) FROM fixes_old;
SELECT COUNT(*) FROM aprs_messages;
SELECT COUNT(*) FROM aprs_messages_old;

-- If incomplete, rollback
diesel migration revert

-- Fix the issue and try again
```

### Partitions Not Being Created Automatically

Check pg_partman maintenance is running:

```sql
-- For pg_cron
SELECT * FROM cron.job WHERE command LIKE '%partman%';

-- Check pg_partman configuration
SELECT * FROM partman.part_config;
```

### Performance Not Improved

Verify partition pruning is working:

```sql
-- This should only scan recent partitions
EXPLAIN (ANALYZE, BUFFERS)
SELECT * FROM fixes
WHERE received_at >= NOW() - INTERVAL '6 hours'
LIMIT 100;
```

Look for "Partitions pruned: N" in the output.

### Foreign Key Issues with aprs_messages

The fixes table has a foreign key to aprs_messages. Both tables reference the `id` column which is now part of a composite primary key `(id, received_at)`.

Foreign keys should work correctly because:
- `fixes.aprs_message_id` → `aprs_messages.id` (ON DELETE SET NULL)
- `receiver_statuses.aprs_message_id` → `aprs_messages.id` (ON DELETE SET NULL)

If you encounter FK constraint errors, verify the constraints were recreated properly:
```sql
SELECT conname, conrelid::regclass, confrelid::regclass, pg_get_constraintdef(oid)
FROM pg_constraint
WHERE contype = 'f'
  AND (confrelid = 'fixes'::regclass OR confrelid = 'aprs_messages'::regclass);
```

## Query Changes Required

**None!** The partitioned tables are transparent to application queries. All existing queries work exactly the same.

The PostgreSQL query planner automatically uses partition pruning when queries include `received_at` filters.

## Retention Policy

- **Retention period**: 30 days
- **Behavior**: Detach partitions older than 30 days (don't drop automatically)
- **Manual cleanup**: Drop detached partitions after verifying they're archived and empty

To adjust retention in the future:
```sql
-- Change retention period (requires manual partition drops)
UPDATE partman.part_config SET retention = '60 days' WHERE parent_table = 'public.fixes';
UPDATE partman.part_config SET retention = '60 days' WHERE parent_table = 'public.aprs_messages';

-- Or enable automatic dropping (NOT recommended initially)
UPDATE partman.part_config SET retention_keep_table = false WHERE parent_table = 'public.fixes';
UPDATE partman.part_config SET retention_keep_table = false WHERE parent_table = 'public.aprs_messages';
```
