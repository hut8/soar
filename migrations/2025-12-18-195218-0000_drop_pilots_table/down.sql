-- Rollback: Recreate the pilots table
-- WARNING: This does NOT restore pilot data - it only recreates the table structure
-- Data would need to be manually migrated back from users table

CREATE TABLE pilots (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  first_name TEXT NOT NULL,
  last_name TEXT NOT NULL,
  is_licensed BOOLEAN NOT NULL DEFAULT FALSE,
  is_instructor BOOLEAN NOT NULL DEFAULT FALSE,
  is_tow_pilot BOOLEAN NOT NULL DEFAULT FALSE,
  is_examiner BOOLEAN NOT NULL DEFAULT FALSE,
  club_id UUID REFERENCES clubs(id) ON DELETE SET NULL,
  user_id UUID REFERENCES users(id) ON DELETE SET NULL,
  deleted_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Recreate indexes
CREATE INDEX idx_pilots_club_id ON pilots(club_id);
CREATE INDEX idx_pilots_deleted_at ON pilots(deleted_at) WHERE deleted_at IS NULL;
CREATE INDEX idx_club_pilots_name ON pilots(last_name, first_name);

-- Recreate trigger for updated_at
CREATE TRIGGER set_pilots_updated_at
  BEFORE UPDATE ON pilots
  FOR EACH ROW
  EXECUTE FUNCTION update_updated_at();

-- Add comments
COMMENT ON TABLE pilots IS 'DEPRECATED: Pilot data has been migrated to users table';
