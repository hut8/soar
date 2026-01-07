-- Reverse the migration: restore the 'aprs_type' column from source_metadata

-- Add the 'aprs_type' column back
ALTER TABLE fixes ADD COLUMN aprs_type VARCHAR(9) NOT NULL DEFAULT '';

-- Extract 'aprs_type' data from source_metadata back to the column
-- Default to empty string if not present in source_metadata
UPDATE fixes
SET aprs_type = COALESCE(source_metadata->>'aprs_type', '');

-- Remove 'aprs_type' from source_metadata
UPDATE fixes
SET source_metadata = source_metadata - 'aprs_type'
WHERE source_metadata ? 'aprs_type';

-- Set source_metadata to NULL if it's now an empty object
UPDATE fixes
SET source_metadata = NULL
WHERE source_metadata = '{}'::jsonb;
