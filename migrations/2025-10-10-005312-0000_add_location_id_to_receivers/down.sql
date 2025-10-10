-- Remove index
DROP INDEX IF EXISTS idx_receivers_location_id;

-- Remove location_id column from receivers table
ALTER TABLE receivers
DROP COLUMN IF EXISTS location_id;
