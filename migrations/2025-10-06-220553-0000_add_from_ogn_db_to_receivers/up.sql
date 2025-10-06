-- Add from_ogn_db column to receivers table
ALTER TABLE receivers ADD COLUMN from_ogn_db BOOLEAN NOT NULL DEFAULT false;

-- Set existing receivers to true (they came from OGN database)
UPDATE receivers SET from_ogn_db = true;
