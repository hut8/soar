-- Remove runways_inferred column from flights table

ALTER TABLE flights
DROP COLUMN IF EXISTS runways_inferred;
