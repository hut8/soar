-- Add up migration script here
-- =========================================================
-- Add club_id column to flights table
-- =========================================================

ALTER TABLE flights
ADD COLUMN club_id UUID REFERENCES clubs(id);

-- Create index for club_id lookups
CREATE INDEX flights_club_id_idx ON flights (club_id);
