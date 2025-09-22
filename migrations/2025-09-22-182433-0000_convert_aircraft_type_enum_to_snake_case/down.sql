-- Revert aircraft_type enum from snake_case back to camelCase

-- First, create the old enum with camelCase values
CREATE TYPE aircraft_type_old AS ENUM (
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

-- Convert the fixes table column to use the old enum
ALTER TABLE fixes ALTER COLUMN aircraft_type TYPE aircraft_type_old USING (
    CASE aircraft_type::text
        WHEN 'reserved_0' THEN 'Reserved0'::aircraft_type_old
        WHEN 'glider' THEN 'GliderMotorGlider'::aircraft_type_old
        WHEN 'tow_tug' THEN 'TowTug'::aircraft_type_old
        WHEN 'helicopter_gyro' THEN 'HelicopterGyro'::aircraft_type_old
        WHEN 'skydiver_parachute' THEN 'SkydiverParachute'::aircraft_type_old
        WHEN 'drop_plane' THEN 'DropPlane'::aircraft_type_old
        WHEN 'hang_glider' THEN 'HangGlider'::aircraft_type_old
        WHEN 'paraglider' THEN 'Paraglider'::aircraft_type_old
        WHEN 'recip_engine' THEN 'RecipEngine'::aircraft_type_old
        WHEN 'jet_turboprop' THEN 'JetTurboprop'::aircraft_type_old
        WHEN 'unknown' THEN 'Unknown'::aircraft_type_old
        WHEN 'balloon' THEN 'Balloon'::aircraft_type_old
        WHEN 'airship' THEN 'Airship'::aircraft_type_old
        WHEN 'uav' THEN 'Uav'::aircraft_type_old
        WHEN 'reserved_e' THEN 'ReservedE'::aircraft_type_old
        WHEN 'static_obstacle' THEN 'StaticObstacle'::aircraft_type_old
        ELSE NULL
    END
);

-- Drop the new enum and rename the old one
DROP TYPE aircraft_type;
ALTER TYPE aircraft_type_old RENAME TO aircraft_type;
