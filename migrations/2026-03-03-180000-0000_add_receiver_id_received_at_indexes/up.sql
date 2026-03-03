-- Add composite index on fixes for receiver queries (aggregate-stats, fixes endpoints).
-- The receiver_id index was lost when fixes was converted to a TimescaleDB hypertable.
-- Using (receiver_id, received_at DESC) so the index supports both filtering and sorting.
-- Note: CONCURRENTLY is not supported on TimescaleDB hypertables, so we use regular
-- CREATE INDEX which acquires a SHARE lock. TimescaleDB propagates the index to all chunks.
-- safety-assured:start
CREATE INDEX IF NOT EXISTS idx_fixes_receiver_id_received_at
    ON fixes (receiver_id, received_at DESC);
-- safety-assured:end

-- Add composite index on raw_messages for paginated receiver queries.
-- The existing idx_raw_messages_receiver_id supports COUNT but not ORDER BY received_at DESC.
-- safety-assured:start
CREATE INDEX IF NOT EXISTS idx_raw_messages_receiver_id_received_at
    ON raw_messages (receiver_id, received_at DESC);
-- safety-assured:end
