-- Fix foreign key constraints to point to partitioned aprs_messages table
-- Both fixes and receiver_statuses need composite FKs matching the composite PK

-- Step 1: Drop old FK constraints and any auto-generated partition-specific constraints
ALTER TABLE fixes DROP CONSTRAINT IF EXISTS fixes_aprs_message_id_fkey;
ALTER TABLE receiver_statuses DROP CONSTRAINT IF EXISTS receiver_statuses_aprs_message_id_fkey;

-- Drop any partition-specific constraints that may have been auto-created
-- These point to individual partitions instead of the parent table
DO $$
DECLARE
    constraint_rec RECORD;
BEGIN
    -- Drop partition-specific FK constraints from fixes table
    FOR constraint_rec IN
        SELECT conname
        FROM pg_constraint
        WHERE conrelid = 'fixes'::regclass
          AND conname LIKE 'fixes_aprs_message_id_%_fkey%'
    LOOP
        EXECUTE 'ALTER TABLE fixes DROP CONSTRAINT IF EXISTS ' || quote_ident(constraint_rec.conname);
        RAISE NOTICE 'Dropped constraint: %', constraint_rec.conname;
    END LOOP;

    -- Drop partition-specific FK constraints from receiver_statuses table
    FOR constraint_rec IN
        SELECT conname
        FROM pg_constraint
        WHERE conrelid = 'receiver_statuses'::regclass
          AND conname LIKE 'receiver_statuses_aprs_message_id_%_fkey%'
    LOOP
        EXECUTE 'ALTER TABLE receiver_statuses DROP CONSTRAINT IF EXISTS ' || quote_ident(constraint_rec.conname);
        RAISE NOTICE 'Dropped constraint: %', constraint_rec.conname;
    END LOOP;
END $$;

-- Step 2: Sync received_at timestamps to match aprs_messages
-- The received_at values were set independently and differ by microseconds
-- We need them to match exactly for the composite FK constraint to work

-- Log progress for fixes sync
DO $$
DECLARE
    rows_to_update BIGINT;
BEGIN
    SELECT COUNT(*) INTO rows_to_update
    FROM fixes f
    JOIN aprs_messages am ON f.aprs_message_id = am.id
    WHERE f.received_at != am.received_at;

    RAISE NOTICE 'Syncing % rows in fixes table to match aprs_messages timestamps', rows_to_update;
END $$;

-- Sync fixes.received_at to match aprs_messages.received_at
-- Note: This UPDATE on a partitioned table may move rows between partitions
-- This is slow but necessary for FK constraint integrity
UPDATE fixes f
SET received_at = am.received_at
FROM aprs_messages am
WHERE f.aprs_message_id = am.id
  AND f.received_at != am.received_at;

-- Log progress for receiver_statuses sync
DO $$
DECLARE
    rows_to_update BIGINT;
BEGIN
    SELECT COUNT(*) INTO rows_to_update
    FROM receiver_statuses rs
    JOIN aprs_messages am ON rs.aprs_message_id = am.id
    WHERE rs.received_at != am.received_at;

    RAISE NOTICE 'Syncing % rows in receiver_statuses table to match aprs_messages timestamps', rows_to_update;
END $$;

-- Sync receiver_statuses.received_at to match aprs_messages.received_at
UPDATE receiver_statuses rs
SET received_at = am.received_at
FROM aprs_messages am
WHERE rs.aprs_message_id = am.id
  AND rs.received_at != am.received_at;

-- Step 2b: Delete any rows that STILL would violate the FK constraint
-- This handles orphaned aprs_message_ids that don't exist in aprs_messages
DELETE FROM fixes f
WHERE f.aprs_message_id IS NOT NULL
  AND NOT EXISTS (
      SELECT 1 FROM aprs_messages am
      WHERE am.id = f.aprs_message_id
        AND am.received_at = f.received_at
  );

DELETE FROM receiver_statuses rs
WHERE rs.aprs_message_id IS NOT NULL
  AND NOT EXISTS (
      SELECT 1 FROM aprs_messages am
      WHERE am.id = rs.aprs_message_id
        AND am.received_at = rs.received_at
  );

-- Step 3: Add new composite FK constraints pointing to aprs_messages (id, received_at)
ALTER TABLE fixes
    ADD CONSTRAINT fixes_aprs_message_id_fkey
    FOREIGN KEY (aprs_message_id, received_at)
    REFERENCES aprs_messages(id, received_at);

ALTER TABLE receiver_statuses
    ADD CONSTRAINT receiver_statuses_aprs_message_id_fkey
    FOREIGN KEY (aprs_message_id, received_at)
    REFERENCES aprs_messages(id, received_at);
