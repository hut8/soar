-- Revert foreign keys back to simple FK (not composite)
-- Note: This assumes we're reverting to before the partition migration was completed

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

-- Restore simple FK constraints (pointing to parent table, not composite)
-- This matches the state after the partition migration but before this fix
ALTER TABLE fixes
    ADD CONSTRAINT fixes_aprs_message_id_fkey
    FOREIGN KEY (aprs_message_id)
    REFERENCES aprs_messages(id);

ALTER TABLE receiver_statuses
    ADD CONSTRAINT receiver_statuses_aprs_message_id_fkey
    FOREIGN KEY (aprs_message_id)
    REFERENCES aprs_messages(id);
