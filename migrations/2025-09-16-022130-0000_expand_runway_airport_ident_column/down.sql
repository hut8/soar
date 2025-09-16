-- Revert the airport_ident column in runways table from VARCHAR(16) back to VARCHAR(7)
-- WARNING: This may truncate data if any airport_ident values are longer than 7 characters
ALTER TABLE runways ALTER COLUMN airport_ident TYPE VARCHAR(7);