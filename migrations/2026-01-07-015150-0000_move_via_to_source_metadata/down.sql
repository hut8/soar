-- Reverse the migration: restore the 'via' column from source_metadata

-- Add the 'via' column back
ALTER TABLE fixes ADD COLUMN via TEXT[] NOT NULL DEFAULT '{}';

-- Extract 'via' data from source_metadata back to the column
-- Default to empty array if not present in source_metadata
UPDATE fixes
SET via = CASE
    WHEN source_metadata ? 'via' THEN
        -- Convert JSONB array to TEXT array
        ARRAY(SELECT jsonb_array_elements_text(source_metadata->'via'))
    ELSE
        '{}'
END;

-- Remove 'via' from source_metadata
UPDATE fixes
SET source_metadata = source_metadata - 'via'
WHERE source_metadata ? 'via';

-- Set source_metadata to NULL if it's now an empty object
UPDATE fixes
SET source_metadata = NULL
WHERE source_metadata = '{}'::jsonb;
