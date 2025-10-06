-- Add raw_data column to receiver_statuses table with a default
ALTER TABLE receiver_statuses ADD COLUMN raw_data TEXT NOT NULL DEFAULT '';

-- Drop the default so future inserts must provide a value
ALTER TABLE receiver_statuses ALTER COLUMN raw_data DROP DEFAULT;
