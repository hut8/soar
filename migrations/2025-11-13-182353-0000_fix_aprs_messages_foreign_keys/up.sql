-- Fix foreign key constraints to point to partitioned aprs_messages table
-- Both fixes and receiver_statuses need composite FKs matching the composite PK
-- This migration is idempotent and safe to run multiple times

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

-- Step 2: Add new composite FK constraints pointing to aprs_messages (id, received_at)
-- Use DO block to make this idempotent (skip if constraint already exists)
DO $$
BEGIN
    -- Add fixes FK if it doesn't exist
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'fixes_aprs_message_id_fkey'
          AND conrelid = 'fixes'::regclass
    ) THEN
        ALTER TABLE fixes
            ADD CONSTRAINT fixes_aprs_message_id_fkey
            FOREIGN KEY (aprs_message_id, received_at)
            REFERENCES aprs_messages(id, received_at);
        RAISE NOTICE 'Added composite FK constraint: fixes_aprs_message_id_fkey';
    ELSE
        RAISE NOTICE 'FK constraint fixes_aprs_message_id_fkey already exists, skipping';
    END IF;

    -- Add receiver_statuses FK if it doesn't exist
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'receiver_statuses_aprs_message_id_fkey'
          AND conrelid = 'receiver_statuses'::regclass
    ) THEN
        ALTER TABLE receiver_statuses
            ADD CONSTRAINT receiver_statuses_aprs_message_id_fkey
            FOREIGN KEY (aprs_message_id, received_at)
            REFERENCES aprs_messages(id, received_at);
        RAISE NOTICE 'Added composite FK constraint: receiver_statuses_aprs_message_id_fkey';
    ELSE
        RAISE NOTICE 'FK constraint receiver_statuses_aprs_message_id_fkey already exists, skipping';
    END IF;
END $$;
