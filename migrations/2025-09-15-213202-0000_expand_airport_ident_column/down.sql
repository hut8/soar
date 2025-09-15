-- Revert the ident column in airports table from VARCHAR(16) back to VARCHAR(7)
-- WARNING: This may truncate data if any ident values are longer than 7 characters
ALTER TABLE airports ALTER COLUMN ident TYPE VARCHAR(7);