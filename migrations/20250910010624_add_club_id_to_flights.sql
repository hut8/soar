-- Add club_id foreign key to flights table
-- This links flights to the club that owns the aircraft

-- Add the club_id column
ALTER TABLE flights 
ADD COLUMN club_id UUID REFERENCES clubs(id);

-- Create index for efficient club-based flight queries
CREATE INDEX flights_club_id_idx ON flights (club_id);

-- Create composite index for club flight history
CREATE INDEX flights_club_takeoff_idx ON flights (club_id, takeoff_time DESC);