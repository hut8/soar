-- Drop is_tow_plane column from aircraft_registrations
-- This data is now tracked in devices.aircraft_type_ogn
ALTER TABLE aircraft_registrations DROP COLUMN is_tow_plane;
