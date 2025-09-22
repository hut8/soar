-- Combine Reserved0 and ReservedE aircraft type variants into a single Reserved variant

-- First, create the new enum with the combined Reserved variant
CREATE TYPE aircraft_type_new AS ENUM (
    'reserved',
    'glider',
    'tow_tug',
    'helicopter_gyro',
    'skydiver_parachute',
    'drop_plane',
    'hang_glider',
    'paraglider',
    'recip_engine',
    'jet_turboprop',
    'unknown',
    'balloon',
    'airship',
    'uav',
    'static_obstacle'
);

-- Update any existing data to map both Reserved0 and ReservedE to reserved
-- Also update all other values to snake_case as they should have been updated already
ALTER TABLE fixes ALTER COLUMN aircraft_type TYPE aircraft_type_new USING (
    CASE aircraft_type::text
        WHEN 'Reserved0' THEN 'reserved'::aircraft_type_new
        WHEN 'ReservedE' THEN 'reserved'::aircraft_type_new
        WHEN 'glider' THEN 'glider'::aircraft_type_new
        WHEN 'tow_tug' THEN 'tow_tug'::aircraft_type_new
        WHEN 'helicopter_gyro' THEN 'helicopter_gyro'::aircraft_type_new
        WHEN 'skydiver_parachute' THEN 'skydiver_parachute'::aircraft_type_new
        WHEN 'drop_plane' THEN 'drop_plane'::aircraft_type_new
        WHEN 'hang_glider' THEN 'hang_glider'::aircraft_type_new
        WHEN 'paraglider' THEN 'paraglider'::aircraft_type_new
        WHEN 'recip_engine' THEN 'recip_engine'::aircraft_type_new
        WHEN 'jet_turboprop' THEN 'jet_turboprop'::aircraft_type_new
        WHEN 'unknown' THEN 'unknown'::aircraft_type_new
        WHEN 'balloon' THEN 'balloon'::aircraft_type_new
        WHEN 'airship' THEN 'airship'::aircraft_type_new
        WHEN 'uav' THEN 'uav'::aircraft_type_new
        WHEN 'static_obstacle' THEN 'static_obstacle'::aircraft_type_new
        ELSE NULL
    END
);

-- Drop the old enum and rename the new one
DROP TYPE aircraft_type;
ALTER TYPE aircraft_type_new RENAME TO aircraft_type;
