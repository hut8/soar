CREATE UNIQUE INDEX CONCURRENTLY idx_airspaces_source_source_id ON airspaces (source, source_id);
CREATE INDEX CONCURRENTLY idx_airspaces_source ON airspaces (source);
