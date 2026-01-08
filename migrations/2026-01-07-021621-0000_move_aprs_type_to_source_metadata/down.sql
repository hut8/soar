-- Reverse the migration: restore the 'aprs_type' column

-- Add the 'aprs_type' column back with default empty string
ALTER TABLE fixes ADD COLUMN aprs_type VARCHAR(9) NOT NULL DEFAULT '';
