-- Recreate aircraft_type_ogn enum type
CREATE TYPE aircraft_type_ogn AS ENUM (
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

-- Add aircraft_type_ogn column back
ALTER TABLE aircraft ADD COLUMN aircraft_type_ogn aircraft_type_ogn;

-- Recreate aircraft_category_adsb enum type (identical to aircraft_category)
CREATE TYPE aircraft_category_adsb AS ENUM (
    'landplane',
    'helicopter',
    'balloon',
    'amphibian',
    'gyroplane',
    'drone',
    'powered_parachute',
    'rotorcraft',
    'seaplane',
    'tiltrotor',
    'vtol',
    'electric',
    'unknown'
);

-- Add aircraft_category_adsb column back
ALTER TABLE aircraft ADD COLUMN aircraft_category_adsb aircraft_category_adsb;

-- Note: Data migration back is not performed as it would be lossy
-- (multiple aircraft_type_ogn values map to single aircraft_category values)
