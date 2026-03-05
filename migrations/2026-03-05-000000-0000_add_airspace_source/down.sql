-- Remove source tracking columns
ALTER TABLE airspaces DROP COLUMN source_id;
ALTER TABLE airspaces DROP COLUMN source;

-- Restore openaip_id as NOT NULL with unique constraint
ALTER TABLE airspaces ALTER COLUMN openaip_id SET NOT NULL;
ALTER TABLE airspaces ADD CONSTRAINT airspaces_openaip_id_key UNIQUE (openaip_id);

DROP TYPE airspace_source;
