-- Remove redundant index on user_fixes.user_id
-- The composite index idx_user_fixes_user_timestamp (user_id, timestamp DESC) covers user_id queries
DROP INDEX IF EXISTS idx_user_fixes_user_id;

-- Remove redundant index on receiver_coverage_h3.h3_index
-- The primary key (h3_index, resolution, receiver_id, date) covers h3_index queries
DROP INDEX IF EXISTS idx_coverage_h3_index;
