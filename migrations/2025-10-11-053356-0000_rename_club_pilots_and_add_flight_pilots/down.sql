-- Drop indexes
DROP INDEX IF EXISTS idx_pilots_club_id;
DROP INDEX IF EXISTS idx_flight_pilots_pilot_id;
DROP INDEX IF EXISTS idx_flight_pilots_flight_id;

-- Drop flight_pilots linking table
DROP TABLE IF EXISTS flight_pilots;

-- Add role-related columns back to pilots table
ALTER TABLE pilots ADD COLUMN is_tow_pilot BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE pilots ADD COLUMN is_instructor BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE pilots ADD COLUMN is_student BOOLEAN NOT NULL DEFAULT false;

-- Remove club_id column from pilots table
ALTER TABLE pilots DROP COLUMN club_id;

-- Rename pilots table back to club_pilots
ALTER TABLE pilots RENAME TO club_pilots;
