-- Create GIST index on fixes.geom column for fast spatial queries
-- This is split into a separate migration from the column creation
-- to separate the fast schema changes from the slow index build
CREATE INDEX fixes_geom_idx ON fixes USING GIST (geom);
