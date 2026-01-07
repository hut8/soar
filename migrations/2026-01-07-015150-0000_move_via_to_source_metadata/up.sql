-- Move the 'via' column data into the source_metadata JSONB column
-- The 'via' column contains APRS routing information (digipeater path) which is
-- specific to OGN/APRS and should be stored in source_metadata

-- Update existing rows to add 'via' to source_metadata
-- Handle both NULL and non-NULL source_metadata cases
UPDATE fixes
SET source_metadata = CASE
    -- If source_metadata is NULL, create new object with just 'via'
    WHEN source_metadata IS NULL THEN
        jsonb_build_object('via', to_jsonb(via))
    -- If source_metadata exists, merge 'via' into it
    ELSE
        source_metadata || jsonb_build_object('via', to_jsonb(via))
END;

-- Drop the 'via' column as it's now in source_metadata
ALTER TABLE fixes DROP COLUMN via;
