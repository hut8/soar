-- Restore is_tow_plane column to aircraft_registrations
ALTER TABLE aircraft_registrations ADD COLUMN is_tow_plane BOOLEAN;
