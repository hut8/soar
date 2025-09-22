-- Revert the combined Reserved variant back to Reserved0 and ReservedE
-- Note: This will map all 'reserved' values back to 'reserved_0' since we can't distinguish between them

-- First, create the old enum with separate Reserved0 and ReservedE variants
CREATE TYPE aircraft_type_old AS ENUM (
    'reserved_0',
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
    'reserved_e',
    'static_obstacle'
);

-- Convert the fixes table column to use the old enum
-- All 'reserved' values will become 'reserved_0' (we can't distinguish the original source)
ALTER TABLE fixes ALTER COLUMN aircraft_type TYPE aircraft_type_old USING (
    CASE aircraft_type::text
        WHEN 'reserved' THEN 'reserved_0'::aircraft_type_old
        WHEN 'glider' THEN 'glider'::aircraft_type_old
        WHEN 'tow_tug' THEN 'tow_tug'::aircraft_type_old
        WHEN 'helicopter_gyro' THEN 'helicopter_gyro'::aircraft_type_old
        WHEN 'skydiver_parachute' THEN 'skydiver_parachute'::aircraft_type_old
        WHEN 'drop_plane' THEN 'drop_plane'::aircraft_type_old
        WHEN 'hang_glider' THEN 'hang_glider'::aircraft_type_old
        WHEN 'paraglider' THEN 'paraglider'::aircraft_type_old
        WHEN 'recip_engine' THEN 'recip_engine'::aircraft_type_old
        WHEN 'jet_turboprop' THEN 'jet_turboprop'::aircraft_type_old
        WHEN 'unknown' THEN 'unknown'::aircraft_type_old
        WHEN 'balloon' THEN 'balloon'::aircraft_type_old
        WHEN 'airship' THEN 'airship'::aircraft_type_old
        WHEN 'uav' THEN 'uav'::aircraft_type_old
        WHEN 'static_obstacle' THEN 'static_obstacle'::aircraft_type_old
        ELSE NULL
    END
);

-- Drop the new enum and rename the old one
DROP TYPE aircraft_type;
ALTER TYPE aircraft_type_old RENAME TO aircraft_type;
