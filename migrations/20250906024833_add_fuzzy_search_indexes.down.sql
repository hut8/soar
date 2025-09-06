-- Remove trigram indexes
DROP INDEX IF EXISTS clubs_name_trgm_idx;
DROP INDEX IF EXISTS airports_name_trgm_idx;
DROP INDEX IF EXISTS airports_icao_trgm_idx;
DROP INDEX IF EXISTS airports_iata_trgm_idx;
DROP INDEX IF EXISTS airports_ident_trgm_idx;

-- Recreate the original index on clubs.name
CREATE INDEX clubs_name_idx ON clubs (name);