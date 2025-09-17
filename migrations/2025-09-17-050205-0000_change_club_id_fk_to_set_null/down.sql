-- Revert foreign key constraints for club_id columns back to original state
-- This removes the ON DELETE SET NULL behavior

-- Drop the modified foreign key constraints
ALTER TABLE aircraft_registrations DROP CONSTRAINT IF EXISTS aircraft_registrations_club_id_fkey;
ALTER TABLE fixes DROP CONSTRAINT IF EXISTS fixes_club_id_fkey;
ALTER TABLE flights DROP CONSTRAINT IF EXISTS flights_club_id_fkey;

-- Re-add foreign key constraints without ON DELETE SET NULL (original behavior)
ALTER TABLE aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_club_id_fkey
    FOREIGN KEY (club_id) REFERENCES clubs(id);

ALTER TABLE fixes
    ADD CONSTRAINT fixes_club_id_fkey
    FOREIGN KEY (club_id) REFERENCES clubs(id);

ALTER TABLE flights
    ADD CONSTRAINT flights_club_id_fkey
    FOREIGN KEY (club_id) REFERENCES clubs(id);

-- Note: users.club_id should remain unchanged as it already had ON DELETE SET NULL in the original schema