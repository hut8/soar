-- Enable pg_trgm extension for trigram-based fuzzy search
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- Remove existing index on clubs.name
DROP INDEX IF EXISTS clubs_name_idx;

-- Add trigram indexes for fuzzy search
CREATE INDEX clubs_name_trgm_idx ON clubs USING gin (name gin_trgm_ops);
CREATE INDEX airports_name_trgm_idx ON airports USING gin (name gin_trgm_ops);
CREATE INDEX airports_icao_trgm_idx ON airports USING gin (icao_code gin_trgm_ops) WHERE icao_code IS NOT NULL;
CREATE INDEX airports_iata_trgm_idx ON airports USING gin (iata_code gin_trgm_ops) WHERE iata_code IS NOT NULL;
CREATE INDEX airports_ident_trgm_idx ON airports USING gin (ident gin_trgm_ops);