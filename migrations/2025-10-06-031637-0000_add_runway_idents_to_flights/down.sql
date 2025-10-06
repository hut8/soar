-- Remove runway identifier columns from flights table

ALTER TABLE flights
DROP COLUMN IF EXISTS takeoff_runway_ident,
DROP COLUMN IF EXISTS landing_runway_ident;
