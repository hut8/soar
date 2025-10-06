-- Add runway identifier columns to flights table for tracking
-- which specific runway was used for takeoff and landing

ALTER TABLE flights
ADD COLUMN IF NOT EXISTS takeoff_runway_ident TEXT,
ADD COLUMN IF NOT EXISTS landing_runway_ident TEXT;

COMMENT ON COLUMN flights.takeoff_runway_ident IS 'Runway identifier used for takeoff (e.g., "09L", "27R")';
COMMENT ON COLUMN flights.landing_runway_ident IS 'Runway identifier used for landing (e.g., "09L", "27R")';
