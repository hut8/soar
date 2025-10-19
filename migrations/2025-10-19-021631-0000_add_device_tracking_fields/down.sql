-- Drop index
DROP INDEX IF EXISTS idx_devices_icao_model_code;

-- Remove adsb_emitter_category from devices table
ALTER TABLE devices DROP COLUMN IF EXISTS adsb_emitter_category;

-- Remove icao_model_code from devices table
ALTER TABLE devices DROP COLUMN IF EXISTS icao_model_code;
