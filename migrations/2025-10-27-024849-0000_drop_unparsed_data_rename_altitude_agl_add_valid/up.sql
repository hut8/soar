-- Drop unparsed_data column from fixes table (it's already in aprs_messages)
ALTER TABLE fixes DROP COLUMN IF EXISTS unparsed_data;

-- Rename altitude_agl to altitude_agl_feet for consistency (idempotent)
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'fixes' AND column_name = 'altitude_agl'
    ) THEN
        ALTER TABLE fixes RENAME COLUMN altitude_agl TO altitude_agl_feet;
    END IF;
END $$;

-- Add altitude_agl_valid boolean column
-- Set to true if altitude_agl_feet is not null (meaning it was already calculated)
-- Default to false for new rows
ALTER TABLE fixes
ADD COLUMN IF NOT EXISTS altitude_agl_valid BOOLEAN NOT NULL DEFAULT false;

-- Set altitude_agl_valid to true for all existing rows where altitude_agl_feet is not null
UPDATE fixes
SET altitude_agl_valid = true
WHERE altitude_agl_feet IS NOT NULL AND altitude_agl_valid = false;

-- Rename the index to match the new column name (idempotent)
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM pg_indexes
        WHERE indexname = 'idx_fixes_altitude_agl'
    ) THEN
        ALTER INDEX idx_fixes_altitude_agl RENAME TO idx_fixes_altitude_agl_feet;
    END IF;
END $$;
