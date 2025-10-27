-- Drop altitude_agl_valid column
ALTER TABLE fixes DROP COLUMN IF EXISTS altitude_agl_valid;

-- Rename the index back to the old name (idempotent)
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM pg_indexes
        WHERE indexname = 'idx_fixes_altitude_agl_feet'
    ) THEN
        ALTER INDEX idx_fixes_altitude_agl_feet RENAME TO idx_fixes_altitude_agl;
    END IF;
END $$;

-- Rename altitude_agl_feet back to altitude_agl (idempotent)
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'fixes' AND column_name = 'altitude_agl_feet'
    ) THEN
        ALTER TABLE fixes RENAME COLUMN altitude_agl_feet TO altitude_agl;
    END IF;
END $$;

-- Re-add unparsed_data column (will be empty)
ALTER TABLE fixes ADD COLUMN IF NOT EXISTS unparsed_data VARCHAR;
