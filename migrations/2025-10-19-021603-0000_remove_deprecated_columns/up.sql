-- Remove tow_aircraft_id from flights table (replaced by towed_by_device_id and towed_by_flight_id)
ALTER TABLE flights DROP COLUMN IF EXISTS tow_aircraft_id;

-- Remove raw_packet from fixes table (data moved to aprs_messages table)
ALTER TABLE fixes DROP COLUMN IF EXISTS raw_packet;

-- Remove lag from fixes table (not needed, can be calculated from timestamp and received_at)
ALTER TABLE fixes DROP COLUMN IF EXISTS lag;

-- Remove model from fixes table (moving to devices.icao_model_code)
ALTER TABLE fixes DROP COLUMN IF EXISTS model;

-- Remove emitter_category from fixes table (moving to devices.adsb_emitter_category)
ALTER TABLE fixes DROP COLUMN IF EXISTS emitter_category;
