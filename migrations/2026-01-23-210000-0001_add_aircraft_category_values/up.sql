-- Add OGN-specific values to the aircraft_category enum
-- These must be committed before they can be used in subsequent queries

ALTER TYPE aircraft_category ADD VALUE IF NOT EXISTS 'glider';
ALTER TYPE aircraft_category ADD VALUE IF NOT EXISTS 'tow_tug';
ALTER TYPE aircraft_category ADD VALUE IF NOT EXISTS 'paraglider';
ALTER TYPE aircraft_category ADD VALUE IF NOT EXISTS 'hang_glider';
ALTER TYPE aircraft_category ADD VALUE IF NOT EXISTS 'airship';
ALTER TYPE aircraft_category ADD VALUE IF NOT EXISTS 'skydiver_parachute';
ALTER TYPE aircraft_category ADD VALUE IF NOT EXISTS 'static_obstacle';
