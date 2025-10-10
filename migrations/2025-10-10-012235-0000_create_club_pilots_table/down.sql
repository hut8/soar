-- Drop club_pilots table
DROP TRIGGER IF EXISTS set_club_pilots_updated_at ON club_pilots;
DROP INDEX IF EXISTS idx_club_pilots_name;
DROP TABLE IF EXISTS club_pilots;
