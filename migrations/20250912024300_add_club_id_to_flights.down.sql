-- Add down migration script here
-- =========================================================
-- Remove club_id column from flights table
-- =========================================================

-- Drop the index first
DROP INDEX IF EXISTS flights_club_id_idx;

-- Remove the club_id column
ALTER TABLE flights DROP COLUMN club_id;
