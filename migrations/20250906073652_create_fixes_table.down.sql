-- Add down migration script here
-- =========================================================
-- Drop fixes table and related types
-- =========================================================

DROP TABLE IF EXISTS fixes;
DROP TYPE IF EXISTS adsb_emitter_category;
DROP TYPE IF EXISTS aircraft_type;
DROP TYPE IF EXISTS address_type;
