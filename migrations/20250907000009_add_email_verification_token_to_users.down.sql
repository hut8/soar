-- Add down migration script here
-- Remove email verification token fields from users table
DROP INDEX IF EXISTS users_email_verification_token_idx;
ALTER TABLE users DROP COLUMN IF EXISTS email_verification_expires_at;
ALTER TABLE users DROP COLUMN IF EXISTS email_verification_token;
