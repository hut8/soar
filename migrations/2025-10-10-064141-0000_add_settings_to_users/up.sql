-- Add settings JSONB column to users table for storing user preferences
ALTER TABLE users
    ADD COLUMN settings JSONB NOT NULL DEFAULT '{}'::jsonb;
