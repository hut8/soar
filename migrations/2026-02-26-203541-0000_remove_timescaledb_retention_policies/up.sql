-- Remove TimescaleDB retention policies from fixes and raw_messages
-- The archive process (soar archive) handles data cleanup instead,
-- archiving records to compressed files before deleting them.
-- Having both the retention policy AND the archive process is problematic:
-- - Retention policy (30 days) deletes data before archive (45 days) can export it
-- - Data gets silently dropped without being archived to disk

SELECT remove_retention_policy('raw_messages', if_exists => true);
SELECT remove_retention_policy('fixes', if_exists => true);

-- Update table comments to reflect the change
COMMENT ON TABLE raw_messages IS 'TimescaleDB hypertable partitioned by received_at (1 day chunks). Compression enabled for chunks older than 7 days. Retention managed by soar archive command.';
COMMENT ON TABLE fixes IS 'TimescaleDB hypertable partitioned by received_at (1 day chunks). Compression enabled for chunks older than 7 days. Retention managed by soar archive command.';
