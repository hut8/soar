-- Revert aircraft_other_names table changes

-- Revert other_name back to VARCHAR(50) (may truncate data if any names exceed 50 chars)
ALTER TABLE aircraft_other_names
ALTER COLUMN other_name TYPE VARCHAR(50);

-- Revert registration_number back to VARCHAR(5) (may truncate data if any exceed 5 chars)
ALTER TABLE aircraft_other_names
ALTER COLUMN registration_number TYPE VARCHAR(5);