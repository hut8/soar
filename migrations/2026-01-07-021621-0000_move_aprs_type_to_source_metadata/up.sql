-- The 'aprs_type' column contains the APRS "to" field (destination callsign) which is
-- specific to OGN/APRS and should be stored in source_metadata

-- Drop the 'aprs_type' column as it's now in source_metadata
ALTER TABLE fixes DROP COLUMN aprs_type;
