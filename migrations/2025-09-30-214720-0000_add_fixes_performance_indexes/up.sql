-- Add indexes for performance queries on fixes table
-- Index on ground_speed_knots for filtering by speed
CREATE INDEX idx_fixes_ground_speed_knots ON fixes (ground_speed_knots);

-- Index on is_active for filtering active/inactive aircraft
CREATE INDEX idx_fixes_is_active ON fixes (is_active);
