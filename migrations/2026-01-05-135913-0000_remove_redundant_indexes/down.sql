-- Recreate index on user_fixes.user_id
CREATE INDEX IF NOT EXISTS idx_user_fixes_user_id ON user_fixes (user_id);

-- Recreate index on receiver_coverage_h3.h3_index
CREATE INDEX IF NOT EXISTS idx_coverage_h3_index
    ON receiver_coverage_h3 (h3_index, resolution);
