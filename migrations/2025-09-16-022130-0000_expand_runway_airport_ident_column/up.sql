-- Expand the airport_ident column in runways table from VARCHAR(7) to VARCHAR(16)
-- This matches the ident column expansion in the airports table
ALTER TABLE runways ALTER COLUMN airport_ident TYPE VARCHAR(16);