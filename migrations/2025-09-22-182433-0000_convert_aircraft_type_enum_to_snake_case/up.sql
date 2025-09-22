-- Convert aircraft_type enum from camelCase to snake_case

-- First, create the new enum with snake_case values
CREATE TYPE aircraft_type_new AS ENUM (
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

-- Convert the fixes table column to use the new enum
ALTER TABLE fixes ALTER COLUMN aircraft_type TYPE aircraft_type_new USING (
    CASE aircraft_type::text
        WHEN 'Reserved0' THEN 'reserved_0'::aircraft_type_new
        WHEN 'GliderMotorGlider' THEN 'glider'::aircraft_type_new
        WHEN 'TowTug' THEN 'tow_tug'::aircraft_type_new
        WHEN 'HelicopterGyro' THEN 'helicopter_gyro'::aircraft_type_new
        WHEN 'SkydiverParachute' THEN 'skydiver_parachute'::aircraft_type_new
        WHEN 'DropPlane' THEN 'drop_plane'::aircraft_type_new
        WHEN 'HangGlider' THEN 'hang_glider'::aircraft_type_new
        WHEN 'Paraglider' THEN 'paraglider'::aircraft_type_new
        WHEN 'RecipEngine' THEN 'recip_engine'::aircraft_type_new
        WHEN 'JetTurboprop' THEN 'jet_turboprop'::aircraft_type_new
        WHEN 'Unknown' THEN 'unknown'::aircraft_type_new
        WHEN 'Balloon' THEN 'balloon'::aircraft_type_new
        WHEN 'Airship' THEN 'airship'::aircraft_type_new
        WHEN 'Uav' THEN 'uav'::aircraft_type_new
        WHEN 'ReservedE' THEN 'reserved_e'::aircraft_type_new
        WHEN 'StaticObstacle' THEN 'static_obstacle'::aircraft_type_new
        ELSE NULL
    END
);

-- Drop the old enum and rename the new one
DROP TYPE aircraft_type;
ALTER TYPE aircraft_type_new RENAME TO aircraft_type;
