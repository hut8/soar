-- Add country_code column to devices table
-- Two-letter country code (ISO 3166-1 alpha-2), nullable
-- Will be populated from ICAO addresses using flydent crate
ALTER TABLE devices
ADD COLUMN country_code CHAR(2) NULL;

-- Create index for filtering/querying by country
CREATE INDEX idx_devices_country_code ON devices(country_code);
