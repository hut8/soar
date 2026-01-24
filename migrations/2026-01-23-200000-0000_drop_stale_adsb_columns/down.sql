-- Recreate stale columns and types (for rollback only - these are unused)

-- Recreate enum type (identical to engine_type)
CREATE TYPE engine_type_adsb AS ENUM (
    'piston',
    'jet',
    'turbine',
    'electric',
    'rocket',
    'special',
    'none',
    'unknown'
);

-- Recreate columns
ALTER TABLE aircraft ADD COLUMN IF NOT EXISTS engine_type_adsb engine_type_adsb;
ALTER TABLE aircraft ADD COLUMN IF NOT EXISTS num_engines SMALLINT;

-- Recreate index
CREATE INDEX IF NOT EXISTS idx_aircraft_engine_type_adsb
    ON aircraft (engine_type_adsb)
    WHERE engine_type_adsb IS NOT NULL;
