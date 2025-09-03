-- =========================================================
-- Reverse the club_id foreign key addition to aircraft_registrations
-- =========================================================

-- Drop index first
DROP INDEX IF EXISTS aircraft_registrations_club_id_idx;

-- Drop the club_id column
ALTER TABLE aircraft_registrations DROP COLUMN IF EXISTS club_id;
