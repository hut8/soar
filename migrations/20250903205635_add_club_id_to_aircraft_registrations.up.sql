-- Add up migration script here
-- =========================================================
-- Add club_id foreign key to aircraft_registrations table
-- =========================================================
ALTER TABLE aircraft_registrations
ADD COLUMN club_id UUID REFERENCES clubs(id);

-- Create index on club_id for faster lookups
CREATE INDEX aircraft_registrations_club_id_idx ON aircraft_registrations (club_id);
