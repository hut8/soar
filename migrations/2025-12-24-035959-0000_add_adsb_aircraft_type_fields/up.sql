-- Create enum for aircraft category (first character of short_type)
CREATE TYPE aircraft_category AS ENUM (
    'landplane',           -- L
    'helicopter',          -- H
    'balloon',             -- B
    'amphibian',           -- A
    'gyroplane',           -- G
    'drone',               -- D
    'powered_parachute',   -- P
    'rotorcraft',          -- R
    'seaplane',            -- S
    'tiltrotor',           -- T
    'vtol',                -- V
    'electric',            -- E (rarely used for category)
    'unknown'              -- - (unknown/unspecified)
);

-- Create enum for engine type (third character of short_type)
CREATE TYPE engine_type AS ENUM (
    'piston',              -- P
    'jet',                 -- J
    'turbine',             -- T (turboprop/turboshaft)
    'electric',            -- E
    'rocket',              -- R
    'special',             -- S
    'none',                -- - (no engine: glider, balloon, ground vehicle)
    'unknown'              -- unknown/unspecified
);

-- Add new columns to aircraft table
ALTER TABLE aircraft
ADD COLUMN IF NOT EXISTS aircraft_category aircraft_category,
ADD COLUMN IF NOT EXISTS engine_count SMALLINT,
ADD COLUMN IF NOT EXISTS engine_type engine_type,
ADD COLUMN IF NOT EXISTS faa_pia BOOLEAN,
ADD COLUMN IF NOT EXISTS faa_ladd BOOLEAN;

-- Create indexes for common queries
CREATE INDEX IF NOT EXISTS idx_aircraft_category ON aircraft (aircraft_category) WHERE aircraft_category IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_aircraft_engine_type ON aircraft (engine_type) WHERE engine_type IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_aircraft_faa_pia ON aircraft (faa_pia) WHERE faa_pia = true;
CREATE INDEX IF NOT EXISTS idx_aircraft_faa_ladd ON aircraft (faa_ladd) WHERE faa_ladd = true;

-- Drop old columns with _adsb suffix if they exist
ALTER TABLE aircraft
DROP COLUMN IF EXISTS aircraft_category_adsb,
DROP COLUMN IF EXISTS num_engines,
DROP COLUMN IF EXISTS engine_type_adsb;

-- Add comments
COMMENT ON COLUMN aircraft.aircraft_category IS 'Aircraft category from ADS-B Exchange short_type (1st character)';
COMMENT ON COLUMN aircraft.engine_count IS 'Number of engines from ADS-B Exchange short_type (2nd character)';
COMMENT ON COLUMN aircraft.engine_type IS 'Engine type from ADS-B Exchange short_type (3rd character)';
COMMENT ON COLUMN aircraft.faa_pia IS 'Privacy ICAO Address program participation (from ADS-B Exchange)';
COMMENT ON COLUMN aircraft.faa_ladd IS 'Limited Aircraft Data Displayed program participation (from ADS-B Exchange)';
