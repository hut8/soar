-- Move the 'aprs_type' column data into the source_metadata JSONB column
-- The 'aprs_type' column contains the APRS "to" field (destination callsign) which is
-- specific to OGN/APRS and should be stored in source_metadata

-- Update existing rows to add 'aprs_type' to source_metadata
-- Handle both NULL and non-NULL source_metadata cases
UPDATE fixes
SET source_metadata = CASE
    -- If source_metadata is NULL, create new object with just 'aprs_type'
    WHEN source_metadata IS NULL THEN
        jsonb_build_object('aprs_type', aprs_type)
    -- If source_metadata exists, merge 'aprs_type' into it
    ELSE
        source_metadata || jsonb_build_object('aprs_type', aprs_type)
END;

-- Drop the 'aprs_type' column as it's now in source_metadata
ALTER TABLE fixes DROP COLUMN aprs_type;
