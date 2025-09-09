-- Add up migration script here
-- =========================================================
-- Create users table for user authentication and management
-- =========================================================
CREATE TYPE access_level AS ENUM ('standard', 'admin');

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    first_name VARCHAR(255) NOT NULL,
    last_name VARCHAR(255) NOT NULL,
    email VARCHAR(320) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    access_level access_level NOT NULL DEFAULT 'standard',
    club_id UUID REFERENCES clubs(id) ON DELETE SET NULL,
    email_verified BOOLEAN DEFAULT FALSE,
    password_reset_token VARCHAR(255),
    password_reset_expires_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create indexes for performance
CREATE INDEX users_email_idx ON users (email);
CREATE INDEX users_club_id_idx ON users (club_id);
CREATE INDEX users_access_level_idx ON users (access_level);
CREATE INDEX users_password_reset_token_idx ON users (password_reset_token);

-- Add trigger to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
