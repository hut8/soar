-- Reverse: drop created_by first (depends on nothing), then status
DROP INDEX IF EXISTS idx_clubs_status;
ALTER TABLE clubs DROP COLUMN IF EXISTS created_by;
ALTER TABLE clubs DROP COLUMN IF EXISTS status;
