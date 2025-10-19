-- Restore emitter_category to fixes table
ALTER TABLE fixes ADD COLUMN emitter_category adsb_emitter_category;

-- Restore model to fixes table
ALTER TABLE fixes ADD COLUMN model VARCHAR(50);

-- Restore lag to fixes table
ALTER TABLE fixes ADD COLUMN lag INTEGER;

-- Restore raw_packet to fixes table
ALTER TABLE fixes ADD COLUMN raw_packet TEXT NOT NULL DEFAULT '';

-- Restore tow_aircraft_id to flights table
ALTER TABLE flights ADD COLUMN tow_aircraft_id VARCHAR(5);
