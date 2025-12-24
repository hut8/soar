-- Create enum for aircraft category (first character of ICAO description from ICAO Doc 8643)
-- ICAO standard values: L, S, A, G, H, T
-- Extended values for non-standard aircraft types: B, D, P, R, V, E
CREATE TYPE aircraft_category AS ENUM (
    'landplane',           -- L (ICAO Doc 8643)
    'helicopter',          -- H (ICAO Doc 8643)
    'balloon',             -- B (extended)
    'amphibian',           -- A (ICAO Doc 8643)
    'gyroplane',           -- G (ICAO Doc 8643 - gyrocopter)
    'drone',               -- D (extended)
    'powered_parachute',   -- P (extended)
    'rotorcraft',          -- R (extended)
    'seaplane',            -- S (ICAO Doc 8643)
    'tiltrotor',           -- T (ICAO Doc 8643)
    'vtol',                -- V (extended)
    'electric',            -- E (extended)
    'unknown'              -- - (unknown/unspecified)
);

-- Create enum for engine type (third character of ICAO description from ICAO Doc 8643)
-- ICAO standard values: J, T, P, E, R
-- Extended values: S (special), - (none)
CREATE TYPE engine_type AS ENUM (
    'piston',              -- P (ICAO Doc 8643)
    'jet',                 -- J (ICAO Doc 8643)
    'turbine',             -- T (ICAO Doc 8643 - turboprop/turboshaft)
    'electric',            -- E (ICAO Doc 8643)
    'rocket',              -- R (ICAO Doc 8643)
    'special',             -- S (extended)
    'none',                -- - (extended - no engine: glider, balloon, ground vehicle)
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
COMMENT ON COLUMN aircraft.aircraft_category IS 'ICAO aircraft category (1st character of ICAO description from ICAO Doc 8643)';
COMMENT ON COLUMN aircraft.engine_count IS 'Number of engines (2nd character of ICAO description from ICAO Doc 8643)';
COMMENT ON COLUMN aircraft.engine_type IS 'ICAO engine type (3rd character of ICAO description from ICAO Doc 8643)';
COMMENT ON COLUMN aircraft.faa_pia IS 'FAA Privacy ICAO Address (PIA) program participation';
COMMENT ON COLUMN aircraft.faa_ladd IS 'FAA Limited Aircraft Data Displayed (LADD) program participation';
