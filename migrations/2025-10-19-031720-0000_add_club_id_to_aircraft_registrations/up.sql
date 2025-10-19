-- Add club_id column back to aircraft_registrations table
ALTER TABLE aircraft_registrations ADD COLUMN club_id UUID;

-- Add foreign key constraint to clubs table
ALTER TABLE aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_club_id_fkey
    FOREIGN KEY (club_id)
    REFERENCES clubs(id)
    ON DELETE SET NULL;

-- Create index for club_id lookups
CREATE INDEX idx_aircraft_registrations_club_id ON aircraft_registrations(club_id) WHERE club_id IS NOT NULL;
