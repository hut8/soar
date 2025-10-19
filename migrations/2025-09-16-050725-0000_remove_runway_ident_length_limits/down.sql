-- Revert le_ident and he_ident columns in runways table from TEXT back to VARCHAR(7)
-- WARNING: This may truncate data if any runway identifiers are longer than 7 characters
ALTER TABLE runways ALTER COLUMN le_ident TYPE VARCHAR(7);
ALTER TABLE runways ALTER COLUMN he_ident TYPE VARCHAR(7);
