-- Move the 'via' column data into the source_metadata JSONB column
-- The 'via' column contains APRS routing information (digipeater path) which is
-- specific to OGN/APRS and should be stored in source_metadata

-- Drop the 'via' column without migrating data
ALTER TABLE fixes DROP COLUMN via;
