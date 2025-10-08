-- Add new towing detection fields to flights table
-- These fields track which towplane towed which glider and when/where release occurred

-- Reference to the towplane device
ALTER TABLE flights ADD COLUMN towed_by_device_id UUID REFERENCES devices(id);

-- Reference to the specific towplane flight
ALTER TABLE flights ADD COLUMN towed_by_flight_id UUID REFERENCES flights(id);

-- Tow release altitude in feet MSL (more precise than old meters field)
ALTER TABLE flights ADD COLUMN tow_release_altitude_msl_ft INTEGER;

-- Timestamp when tow release occurred
ALTER TABLE flights ADD COLUMN tow_release_time TIMESTAMPTZ;

-- Create index for finding glider flights towed by a specific towplane
CREATE INDEX idx_flights_towed_by_device ON flights(towed_by_device_id) WHERE towed_by_device_id IS NOT NULL;
CREATE INDEX idx_flights_towed_by_flight ON flights(towed_by_flight_id) WHERE towed_by_flight_id IS NOT NULL;

-- Note: Keeping old tow_aircraft_id and tow_release_height_msl for backward compatibility
-- New code should use the new fields above
