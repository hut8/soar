-- Remove length limits from VARCHAR columns in devices table
-- Convert VARCHAR(8) to TEXT to allow unlimited length

ALTER TABLE devices
ALTER COLUMN icao_model_code TYPE TEXT;

COMMENT ON COLUMN devices.icao_model_code IS 'ICAO aircraft model code (no length limit)';
