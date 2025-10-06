-- Add pilot information and radio frequency columns to devices table

ALTER TABLE devices
ADD COLUMN IF NOT EXISTS frequency_mhz NUMERIC(6, 3),
ADD COLUMN IF NOT EXISTS pilot_name TEXT,
ADD COLUMN IF NOT EXISTS home_base_airport_ident TEXT;

COMMENT ON COLUMN devices.frequency_mhz IS 'Radio frequency in MHz (e.g., 123.450)';
COMMENT ON COLUMN devices.pilot_name IS 'Name of the pilot associated with this device';
COMMENT ON COLUMN devices.home_base_airport_ident IS 'ICAO or IATA identifier of the pilot''s home base airport';
