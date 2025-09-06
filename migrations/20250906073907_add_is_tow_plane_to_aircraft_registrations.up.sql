-- Add up migration script here
-- Add is_tow_plane column to aircraft_registrations table
ALTER TABLE aircraft_registrations 
ADD COLUMN is_tow_plane BOOLEAN;
