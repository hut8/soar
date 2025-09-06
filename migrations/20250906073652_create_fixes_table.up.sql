-- Add up migration script here
-- =========================================================
-- Fixes table for position reports from APRS/OGN
-- =========================================================

-- Address types for aircraft identification
CREATE TYPE address_type AS ENUM (
    'Unknown',
    'Icao', 
    'Flarm',
    'OgnTracker'
);

-- Aircraft types from OGN
CREATE TYPE aircraft_type AS ENUM (
    'Reserved0',
    'GliderMotorGlider',
    'TowTug',
    'HelicopterGyro',
    'SkydiverParachute',
    'DropPlane',
    'HangGlider',
    'Paraglider',
    'RecipEngine',
    'JetTurboprop',
    'Unknown',
    'Balloon',
    'Airship',
    'Uav',
    'ReservedE',
    'StaticObstacle'
);

-- ADSB emitter categories
CREATE TYPE adsb_emitter_category AS ENUM (
    'A0', 'A1', 'A2', 'A3', 'A4', 'A5', 'A6', 'A7',
    'B0', 'B1', 'B2', 'B3', 'B4', 'B6', 'B7',
    'C0', 'C1', 'C2', 'C3', 'C4', 'C5'
);

-- Main fixes table
CREATE TABLE fixes (
    id UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    
    -- APRS packet header information
    source VARCHAR(9) NOT NULL,           -- Source callsign (max 9 chars per APRS spec)
    destination VARCHAR(9) NOT NULL,      -- Destination callsign
    via TEXT[],                          -- Via/relay stations array
    
    -- Raw APRS packet for debugging/audit
    raw_packet TEXT NOT NULL,
    
    -- Timestamp
    timestamp TIMESTAMPTZ NOT NULL,
    
    -- Position
    latitude DOUBLE PRECISION NOT NULL,
    longitude DOUBLE PRECISION NOT NULL,
    location GEOGRAPHY(POINT, 4326) GENERATED ALWAYS AS (ST_Point(longitude, latitude)::geography) STORED,
    altitude_feet INTEGER,
    
    -- Aircraft identification
    aircraft_id VARCHAR(10),              -- Hex aircraft ID (e.g., "39D304")
    address INTEGER,                      -- Raw address from OGN parameters
    address_type address_type,
    aircraft_type aircraft_type,
    
    -- Flight information
    flight_number VARCHAR(20),
    emitter_category adsb_emitter_category,
    registration VARCHAR(10),
    model VARCHAR(50),
    squawk VARCHAR(4),
    
    -- Performance data
    ground_speed_knots REAL,
    track_degrees REAL CHECK (track_degrees >= 0 AND track_degrees < 360),
    climb_fpm INTEGER,
    turn_rate_rot REAL,
    
    -- Signal quality
    snr_db REAL,
    bit_errors_corrected INTEGER,
    freq_offset_khz REAL,
    
    -- Club association (nullable foreign key)
    club_id UUID REFERENCES clubs(id),
    
    -- Timestamps
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for common queries
CREATE INDEX fixes_timestamp_idx ON fixes (timestamp DESC);
CREATE INDEX fixes_aircraft_id_idx ON fixes (aircraft_id);
CREATE INDEX fixes_registration_idx ON fixes (registration);
CREATE INDEX fixes_source_idx ON fixes (source);
CREATE INDEX fixes_location_idx ON fixes USING GIST (location);
CREATE INDEX fixes_club_id_idx ON fixes (club_id);

-- Composite index for aircraft tracking queries
CREATE INDEX fixes_aircraft_timestamp_idx ON fixes (aircraft_id, timestamp DESC);
CREATE INDEX fixes_registration_timestamp_idx ON fixes (registration, timestamp DESC);
