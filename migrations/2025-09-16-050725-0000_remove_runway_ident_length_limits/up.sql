-- Remove the 7 character limit on le_ident and he_ident columns in runways table
-- Change from VARCHAR(7) to TEXT to allow longer runway endpoint identifiers
ALTER TABLE runways ALTER COLUMN le_ident TYPE TEXT;
ALTER TABLE runways ALTER COLUMN he_ident TYPE TEXT;