-- Add source tracking for airspace data (OpenAIP vs FAA NASR)
CREATE TYPE airspace_source AS ENUM ('openaip', 'faa_nasr');

-- Add source column (nullable first, backfill, then safe NOT NULL via CHECK)
ALTER TABLE airspaces ADD COLUMN source airspace_source;
UPDATE airspaces SET source = 'openaip';

ALTER TABLE airspaces ADD CONSTRAINT airspaces_source_not_null
    CHECK (source IS NOT NULL) NOT VALID;
ALTER TABLE airspaces VALIDATE CONSTRAINT airspaces_source_not_null;
-- safety-assured:start
-- Safe: CHECK constraint validated above guarantees no NULLs exist,
-- so SET NOT NULL is instant (no table scan needed on PG 12+).
ALTER TABLE airspaces ALTER COLUMN source SET NOT NULL;
-- safety-assured:end
ALTER TABLE airspaces DROP CONSTRAINT airspaces_source_not_null;
ALTER TABLE airspaces ALTER COLUMN source SET DEFAULT 'openaip';

-- Add source_id column for source-specific identifiers
ALTER TABLE airspaces ADD COLUMN source_id TEXT;

-- Backfill: copy openaip_id to source_id for existing records
UPDATE airspaces SET source_id = openaip_id;

-- Safe NOT NULL addition via CHECK constraint
ALTER TABLE airspaces ADD CONSTRAINT airspaces_source_id_not_null
    CHECK (source_id IS NOT NULL) NOT VALID;
ALTER TABLE airspaces VALIDATE CONSTRAINT airspaces_source_id_not_null;
-- safety-assured:start
-- Safe: CHECK constraint validated above guarantees no NULLs exist,
-- so SET NOT NULL is instant (no table scan needed on PG 12+).
ALTER TABLE airspaces ALTER COLUMN source_id SET NOT NULL;
-- safety-assured:end
ALTER TABLE airspaces DROP CONSTRAINT airspaces_source_id_not_null;

-- Drop old unique constraint on openaip_id
ALTER TABLE airspaces DROP CONSTRAINT airspaces_openaip_id_key;

-- Make openaip_id nullable (FAA records won't have one)
ALTER TABLE airspaces ALTER COLUMN openaip_id DROP NOT NULL;

-- safety-assured:start
-- Safe: airspaces table is small (~1,500 rows) and the SHARE lock during
-- index creation will be brief. CONCURRENTLY is not used because diesel
-- migration run does not support run_in_transaction = false.
CREATE UNIQUE INDEX idx_airspaces_source_source_id ON airspaces (source, source_id);
CREATE INDEX idx_airspaces_source ON airspaces (source);
-- safety-assured:end
