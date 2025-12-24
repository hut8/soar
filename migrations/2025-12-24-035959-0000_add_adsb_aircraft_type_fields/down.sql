-- Remove indexes
DROP INDEX IF EXISTS idx_aircraft_category;
DROP INDEX IF EXISTS idx_aircraft_engine_type;
DROP INDEX IF EXISTS idx_aircraft_faa_pia;
DROP INDEX IF EXISTS idx_aircraft_faa_ladd;

-- Remove columns from aircraft table
ALTER TABLE aircraft
DROP COLUMN IF EXISTS aircraft_category,
DROP COLUMN IF EXISTS engine_count,
DROP COLUMN IF EXISTS engine_type,
DROP COLUMN IF EXISTS faa_pia,
DROP COLUMN IF EXISTS faa_ladd;

-- Drop enum types
DROP TYPE IF EXISTS engine_type;
DROP TYPE IF EXISTS aircraft_category;

-- Restore old columns with _adsb suffix
CREATE TYPE aircraft_category_adsb AS ENUM (
    'landplane', 'helicopter', 'balloon', 'amphibian', 'gyroplane',
    'drone', 'powered_parachute', 'rotorcraft', 'seaplane',
    'tiltrotor', 'vtol', 'electric', 'unknown'
);

CREATE TYPE engine_type_adsb AS ENUM (
    'piston', 'jet', 'turbine', 'electric', 'rocket',
    'special', 'none', 'unknown'
);

ALTER TABLE aircraft
ADD COLUMN IF NOT EXISTS aircraft_category_adsb aircraft_category_adsb,
ADD COLUMN IF NOT EXISTS num_engines SMALLINT,
ADD COLUMN IF NOT EXISTS engine_type_adsb engine_type_adsb;
