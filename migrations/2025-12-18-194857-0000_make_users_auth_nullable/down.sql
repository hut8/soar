-- Rollback: Restore authentication fields to NOT NULL
-- WARNING: This will fail if there are users with NULL email/password

-- Remove the check constraint
ALTER TABLE users DROP CONSTRAINT IF EXISTS users_auth_consistency_check;

-- Drop the partial unique index
DROP INDEX IF EXISTS users_email_unique_idx;

-- Recreate the full unique constraint on email
ALTER TABLE users ADD CONSTRAINT users_email_key UNIQUE (email);

-- Make fields NOT NULL again (will fail if NULL values exist)
ALTER TABLE users ALTER COLUMN password_hash SET NOT NULL;
ALTER TABLE users ALTER COLUMN email SET NOT NULL;

-- Remove comments
COMMENT ON TABLE users IS NULL;
COMMENT ON COLUMN users.email IS NULL;
COMMENT ON COLUMN users.password_hash IS NULL;
