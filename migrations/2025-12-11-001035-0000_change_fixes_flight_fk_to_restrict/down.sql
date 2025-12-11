-- Revert fixes.flight_id FK back to ON DELETE SET NULL

-- Drop the RESTRICT FK constraint
ALTER TABLE fixes DROP CONSTRAINT IF EXISTS fixes_flight_id_fkey;

-- For each partition, drop the FK constraint
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

-- Restore old FK constraint with ON DELETE SET NULL
ALTER TABLE fixes
    ADD CONSTRAINT fixes_flight_id_fkey
    FOREIGN KEY (flight_id)
    REFERENCES flights(id)
    ON DELETE SET NULL;
