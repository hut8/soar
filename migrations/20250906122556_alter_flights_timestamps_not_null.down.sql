-- Add down migration script here
-- Revert created_at and updated_at columns to allow NULL on flights table

ALTER TABLE flights 
ALTER COLUMN created_at DROP NOT NULL,
ALTER COLUMN updated_at DROP NOT NULL;