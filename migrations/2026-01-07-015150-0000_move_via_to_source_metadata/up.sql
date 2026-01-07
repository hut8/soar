-- The 'via' column contains APRS routing information (digipeater path) which is
-- specific to OGN/APRS and should be stored in source_metadata

-- Drop the 'via' column as it's now in source_metadata
ALTER TABLE fixes DROP COLUMN via;
