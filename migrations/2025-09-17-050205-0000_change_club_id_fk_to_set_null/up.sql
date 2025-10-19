-- Change foreign key constraints for club_id columns to SET NULL on deletion
-- This allows deleting clubs without having to cascade delete or restrict the operation

-- Drop existing foreign key constraints
ALTER TABLE aircraft_registrations DROP CONSTRAINT IF EXISTS aircraft_registrations_club_id_fkey;
ALTER TABLE fixes DROP CONSTRAINT IF EXISTS fixes_club_id_fkey;
ALTER TABLE flights DROP CONSTRAINT IF EXISTS flights_club_id_fkey;

-- Re-add foreign key constraints with ON DELETE SET NULL
ALTER TABLE aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_club_id_fkey
    FOREIGN KEY (club_id) REFERENCES clubs(id) ON DELETE SET NULL;

ALTER TABLE fixes
    ADD CONSTRAINT fixes_club_id_fkey
    FOREIGN KEY (club_id) REFERENCES clubs(id) ON DELETE SET NULL;

ALTER TABLE flights
    ADD CONSTRAINT flights_club_id_fkey
    FOREIGN KEY (club_id) REFERENCES clubs(id) ON DELETE SET NULL;

-- Note: users.club_id already has ON DELETE SET NULL, so no change needed
