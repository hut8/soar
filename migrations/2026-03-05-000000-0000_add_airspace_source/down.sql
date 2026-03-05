-- Drop indexes
DROP INDEX IF EXISTS idx_airspaces_source;
DROP INDEX IF EXISTS idx_airspaces_source_source_id;

-- Remove non-OpenAIP airspaces so openaip_id can be restored as NOT NULL
DELETE FROM airspaces WHERE openaip_id IS NULL;

-- Restore openaip_id as NOT NULL with unique constraint
ALTER TABLE airspaces ALTER COLUMN openaip_id SET NOT NULL;
ALTER TABLE airspaces ADD CONSTRAINT airspaces_openaip_id_key UNIQUE (openaip_id);

-- Remove source tracking columns
ALTER TABLE airspaces DROP COLUMN source_id;
ALTER TABLE airspaces DROP COLUMN source;

DROP TYPE airspace_source;
