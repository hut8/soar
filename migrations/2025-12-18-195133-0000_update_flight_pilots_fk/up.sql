-- Update flight_pilots table to reference users instead of pilots
-- This completes the consolidation by updating the foreign key relationship

-- Drop the old foreign key constraint that references pilots table
ALTER TABLE flight_pilots DROP CONSTRAINT flight_pilots_pilot_id_fkey;

-- Rename the column for semantic clarity (pilot_id -> user_id)
ALTER TABLE flight_pilots RENAME COLUMN pilot_id TO user_id;

-- Add new foreign key constraint that references users table
ALTER TABLE flight_pilots ADD CONSTRAINT flight_pilots_user_id_fkey
  FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

-- Drop the old index
DROP INDEX IF EXISTS idx_flight_pilots_pilot_id;

-- Create new index on user_id for efficient lookups
CREATE INDEX idx_flight_pilots_user_id ON flight_pilots(user_id);

-- Update comment to reflect the new relationship
COMMENT ON COLUMN flight_pilots.user_id IS 'References users table (formerly pilot_id). Links flights to users with pilot qualifications.';
