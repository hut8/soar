-- Rollback: Restore flight_pilots to reference pilots table
-- WARNING: This requires the pilots table to still exist

-- Drop the new foreign key constraint
ALTER TABLE flight_pilots DROP CONSTRAINT IF EXISTS flight_pilots_user_id_fkey;

-- Rename the column back
ALTER TABLE flight_pilots RENAME COLUMN user_id TO pilot_id;

-- Restore the old foreign key constraint
ALTER TABLE flight_pilots ADD CONSTRAINT flight_pilots_pilot_id_fkey
  FOREIGN KEY (pilot_id) REFERENCES pilots(id) ON DELETE CASCADE;

-- Drop the new index
DROP INDEX IF EXISTS idx_flight_pilots_user_id;

-- Recreate the old index
CREATE INDEX idx_flight_pilots_pilot_id ON flight_pilots(pilot_id);

-- Remove comment
COMMENT ON COLUMN flight_pilots.pilot_id IS NULL;
