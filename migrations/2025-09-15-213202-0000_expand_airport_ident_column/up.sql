-- Expand the ident column in airports table from VARCHAR(7) to VARCHAR(16)
ALTER TABLE airports ALTER COLUMN ident TYPE VARCHAR(16);