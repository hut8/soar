-- Reverse the renaming from raw_messages back to aprs_messages
-- NOTE: This does not restore the aprs_messages_old table that was dropped

-- 1. Update pg_partman configuration to reference the old template table name
UPDATE partman.part_config
SET template_table = 'partman.template_public_aprs_messages'
WHERE parent_table = 'public.raw_messages'
AND template_table = 'partman.template_public_raw_messages';

-- 2. Rename the default partition back
ALTER TABLE IF EXISTS raw_messages_default
    RENAME TO aprs_messages_default;

-- 3. Rename all existing partition tables back to aprs_messages_*
DO $$
DECLARE
    partition_name TEXT;
BEGIN
    FOR partition_name IN
        SELECT tablename
        FROM pg_tables
        WHERE schemaname = 'public'
        AND tablename LIKE 'raw_messages_p%'
    LOOP
        EXECUTE format('ALTER TABLE %I RENAME TO %I',
            partition_name,
            replace(partition_name, 'raw_messages_', 'aprs_messages_')
        );
    END LOOP;
END $$;

-- 4. Rename the foreign key constraint back
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.table_constraints
        WHERE constraint_name = 'raw_messages_receiver_id_fkey'
        AND table_name = 'raw_messages'
    ) THEN
        ALTER TABLE raw_messages
            RENAME CONSTRAINT raw_messages_receiver_id_fkey TO aprs_messages_receiver_id_fkey;
    END IF;
END $$;

-- 5. Rename the primary key index back
ALTER INDEX IF EXISTS raw_messages_pkey
    RENAME TO aprs_messages_pkey1;

-- 6. Rename the template table back
ALTER TABLE IF EXISTS partman.template_public_raw_messages
    RENAME TO template_public_aprs_messages;
