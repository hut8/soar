-- Create geofence_exit_events table
-- Records when an aircraft exits a geofence boundary
-- Associated with a specific flight for tracking and reporting

CREATE TABLE geofence_exit_events (
    id UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    geofence_id UUID NOT NULL REFERENCES geofences(id) ON DELETE CASCADE,
    flight_id UUID NOT NULL REFERENCES flights(id) ON DELETE CASCADE,
    aircraft_id UUID NOT NULL REFERENCES aircraft(id) ON DELETE CASCADE,
    -- Exit details
    exit_time TIMESTAMPTZ NOT NULL,
    exit_latitude DOUBLE PRECISION NOT NULL,
    exit_longitude DOUBLE PRECISION NOT NULL,
    exit_altitude_msl_ft INTEGER,
    -- Which layer was exited (captured at time of exit)
    exit_layer_floor_ft INTEGER NOT NULL,
    exit_layer_ceiling_ft INTEGER NOT NULL,
    exit_layer_radius_nm DOUBLE PRECISION NOT NULL,
    -- Email notification tracking
    email_notifications_sent INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for flight queries (get events for a flight)
CREATE INDEX idx_geofence_exit_events_flight ON geofence_exit_events (flight_id);

-- Index for geofence queries (recent events for a geofence)
CREATE INDEX idx_geofence_exit_events_geofence ON geofence_exit_events (geofence_id, exit_time DESC);

-- Index for aircraft queries (events for an aircraft)
CREATE INDEX idx_geofence_exit_events_aircraft ON geofence_exit_events (aircraft_id, exit_time DESC);
