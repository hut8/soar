-- Add up migration script here
-- Add email verification token field to users table
ALTER TABLE users ADD COLUMN email_verification_token VARCHAR(255);
ALTER TABLE users ADD COLUMN email_verification_expires_at TIMESTAMP WITH TIME ZONE;

-- Create index for email verification token lookup
CREATE INDEX users_email_verification_token_idx ON users (email_verification_token);
