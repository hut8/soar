-- Fix foreign key constraints to point to partitioned aprs_messages table
-- Both fixes and receiver_statuses need composite FKs matching the composite PK
-- This migration is idempotent and safe to run multiple times

-- Check if composite constraints already exist - if so, this is a no-op
DO $$
DECLARE
    fixes_composite_exists BOOLEAN;
    receiver_composite_exists BOOLEAN;
    constraint_rec RECORD;
BEGIN
    -- Check if composite FK constraints already exist
    SELECT EXISTS (
        SELECT 1 FROM pg_constraint c
        JOIN pg_attribute a1 ON a1.attrelid = c.conrelid AND a1.attnum = c.conkey[1]
        JOIN pg_attribute a2 ON a2.attrelid = c.conrelid AND a2.attnum = c.conkey[2]
        WHERE c.conname = 'fixes_aprs_message_id_fkey'
          AND c.conrelid = 'fixes'::regclass
          AND c.contype = 'f'
          AND array_length(c.conkey, 1) = 2
    ) INTO fixes_composite_exists;

    SELECT EXISTS (
        SELECT 1 FROM pg_constraint c
        JOIN pg_attribute a1 ON a1.attrelid = c.conrelid AND a1.attnum = c.conkey[1]
        JOIN pg_attribute a2 ON a2.attrelid = c.conrelid AND a2.attnum = c.conkey[2]
        WHERE c.conname = 'receiver_statuses_aprs_message_id_fkey'
          AND c.conrelid = 'receiver_statuses'::regclass
          AND c.contype = 'f'
          AND array_length(c.conkey, 1) = 2
    ) INTO receiver_composite_exists;

    -- If both composite constraints exist, we're done (true no-op)
    IF fixes_composite_exists AND receiver_composite_exists THEN
        RAISE NOTICE 'Composite FK constraints already exist - migration is a no-op';
        RETURN;
    END IF;

    -- Otherwise, proceed with migration
    RAISE NOTICE 'Composite FK constraints do not exist - proceeding with migration';

    -- Step 1: Drop old FK constraints (single-column or partition-specific)
    RAISE NOTICE 'Dropping old FK constraints...';

    -- Drop main table constraints
    ALTER TABLE fixes DROP CONSTRAINT IF EXISTS fixes_aprs_message_id_fkey;
    ALTER TABLE receiver_statuses DROP CONSTRAINT IF EXISTS receiver_statuses_aprs_message_id_fkey;

    -- Drop any partition-specific constraints
    FOR constraint_rec IN
        SELECT conname
        FROM pg_constraint
        WHERE conrelid = 'fixes'::regclass
          AND conname LIKE 'fixes_aprs_message_id_%_fkey%'
    LOOP
        EXECUTE 'ALTER TABLE fixes DROP CONSTRAINT IF EXISTS ' || quote_ident(constraint_rec.conname);
        RAISE NOTICE 'Dropped fixes constraint: %', constraint_rec.conname;
    END LOOP;

    FOR constraint_rec IN
        SELECT conname
        FROM pg_constraint
        WHERE conrelid = 'receiver_statuses'::regclass
          AND conname LIKE 'receiver_statuses_aprs_message_id_%_fkey%'
    LOOP
        EXECUTE 'ALTER TABLE receiver_statuses DROP CONSTRAINT IF EXISTS ' || quote_ident(constraint_rec.conname);
        RAISE NOTICE 'Dropped receiver_statuses constraint: %', constraint_rec.conname;
    END LOOP;

    -- Step 2: Add new composite FK constraints
    RAISE NOTICE 'Adding composite FK constraints...';

    ALTER TABLE fixes
        ADD CONSTRAINT fixes_aprs_message_id_fkey
        FOREIGN KEY (aprs_message_id, received_at)
        REFERENCES aprs_messages(id, received_at);
    RAISE NOTICE 'Added composite FK: fixes_aprs_message_id_fkey';

    ALTER TABLE receiver_statuses
        ADD CONSTRAINT receiver_statuses_aprs_message_id_fkey
        FOREIGN KEY (aprs_message_id, received_at)
        REFERENCES aprs_messages(id, received_at);
    RAISE NOTICE 'Added composite FK: receiver_statuses_aprs_message_id_fkey';

    RAISE NOTICE 'Migration complete';
END $$;
