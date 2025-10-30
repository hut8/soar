-- Restore the deprecated tow_release_height_msl column
-- (for rollback purposes only)
ALTER TABLE flights ADD COLUMN tow_release_height_msl INTEGER;
