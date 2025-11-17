-- Drop redundant index on flight_duration_buckets.bucket_order
-- The unique constraint flight_duration_buckets_bucket_order_key already provides
-- the same indexing functionality, making this regular index redundant
DROP INDEX IF EXISTS idx_flight_duration_buckets_order;
