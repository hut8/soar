-- =========================================================
-- Reverse the clubs table migration
-- =========================================================

-- Drop index first
DROP INDEX IF EXISTS clubs_name_idx;

-- Drop the clubs table
DROP TABLE IF EXISTS clubs;
