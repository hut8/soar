-- Revert fix_count back to INTEGER
-- Warning: This will fail if any values exceed INTEGER range
ALTER TABLE receiver_coverage_h3
ALTER COLUMN fix_count TYPE INTEGER;
