-- Add up migration script here
-- Make created_at and updated_at columns NOT NULL on flights table

ALTER TABLE flights 
ALTER COLUMN created_at SET NOT NULL,
ALTER COLUMN updated_at SET NOT NULL;