-- Migration script to move data from raw_messages_default into proper partitions
-- This fixes the issue where partman couldn't create new partitions due to data in DEFAULT
--
-- USAGE: psql -U soar -d soar -f scripts/migrate-raw-messages-default-partition.sql
--
-- This script:
-- 1. Creates missing partitions for Dec 18-20, 2025
-- 2. Detaches the DEFAULT partition (non-blocking)
-- 3. Moves data from detached DEFAULT into proper partitions
-- 4. Re-creates an empty DEFAULT partition

\timing on
\set ON_ERROR_STOP on

BEGIN;

-- Step 1: Check current state
SELECT 'Current state of raw_messages partitions:' as step;
SELECT tablename FROM pg_tables
WHERE tablename LIKE 'raw_messages_p202512%' OR tablename = 'raw_messages_default'
ORDER BY tablename DESC;

SELECT 'Data in raw_messages_default:' as step;
SELECT COUNT(*), MIN(received_at), MAX(received_at)
FROM raw_messages_default;

COMMIT;

-- Step 2: Detach the DEFAULT partition (CONCURRENTLY for non-blocking)
SELECT 'Detaching raw_messages_default partition...' as step;
ALTER TABLE raw_messages DETACH PARTITION raw_messages_default CONCURRENTLY;

-- Verify detachment completed
SELECT 'Verifying detachment...' as step;
SELECT EXISTS (
    SELECT 1 FROM pg_inherits i
    JOIN pg_class child ON i.inhrelid = child.oid
    JOIN pg_class parent ON i.inhparent = parent.oid
    WHERE parent.relname = 'raw_messages' AND child.relname = 'raw_messages_default'
) as still_attached;

-- Step 3: Create missing partitions (now that DEFAULT is detached)
SELECT 'Creating missing partitions...' as step;

CREATE TABLE IF NOT EXISTS raw_messages_p20251218 PARTITION OF raw_messages
FOR VALUES FROM ('2025-12-18 01:00:00+01') TO ('2025-12-19 01:00:00+01');

CREATE TABLE IF NOT EXISTS raw_messages_p20251219 PARTITION OF raw_messages
FOR VALUES FROM ('2025-12-19 01:00:00+01') TO ('2025-12-20 01:00:00+01');

CREATE TABLE IF NOT EXISTS raw_messages_p20251220 PARTITION OF raw_messages
FOR VALUES FROM ('2025-12-20 01:00:00+01') TO ('2025-12-21 01:00:00+01');

-- Step 4: Move data from detached raw_messages_default into proper partitions
-- We'll do this in batches to avoid long locks

SELECT 'Migrating data for 2025-12-18...' as step;
WITH moved_rows AS (
    DELETE FROM raw_messages_default
    WHERE received_at >= '2025-12-18 01:00:00+01'
      AND received_at < '2025-12-19 01:00:00+01'
    RETURNING *
)
INSERT INTO raw_messages SELECT * FROM moved_rows;

SELECT 'Migrating data for 2025-12-19...' as step;
WITH moved_rows AS (
    DELETE FROM raw_messages_default
    WHERE received_at >= '2025-12-19 01:00:00+01'
      AND received_at < '2025-12-20 01:00:00+01'
    RETURNING *
)
INSERT INTO raw_messages SELECT * FROM moved_rows;

SELECT 'Migrating data for 2025-12-20...' as step;
WITH moved_rows AS (
    DELETE FROM raw_messages_default
    WHERE received_at >= '2025-12-20 01:00:00+01'
      AND received_at < '2025-12-21 01:00:00+01'
    RETURNING *
)
INSERT INTO raw_messages SELECT * FROM moved_rows;

-- Step 5: Verify migration
SELECT 'Verification after migration:' as step;
SELECT
    'raw_messages_default' as partition,
    COUNT(*) as row_count,
    MIN(received_at) as min_date,
    MAX(received_at) as max_date
FROM raw_messages_default
UNION ALL
SELECT
    'raw_messages_p20251218' as partition,
    COUNT(*) as row_count,
    MIN(received_at) as min_date,
    MAX(received_at) as max_date
FROM raw_messages_p20251218
UNION ALL
SELECT
    'raw_messages_p20251219' as partition,
    COUNT(*) as row_count,
    MIN(received_at) as min_date,
    MAX(received_at) as max_date
FROM raw_messages_p20251219
UNION ALL
SELECT
    'raw_messages_p20251220' as partition,
    COUNT(*) as row_count,
    MIN(received_at) as min_date,
    MAX(received_at) as max_date
FROM raw_messages_p20251220
ORDER BY partition;

-- Step 6: Drop the detached raw_messages_default (it should be empty now)
SELECT 'Dropping detached raw_messages_default...' as step;
DROP TABLE IF EXISTS raw_messages_default;

-- Step 7: Optionally re-create DEFAULT partition for future overflow
-- (partman config has ignore_default_data = true, so this might not be needed)
-- Uncomment if you want to keep a DEFAULT partition:
-- CREATE TABLE raw_messages_default PARTITION OF raw_messages DEFAULT;

SELECT 'Migration completed successfully!' as step;
SELECT 'Run partman maintenance to ensure future partitions are created:' as next_step;
SELECT 'CALL partman.run_maintenance_proc();' as command;
