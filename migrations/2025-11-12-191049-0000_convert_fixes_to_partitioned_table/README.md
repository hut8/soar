# Fixes Table Partitioning Migration

This migration converts the `fixes` table from a single 225M+ row monolithic table to a partitioned table managed by pg_partman.

## Performance Impact

**Before partitioning:**
- Bounding box queries with time filters: **90+ seconds**
- Full table scans required even for recent data
- 225M rows in one table

**After partitioning:**
- Bounding box queries with time filters: **<1 second** (expected)
- Only recent partitions scanned (partition pruning)
- Data split into daily partitions

## Migration Timeline

**Estimated time on production:** 30-60 minutes

**Steps:**
1. Install pg_partman extension (~1s)
2. Rename fixes → fixes_old (~1s)
3. Create partitioned table structure (~1s)
4. Create partitions for existing data (~30s)
5. Migrate 225M rows to partitions (**SLOW**: 25-55 mins)
6. Recreate indexes on partitions (~2-3 mins)
7. Add constraints (~10s)
8. Configure pg_partman (~1s)

## Prerequisites

### Install pg_partman Extension

On Debian/Ubuntu:
```bash
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

# 5. Once verified (after a few days), drop old table
psql $DATABASE_URL -c "DROP TABLE fixes_old;"

# 6. Restart application
sudo systemctl start soar
```

## Ongoing Maintenance

### Option 1: pg_cron (Recommended)

Install pg_cron extension:
```bash
sudo apt-get install postgresql-16-cron
```

Schedule daily maintenance:
```sql
-- Run partition maintenance daily at 3 AM
SELECT cron.schedule('partman-maintenance', '0 3 * * *',
    $$SELECT partman.run_maintenance('public.fixes')$$
);
```

### Option 2: System Cron

Add to crontab:
```bash
# Run partition maintenance daily at 3 AM
0 3 * * * psql $DATABASE_URL -c "SELECT partman.run_maintenance('public.fixes')" >> /var/soar/logs/partman.log 2>&1
```

### Option 3: Systemd Timer

Create `/etc/systemd/system/partman-maintenance.service`:
```ini
[Unit]
Description=pg_partman partition maintenance
After=postgresql.service

[Service]
Type=oneshot
User=postgres
ExecStart=/usr/bin/psql -d soar -c "SELECT partman.run_maintenance('public.fixes')"
```

Create `/etc/systemd/system/partman-maintenance.timer`:
```ini
[Unit]
Description=Run pg_partman maintenance daily

[Timer]
OnCalendar=daily
OnCalendar=03:00
Persistent=true

[Install]
WantedBy=timers.target
```

Enable the timer:
```bash
sudo systemctl enable partman-maintenance.timer
sudo systemctl start partman-maintenance.timer
```

## What pg_partman Maintenance Does

When `partman.run_maintenance()` runs, it:

1. **Creates future partitions** (3 days ahead by default)
2. **Drops old partitions** (older than 8 days based on retention config)
3. **Updates statistics** on partition tables

## Monitoring

### Check Current Partitions
```sql
SELECT tablename
FROM pg_tables
WHERE schemaname = 'public'
  AND tablename LIKE 'fixes_%'
ORDER BY tablename DESC
LIMIT 10;
```

### Check Partition Sizes
```sql
SELECT
    schemaname || '.' || tablename AS partition,
    pg_size_pretty(pg_total_relation_size(schemaname || '.' || tablename)) AS size,
    n_live_tup AS row_count
FROM pg_stat_user_tables
WHERE tablename LIKE 'fixes_%'
ORDER BY tablename DESC;
```

### Check pg_partman Configuration
```sql
SELECT * FROM partman.part_config WHERE parent_table = 'public.fixes';
```

### Verify Partition Pruning is Working
```sql
EXPLAIN ANALYZE
SELECT COUNT(*)
FROM fixes
WHERE received_at >= NOW() - INTERVAL '1 day';
```

You should see "Partitions pruned: N" in the output.

## Rollback

If something goes wrong:

```bash
diesel migration revert
```

This will:
1. Drop the partitioned `fixes` table
2. Rename `fixes_old` back to `fixes`

**Note:** Only works if `fixes_old` hasn't been dropped yet.

## Troubleshooting

### Migration Fails During Data Copy

If the migration fails partway through:

```sql
-- Check how much data was migrated
SELECT COUNT(*) FROM fixes;
SELECT COUNT(*) FROM fixes_old;

-- If incomplete, rollback
diesel migration revert

-- Fix the issue and try again
```

### Partitions Not Being Created Automatically

Check pg_partman maintenance is running:

```sql
-- For pg_cron
SELECT * FROM cron.job WHERE command LIKE '%partman%';

-- Check last run
SELECT * FROM partman.part_config_sub WHERE sub_parent = 'public.fixes';
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

## Query Changes Required

**None!** The partitioned table is transparent to application queries. All existing queries work exactly the same.

The PostgreSQL query planner automatically uses partition pruning when queries include `received_at` filters.
