-- Add up migration script here
-- =========================================================
-- Flights table for tracking takeoff to landing sequences
-- =========================================================

CREATE TABLE flights (
    id UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    
    -- Aircraft identifier (hex ID like "39D304")
    aircraft_id VARCHAR(10) NOT NULL,
    
    -- Flight times
    takeoff_time TIMESTAMPTZ NOT NULL,
    landing_time TIMESTAMPTZ,
    
    -- Airport identifiers
    departure_airport VARCHAR(10),
    arrival_airport VARCHAR(10),
    
    -- Tow information
    tow_aircraft_id VARCHAR(5) REFERENCES aircraft_registrations(registration_number),
    tow_release_height_msl INTEGER,
    
    -- Database timestamps
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    -- Constraint: if tow_aircraft_id is not null, the referenced aircraft must be a tow plane
    CONSTRAINT valid_tow_plane CHECK (
        tow_aircraft_id IS NULL OR 
        EXISTS (
            SELECT 1 FROM aircraft_registrations 
            WHERE registration_number = tow_aircraft_id 
            AND is_tow_plane = true
        )
    )
);

-- Indexes for common queries
CREATE INDEX flights_aircraft_id_idx ON flights (aircraft_id);
CREATE INDEX flights_takeoff_time_idx ON flights (takeoff_time DESC);
CREATE INDEX flights_landing_time_idx ON flights (landing_time DESC);
CREATE INDEX flights_tow_aircraft_idx ON flights (tow_aircraft_id);

-- Composite index for aircraft flight history
CREATE INDEX flights_aircraft_takeoff_idx ON flights (aircraft_id, takeoff_time DESC);
