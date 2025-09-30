-- Add is_active column to fixes table
-- is_active is true when ground_speed >= 15 knots, false otherwise
-- Default to true for now, will update values manually later
ALTER TABLE fixes ADD COLUMN is_active BOOLEAN NOT NULL DEFAULT true;
