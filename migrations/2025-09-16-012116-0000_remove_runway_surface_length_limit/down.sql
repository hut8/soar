-- Revert the surface column in runways table from TEXT back to VARCHAR(10)
-- WARNING: This may truncate data if any surface values are longer than 10 characters
ALTER TABLE runways ALTER COLUMN surface TYPE VARCHAR(10);