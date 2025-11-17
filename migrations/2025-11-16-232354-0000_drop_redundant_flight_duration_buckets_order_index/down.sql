-- Recreate the index that was dropped in up.sql
-- Note: This index is redundant with the unique constraint, but we recreate it
-- in the down migration to maintain exact reversibility
CREATE INDEX idx_flight_duration_buckets_order ON flight_duration_buckets (bucket_order);
