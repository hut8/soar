-- Add pilot qualification fields to users table
-- These fields migrate from the pilots table to enable unified user/pilot management

-- Add pilot qualification columns
ALTER TABLE users ADD COLUMN is_licensed BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE users ADD COLUMN is_instructor BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE users ADD COLUMN is_tow_pilot BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE users ADD COLUMN is_examiner BOOLEAN NOT NULL DEFAULT FALSE;

-- Add soft delete support (consistent with pilots table)
ALTER TABLE users ADD COLUMN deleted_at TIMESTAMPTZ;

-- Add index for efficient queries on non-deleted users
CREATE INDEX idx_users_deleted_at ON users(deleted_at) WHERE deleted_at IS NULL;

-- Add comments documenting the pilot qualification fields
COMMENT ON COLUMN users.is_licensed IS 'Indicates if user is a licensed pilot';
COMMENT ON COLUMN users.is_instructor IS 'Indicates if user is certified as a flight instructor';
COMMENT ON COLUMN users.is_tow_pilot IS 'Indicates if user is certified as a tow pilot';
COMMENT ON COLUMN users.is_examiner IS 'Indicates if user is certified as an examiner';
COMMENT ON COLUMN users.deleted_at IS 'Soft delete timestamp. NULL indicates active user.';
