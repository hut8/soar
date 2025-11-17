-- Drop APRS-specific columns that have been consolidated into source_metadata JSONB
-- This frees up 5 columns (net gain of 4 columns after adding source_metadata)
-- Allows adding ADS-B-specific fields without hitting PostgreSQL's 32-column limit

ALTER TABLE fixes DROP COLUMN IF EXISTS snr_db;
ALTER TABLE fixes DROP COLUMN IF EXISTS bit_errors_corrected;
ALTER TABLE fixes DROP COLUMN IF EXISTS freq_offset_khz;
ALTER TABLE fixes DROP COLUMN IF EXISTS gnss_horizontal_resolution;
ALTER TABLE fixes DROP COLUMN IF EXISTS gnss_vertical_resolution;

-- Column count before: 30/32
-- Column count after: 26/32 (freed 4 columns for future use)
