-- Change fixes.flight_id FK from ON DELETE SET NULL to ON DELETE RESTRICT
-- This prevents accidental orphaning of fixes when flights are deleted

-- Drop the old FK constraint
ALTER TABLE fixes DROP CONSTRAINT IF EXISTS fixes_flight_id_fkey;

-- For each partition, drop the old FK constraint
DO $$
DECLARE
    partition_name TEXT;
BEGIN
    FOR partition_name IN
        SELECT tablename
        FROM pg_tables
        WHERE schemaname = 'public'
          AND tablename LIKE 'fixes_p%'
    LOOP
        EXECUTE format('ALTER TABLE %I DROP CONSTRAINT IF EXISTS fixes_flight_id_fkey', partition_name);
    END LOOP;
END $$;

-- Add new FK constraint with ON DELETE RESTRICT
ALTER TABLE fixes
    ADD CONSTRAINT fixes_flight_id_fkey
    FOREIGN KEY (flight_id)
    REFERENCES flights(id)
    ON DELETE RESTRICT;

-- Partitions will inherit the constraint from the parent table
