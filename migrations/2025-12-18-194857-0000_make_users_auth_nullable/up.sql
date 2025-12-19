-- Make authentication fields nullable in users table
-- This allows users to exist without login capability (pilots without accounts)

-- Remove NOT NULL constraints from authentication fields
ALTER TABLE users ALTER COLUMN email DROP NOT NULL;
ALTER TABLE users ALTER COLUMN password_hash DROP NOT NULL;

-- Drop the unique constraint on email (we'll replace it with a partial index)
ALTER TABLE users DROP CONSTRAINT users_email_key;

-- Create a partial unique index that only applies to non-null emails
-- This allows multiple NULL emails while ensuring uniqueness for actual email addresses
CREATE UNIQUE INDEX users_email_unique_idx ON users(email) WHERE email IS NOT NULL;

-- Note: We intentionally do NOT add a constraint requiring email and password_hash
-- to both be NULL or both be NOT NULL. The pilot invitation workflow requires an
-- intermediate state where email is set (for sending invitation) but password is NULL
-- (until the pilot completes registration). Application logic enforces consistency.

-- Add comment documenting the architectural change
COMMENT ON TABLE users IS 'Unified table for all people (users and pilots). Users without email/password cannot log in but can be assigned to flights.';
COMMENT ON COLUMN users.email IS 'Authentication credential. NULL indicates user cannot log in (pilot-only record).';
COMMENT ON COLUMN users.password_hash IS 'Argon2 password hash. NULL indicates user cannot log in (pilot-only record).';
