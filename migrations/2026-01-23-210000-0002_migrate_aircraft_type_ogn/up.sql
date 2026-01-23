-- Migrate aircraft_type_ogn data to aircraft_category and clean up old types
-- This must run AFTER the enum values have been added and committed

-- 1. Migrate aircraft_type_ogn data to aircraft_category (only where aircraft_category IS NULL)
UPDATE aircraft
SET aircraft_category = CASE aircraft_type_ogn::text
    WHEN 'glider' THEN 'glider'::aircraft_category
    WHEN 'tow_tug' THEN 'tow_tug'::aircraft_category
    -- OGN's 'helicopter_gyro' covers both helicopters and gyrocopters
    WHEN 'helicopter_gyro' THEN 'rotorcraft'::aircraft_category
    WHEN 'skydiver_parachute' THEN 'skydiver_parachute'::aircraft_category
    WHEN 'drop_plane' THEN 'landplane'::aircraft_category
    WHEN 'hang_glider' THEN 'hang_glider'::aircraft_category
    WHEN 'paraglider' THEN 'paraglider'::aircraft_category
    WHEN 'recip_engine' THEN 'landplane'::aircraft_category
    WHEN 'jet_turboprop' THEN 'landplane'::aircraft_category
    WHEN 'balloon' THEN 'balloon'::aircraft_category
    WHEN 'airship' THEN 'airship'::aircraft_category
    WHEN 'uav' THEN 'drone'::aircraft_category
    WHEN 'unknown' THEN 'unknown'::aircraft_category
    WHEN 'reserved' THEN 'unknown'::aircraft_category
    WHEN 'static_obstacle' THEN 'static_obstacle'::aircraft_category
END
WHERE aircraft_category IS NULL AND aircraft_type_ogn IS NOT NULL;

-- 2. Drop aircraft_type_ogn column
ALTER TABLE aircraft DROP COLUMN IF EXISTS aircraft_type_ogn;

-- 3. Drop aircraft_type_ogn enum type
DROP TYPE IF EXISTS aircraft_type_ogn;

-- 4. Drop aircraft_category_adsb column (empty, unused)
ALTER TABLE aircraft DROP COLUMN IF EXISTS aircraft_category_adsb;

-- 5. Drop aircraft_category_adsb enum type (duplicate of aircraft_category)
DROP TYPE IF EXISTS aircraft_category_adsb;
