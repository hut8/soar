-- Create aircraft_geofences table
-- Links aircraft to geofences for monitoring
-- When an aircraft linked to a geofence exits the boundary, alerts are sent

CREATE TABLE aircraft_geofences (
    aircraft_id UUID NOT NULL REFERENCES aircraft(id) ON DELETE CASCADE,
    geofence_id UUID NOT NULL REFERENCES geofences(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (aircraft_id, geofence_id)
);

-- Index for geofence queries (list aircraft in a geofence)
CREATE INDEX idx_aircraft_geofences_geofence ON aircraft_geofences (geofence_id);
