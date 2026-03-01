-- Re-add deprecated columns (nullable, no data restored)
ALTER TABLE flights ADD COLUMN takeoff_location_id UUID REFERENCES locations(id);
ALTER TABLE flights ADD COLUMN landing_location_id UUID REFERENCES locations(id);

CREATE INDEX idx_flights_takeoff_location_id ON flights (takeoff_location_id);
CREATE INDEX idx_flights_landing_location_id ON flights (landing_location_id);

ALTER TABLE spurious_flights ADD COLUMN takeoff_location_id UUID;
ALTER TABLE spurious_flights ADD COLUMN landing_location_id UUID;
