-- Drop idx_airspaces_source (source) which is redundant because
-- idx_airspaces_source_source_id (source, source_id) covers single-column lookups on source.
DROP INDEX CONCURRENTLY IF EXISTS idx_airspaces_source;

-- Drop idx_raw_messages_receiver_id (receiver_id) which is redundant because
-- idx_raw_messages_receiver_id_received_at (receiver_id, received_at DESC) covers
-- single-column lookups on receiver_id.
-- Note: CONCURRENTLY is not supported on TimescaleDB hypertable indexes.
-- safety-assured:start
DROP INDEX IF EXISTS idx_raw_messages_receiver_id;
-- safety-assured:end
