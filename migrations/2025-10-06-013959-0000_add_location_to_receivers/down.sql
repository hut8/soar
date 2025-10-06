-- Remove spatial index
DROP INDEX IF EXISTS idx_receivers_location;

-- Remove location columns from receivers table
ALTER TABLE receivers
    DROP COLUMN IF EXISTS location,
    DROP COLUMN IF EXISTS longitude,
    DROP COLUMN IF EXISTS latitude;
