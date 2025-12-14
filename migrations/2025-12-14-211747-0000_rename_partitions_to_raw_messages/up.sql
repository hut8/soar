-- Rename partitions and related objects from aprs_messages to raw_messages
-- This migration ensures consistency after the table was renamed from aprs_messages to raw_messages

-- 1. Rename the template table used by pg_partman
ALTER TABLE IF EXISTS partman.template_public_aprs_messages
    RENAME TO template_public_raw_messages;

-- 2. Rename the primary key index on the parent table
ALTER INDEX IF EXISTS aprs_messages_pkey1
    RENAME TO raw_messages_pkey;

-- 3. Rename the foreign key constraint on the parent table
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.table_constraints
        WHERE constraint_name = 'aprs_messages_receiver_id_fkey'
        AND table_name = 'raw_messages'
    ) THEN
        ALTER TABLE raw_messages
            RENAME CONSTRAINT aprs_messages_receiver_id_fkey TO raw_messages_receiver_id_fkey;
    END IF;
END $$;

-- 4. Rename all existing partition tables
-- This uses dynamic SQL to rename any aprs_messages_* partitions that exist
DO $$
DECLARE
    partition_name TEXT;
BEGIN
    FOR partition_name IN
        SELECT tablename
        FROM pg_tables
        WHERE schemaname = 'public'
        AND tablename LIKE 'aprs_messages_p%'
    LOOP
        EXECUTE format('ALTER TABLE %I RENAME TO %I',
            partition_name,
            replace(partition_name, 'aprs_messages_', 'raw_messages_')
        );
    END LOOP;
END $$;

-- 5. Rename the default partition if it exists
ALTER TABLE IF EXISTS aprs_messages_default
    RENAME TO raw_messages_default;

-- 6. Update pg_partman configuration to reference the new template table
UPDATE partman.part_config
SET template_table = 'partman.template_public_raw_messages'
WHERE parent_table = 'public.raw_messages'
AND template_table = 'partman.template_public_aprs_messages';

-- 7. Drop the old aprs_messages_old table if it exists
DROP TABLE IF EXISTS aprs_messages_old CASCADE;
