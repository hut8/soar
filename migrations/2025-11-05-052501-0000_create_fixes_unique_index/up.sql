-- Create unique index on (device_id, timestamp)
-- WARNING: This will be SLOW on large tables
-- This prevents duplicate fixes from being inserted
-- Note: This will FAIL if there are any duplicate (device_id, timestamp) rows
-- If this fails, manually delete duplicates first

CREATE UNIQUE INDEX idx_fixes_unique_key
ON fixes (device_id, timestamp);

-- Add helpful comment
COMMENT ON INDEX idx_fixes_unique_key IS
'Prevents duplicate fixes on JetStream redelivery - a device can only be in one place at one time. Natural key: (device_id, timestamp)';
