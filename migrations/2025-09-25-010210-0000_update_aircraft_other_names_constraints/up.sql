-- Update aircraft_other_names table to remove length constraints and increase registration_number size

-- Remove length limit on other_name (was VARCHAR(50), now TEXT)
ALTER TABLE aircraft_other_names
ALTER COLUMN other_name TYPE TEXT;

-- Increase registration_number size from VARCHAR(5) to VARCHAR(7)
ALTER TABLE aircraft_other_names
ALTER COLUMN registration_number TYPE VARCHAR(7);
