-- Restore length limits to VARCHAR columns in devices table
-- Convert TEXT back to VARCHAR(8)

ALTER TABLE devices
ALTER COLUMN icao_model_code TYPE VARCHAR(8);

COMMENT ON COLUMN devices.icao_model_code IS 'ICAO aircraft model code (max 8 characters)';
