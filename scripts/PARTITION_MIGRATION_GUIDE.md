# Partition Migration Guide - Fixing DEFAULT Partition Issue

## Problem Summary

**Issue**: Partman maintenance is failing with:
```
ERROR: partition "fixes_p20251218" would overlap partition "fixes_p20251217"
```

**Root Cause**:
- Partman maintenance stopped running (likely due to "too many clients" connection limit)
- Without maintenance, new partitions weren't created for Dec 18-20
- Data fell through to DEFAULT partitions (`fixes_default`, `raw_messages_default`)
- PostgreSQL won't create new partitions that overlap with data in DEFAULT
- Result: **54M rows in `fixes_default`**, **94M rows in `raw_messages_default`**

## Current State

```
fixes_default:         53,900,266 rows (Dec 18 - Dec 20)
raw_messages_default:  93,696,238 rows (Dec 18 - Dec 21)
```

Both tables are configured with `ignore_default_data = true`, meaning DEFAULT partitions should NOT exist.

## Migration Plan

This migration will:
1. ✅ **Preserve all data** (no data loss)
2. ✅ Detach DEFAULT partitions (**NOTE**: Cannot use CONCURRENTLY when DEFAULT exists - PostgreSQL limitation)
3. ✅ Create missing partitions (Dec 18, 19, 20)
4. ✅ Move data from DEFAULT into proper partitions (excluding generated columns)
5. ✅ Drop empty DEFAULT partitions
6. ✅ Resume normal partman operation

**Important Notes:**
- `DETACH PARTITION CONCURRENTLY` is not supported when a DEFAULT partition exists on the parent table
- The migration uses regular `DETACH PARTITION` which will briefly lock the parent table
- Generated columns (`location`, `location_geom` in fixes table) are auto-generated and excluded from INSERT

## Execution Steps

### Step 1: Test on Staging First ✅ COMPLETED

**Test Results (2025-12-20):**
- ✅ Successfully detached and dropped `fixes_default` (was empty on staging)
- ✅ Successfully detached and dropped `raw_messages_default` (was empty on staging)
- ✅ Partman maintenance runs without errors
- ✅ New partitions created successfully (e.g., raw_messages_p20251224)
- ✅ No DEFAULT partitions re-created (as expected with `ignore_default_data = true`)

**Note:** Staging had empty DEFAULT partitions, so data migration was not tested. Production has 54M+94M rows that will be migrated.

~~```bash
# On your local machine
psql -U soar -d soar_staging -f scripts/migrate-default-partition.sql
psql -U soar -d soar_staging -f scripts/migrate-raw-messages-default-partition.sql
```~~

### Step 2: Execute on Production

**Timing**: Run during low-traffic period if possible (partitions will be briefly locked during data moves)

```bash
# SSH to production
ssh glider.flights

# Run migrations (they include timing and verification)
psql -U soar -d soar -f /path/to/migrate-default-partition.sql
psql -U soar -d soar -f /path/to/migrate-raw-messages-default-partition.sql
```

### Step 3: Verify Migration

```sql
-- Check that DEFAULT partitions are gone
SELECT tablename FROM pg_tables
WHERE tablename IN ('fixes_default', 'raw_messages_default');
-- Should return 0 rows

-- Verify new partitions exist and have data
SELECT tablename FROM pg_tables
WHERE tablename LIKE 'fixes_p202512%'
ORDER BY tablename DESC;

SELECT tablename FROM pg_tables
WHERE tablename LIKE 'raw_messages_p202512%'
ORDER BY tablename DESC;

-- Check data distribution
SELECT
    schemaname || '.' || tablename as partition,
    pg_size_pretty(pg_total_relation_size(schemaname || '.' || tablename)) as size
FROM pg_tables
WHERE tablename LIKE 'fixes_p202512%'
ORDER BY tablename DESC;
```

### Step 4: Test Partman Maintenance

```sql
-- This should now succeed without errors
CALL partman.run_maintenance_proc();

-- Verify it created today's partition (Dec 21)
SELECT tablename FROM pg_tables
WHERE tablename IN ('fixes_p20251221', 'raw_messages_p20251221');
```

### Step 5: Monitor

```bash
# Check journalctl for partman logs (now that we fixed the logging)
journalctl -u partman-maintenance -f

# Or run the timer manually to verify
sudo systemctl start partman-maintenance.service
journalctl -u partman-maintenance --since "1 minute ago"
```

## What Each Migration Does

### migrate-default-partition.sql (fixes table)

1. Shows current state
2. Detaches `fixes_default` using CONCURRENTLY (non-blocking)
3. Creates `fixes_p20251218`, `fixes_p20251219`, `fixes_p20251220`
4. Moves data from detached default into proper partitions using DELETE...RETURNING + INSERT
5. Verifies row counts
6. Drops empty `fixes_default`

**Estimated time**: 5-10 minutes for 54M rows (depends on disk I/O)

### migrate-raw-messages-default-partition.sql (raw_messages table)

Same process for `raw_messages_default` (94M rows)

**Estimated time**: 10-15 minutes for 94M rows

## Rollback Plan

If something goes wrong DURING migration:

```sql
-- If DEFAULT is detached but data hasn't been moved yet:
-- 1. Re-attach it
ALTER TABLE fixes ATTACH PARTITION fixes_default DEFAULT;

-- 2. Drop any partially created partitions
DROP TABLE IF EXISTS fixes_p20251218;
DROP TABLE IF EXISTS fixes_p20251219;
DROP TABLE IF EXISTS fixes_p20251220;

-- 3. Investigate the error and retry
```

## Post-Migration

1. ✅ DEFAULT partitions should be gone
2. ✅ Partman maintenance should run successfully
3. ✅ New partitions created automatically each day
4. ✅ Systemd logs to journalctl (view with `journalctl -u partman-maintenance`)

## Why This Happened

1. **Max connections hit**: `psql: error: connection to server... too many clients already`
2. **Partman couldn't run**: Without maintenance, partitions weren't pre-created
3. **Data went to DEFAULT**: PostgreSQL routed all new data to DEFAULT partition
4. **Partman blocked**: Can't create partition when DEFAULT already has that data

## Prevention

- ✅ **Fixed systemd logging** (now uses journalctl, no separate log file)
- ✅ **Monitor max_connections**: Check if we need to increase it
- ✅ **partman config verified**: `ignore_default_data = true` (no DEFAULT needed)
- ✅ **Monitor partition creation**: Check that new partitions are created daily

## Questions?

Check logs:
```bash
# Partman maintenance logs
journalctl -u partman-maintenance --since "1 day ago"

# PostgreSQL logs
sudo tail -f /var/log/postgresql/postgresql-*.log
```
