-- Reverse the migration: make receiver_id NOT NULL again
-- WARNING: This will fail if there are NULL values in receiver_id

-- First delete any fixes without a receiver_id (these are ADS-B fixes)
DELETE FROM fixes WHERE receiver_id IS NULL;

-- Make receiver_id NOT NULL (will fail if there are NULL values)
ALTER TABLE fixes
ALTER COLUMN receiver_id SET NOT NULL;
