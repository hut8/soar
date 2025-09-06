-- Add down migration script here
-- Remove is_tow_plane column from aircraft_registrations table
ALTER TABLE aircraft_registrations 
DROP COLUMN IF EXISTS is_tow_plane;
