-- Add deduplication to fixes to prevent duplicate position reports on JetStream redelivery after crashes
-- Natural key: (device_id, timestamp) - a device can only be in one place at one time

-- Delete any existing duplicates before creating unique index
-- Keep the oldest record (smallest id) for each unique (device_id, timestamp) combination
DELETE FROM fixes a
USING fixes b
WHERE a.id > b.id
  AND a.device_id = b.device_id
  AND a.timestamp = b.timestamp;

-- Create unique index on natural key (device_id, timestamp)
-- This prevents the same fix from being inserted twice
-- Uses regular CREATE INDEX (not CONCURRENTLY) since this runs in a transaction
CREATE UNIQUE INDEX idx_fixes_unique_key
ON fixes (device_id, timestamp);

-- Add helpful comment
COMMENT ON INDEX idx_fixes_unique_key IS
'Prevents duplicate fixes on JetStream redelivery - a device can only be in one place at one time. Natural key: (device_id, timestamp)';
