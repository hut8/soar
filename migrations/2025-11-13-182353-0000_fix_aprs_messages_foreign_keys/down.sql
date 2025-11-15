-- Revert foreign keys back to simple FK (not composite)
-- Note: This reverts to the state after partition migration (single-column FK on fixes only)
-- This migration is idempotent and safe to run multiple times

-- Drop the composite FK constraints
ALTER TABLE fixes DROP CONSTRAINT IF EXISTS fixes_aprs_message_id_fkey;
ALTER TABLE receiver_statuses DROP CONSTRAINT IF EXISTS receiver_statuses_aprs_message_id_fkey;

-- Drop any partition-specific constraints
DO $$
DECLARE
    constraint_rec RECORD;
BEGIN
    FOR constraint_rec IN
        SELECT conname
        FROM pg_constraint
        WHERE conrelid IN ('fixes'::regclass, 'receiver_statuses'::regclass)
          AND conname LIKE '%aprs_message_id_%_fkey%'
    LOOP
        EXECUTE 'ALTER TABLE ' || constraint_rec.conrelid::regclass ||
                ' DROP CONSTRAINT IF EXISTS ' || quote_ident(constraint_rec.conname);
    END LOOP;
END $$;

-- Restore simple FK constraint on fixes (matching state after partition migration)
-- Use DO block to make this idempotent
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'fixes_aprs_message_id_fkey'
          AND conrelid = 'fixes'::regclass
    ) THEN
        ALTER TABLE fixes
            ADD CONSTRAINT fixes_aprs_message_id_fkey
            FOREIGN KEY (aprs_message_id)
            REFERENCES aprs_messages(id) ON DELETE SET NULL;
        RAISE NOTICE 'Added single-column FK constraint: fixes_aprs_message_id_fkey';
    ELSE
        RAISE NOTICE 'FK constraint fixes_aprs_message_id_fkey already exists, skipping';
    END IF;
END $$;

-- Note: receiver_statuses does NOT get a FK constraint in the reverted state
-- (it didn't have one in the partition migration)
