-- Drop the index first
DROP INDEX IF EXISTS idx_devices_country_code;

-- Remove the country_code column
ALTER TABLE devices
DROP COLUMN country_code;
