-- Reverse: Remove club_id from aircraft_registrations

-- Drop the index
DROP INDEX IF EXISTS idx_aircraft_registrations_club_id;

-- Drop the foreign key constraint
ALTER TABLE aircraft_registrations DROP CONSTRAINT IF EXISTS aircraft_registrations_club_id_fkey;

-- Drop the column
ALTER TABLE aircraft_registrations DROP COLUMN IF EXISTS club_id;
