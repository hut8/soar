-- Restore TimescaleDB retention policies (30 days)
SELECT add_retention_policy('raw_messages', INTERVAL '30 days');
SELECT add_retention_policy('fixes', INTERVAL '30 days');

COMMENT ON TABLE raw_messages IS 'TimescaleDB hypertable partitioned by received_at (1 day chunks). Automatic retention: 30 days. Compression enabled for chunks older than 7 days.';
COMMENT ON TABLE fixes IS 'TimescaleDB hypertable partitioned by received_at (1 day chunks). Automatic retention: 30 days. Compression enabled for chunks older than 7 days.';
